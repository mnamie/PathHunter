//! Pure clean logic shared by the `ph clean` CLI flow and the GUI: classifies
//! what can be removed from the registry PATH values and computes the
//! rewritten segment lists. Nothing here touches the registry or stdout.

use std::collections::HashSet;

use crate::audit::PathEntry;
use crate::expandvars;

fn strip_trailing_seps_win(s: &str) -> &str {
    s.trim_end_matches(['/', '\\'])
}

/// Comparison key for a raw registry segment: env-expanded, trailing
/// separators stripped, lowercased.
pub fn norm(s: &str) -> String {
    strip_trailing_seps_win(&expandvars::windows(s)).to_ascii_lowercase()
}

/// Comparison key for an audited (already expanded) PATH entry.
pub fn dead_key(e: &PathEntry) -> String {
    strip_trailing_seps_win(&e.path).to_ascii_lowercase()
}

#[derive(Debug, Clone)]
pub struct DupInfo {
    /// Normalized key (see [`norm`]).
    pub key: String,
    pub display: String,
    pub extra: usize,
}

/// Returns paths appearing more than once, in first-occurrence order.
pub fn intra_dup_info(segs: &[String]) -> Vec<DupInfo> {
    // (key, first display, count) — PATH lists are short, a linear scan is fine.
    let mut seen: Vec<(String, String, usize)> = Vec::new();
    for s in segs {
        let key = norm(s);
        match seen.iter_mut().find(|(k, _, _)| *k == key) {
            Some((_, _, count)) => *count += 1,
            None => {
                let display = strip_trailing_seps_win(&expandvars::windows(s)).to_owned();
                seen.push((key, display, 1));
            }
        }
    }
    seen.into_iter()
        .filter(|(_, _, count)| *count > 1)
        .map(|(key, display, count)| DupInfo {
            key,
            display,
            extra: count - 1,
        })
        .collect()
}

/// Keeps the first occurrence of each path, dropping subsequent duplicates.
pub fn dedup_segs(segs: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    segs.iter()
        .filter(|s| seen.insert(norm(s)))
        .cloned()
        .collect()
}

/// Like [`dedup_segs`], but only collapses paths whose key is in `keys`.
pub fn dedup_keys(segs: &[String], keys: &HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    segs.iter()
        .filter(|s| {
            let k = norm(s);
            !keys.contains(&k) || seen.insert(k)
        })
        .cloned()
        .collect()
}

pub fn sum_extra(dups: &[DupInfo]) -> usize {
    dups.iter().map(|d| d.extra).sum()
}

pub fn to_set(segs: &[String]) -> HashSet<String> {
    segs.iter().map(|s| norm(s)).collect()
}

/// Segments whose normalized form is not in `remove`.
pub fn without(segs: &[String], remove: &HashSet<String>) -> Vec<String> {
    segs.iter()
        .filter(|s| !remove.contains(&norm(s)))
        .cloned()
        .collect()
}

/// User segments that are also present in the System PATH.
pub fn cross_dups(user_segs: &[String], system_segs: &[String]) -> Vec<String> {
    let system = to_set(system_segs);
    user_segs
        .iter()
        .filter(|s| system.contains(&norm(s)))
        .cloned()
        .collect()
}

pub fn entry_noun(n: usize) -> &'static str {
    if n == 1 { "entry" } else { "entries" }
}

pub fn extra_noun(n: usize) -> &'static str {
    if n == 1 { "copy" } else { "copies" }
}

/// Everything `clean` could act on, classified by registry hive.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub admin: bool,
    /// Every dead audited entry, in PATH order.
    pub dead: Vec<PathEntry>,
    pub user_dead: Vec<PathEntry>,
    pub system_dead: Vec<PathEntry>,
    /// Dead entries in neither registry value (added by a shell/launcher).
    pub shell_dead: Vec<PathEntry>,
    /// Raw User segments already present in the System PATH.
    pub user_cross_dups: Vec<String>,
    pub user_intra_dups: Vec<DupInfo>,
    pub system_intra_dups: Vec<DupInfo>,
}

impl Plan {
    pub fn build(
        entries: &[PathEntry],
        user_segs: &[String],
        system_segs: &[String],
        admin: bool,
    ) -> Plan {
        let expanded_user = to_set(user_segs);
        let expanded_system = to_set(system_segs);

        let mut plan = Plan {
            admin,
            user_cross_dups: cross_dups(user_segs, system_segs),
            user_intra_dups: intra_dup_info(user_segs),
            system_intra_dups: intra_dup_info(system_segs),
            ..Plan::default()
        };
        for e in entries.iter().filter(|e| e.state.is_dead()) {
            plan.dead.push(e.clone());
            let key = dead_key(e);
            if expanded_user.contains(&key) {
                plan.user_dead.push(e.clone());
            } else if expanded_system.contains(&key) {
                plan.system_dead.push(e.clone());
            } else {
                plan.shell_dead.push(e.clone());
            }
        }
        plan
    }

    /// Something can actually be removed with the current privileges.
    pub fn any_actionable(&self) -> bool {
        !self.user_dead.is_empty()
            || !self.user_cross_dups.is_empty()
            || !self.user_intra_dups.is_empty()
            || (self.admin && (!self.system_dead.is_empty() || !self.system_intra_dups.is_empty()))
    }

    /// Something is worth reporting at all.
    pub fn any_issue(&self) -> bool {
        !self.dead.is_empty()
            || !self.user_cross_dups.is_empty()
            || !self.user_intra_dups.is_empty()
            || !self.system_intra_dups.is_empty()
    }
}

/// Which plan items to remove, as normalized keys per category.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// [`dead_key`]s of dead User entries.
    pub user_dead: HashSet<String>,
    /// [`dead_key`]s of dead System entries.
    pub system_dead: HashSet<String>,
    /// [`norm`] keys of User entries already in System.
    pub user_cross: HashSet<String>,
    /// [`DupInfo::key`]s to collapse within User.
    pub user_dups: HashSet<String>,
    /// [`DupInfo::key`]s to collapse within System.
    pub system_dups: HashSet<String>,
}

/// Applies `sel` to the raw registry segments, in the same order as the CLI
/// flow (dead → cross-registry → intra duplicates). System steps are skipped
/// unless `admin`. Returns the new (user, system) segment lists.
pub fn apply(
    user_segs: &[String],
    system_segs: &[String],
    admin: bool,
    sel: &Selection,
) -> (Vec<String>, Vec<String>) {
    let mut user = without(user_segs, &sel.user_dead);
    let mut system = system_segs.to_vec();
    if admin {
        system = without(&system, &sel.system_dead);
    }

    // Recomputed against the possibly-shrunk System list.
    let remove: HashSet<String> = to_set(&cross_dups(&user, &system))
        .into_iter()
        .filter(|k| sel.user_cross.contains(k))
        .collect();
    user = without(&user, &remove);

    user = dedup_keys(&user, &sel.user_dups);
    if admin {
        system = dedup_keys(&system, &sel.system_dups);
    }
    (user, system)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::EntryState;

    fn segs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn keys(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn entry(path: &str, state: EntryState) -> PathEntry {
        PathEntry {
            path: path.to_owned(),
            state,
            source: String::new(),
        }
    }

    #[test]
    fn intra_dup_info_no_duplicates() {
        assert!(intra_dup_info(&segs(&["C:\\a", "C:\\b", "C:\\c"])).is_empty());
    }

    #[test]
    fn intra_dup_info_finds_duplicates() {
        let got = intra_dup_info(&segs(&["C:\\a", "C:\\b", "C:\\a", "C:\\a"]));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].key, "c:\\a");
        assert_eq!(got[0].display, "C:\\a");
        assert_eq!(got[0].extra, 2);
    }

    #[test]
    fn intra_dup_info_preserves_first_occurrence_casing() {
        let got = intra_dup_info(&segs(&["C:\\Foo", "c:\\foo", "C:\\Bar"]));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].display, "C:\\Foo");
        assert_eq!(got[0].extra, 1);
    }

    #[test]
    fn dedup_segs_keeps_first_occurrence() {
        let got = dedup_segs(&segs(&["C:\\Foo", "C:\\Bar", "c:\\foo", "C:\\Bar\\"]));
        assert_eq!(got, segs(&["C:\\Foo", "C:\\Bar"]));
    }

    #[test]
    fn dedup_keys_only_collapses_selected() {
        let got = dedup_keys(
            &segs(&["C:\\Foo", "C:\\Bar", "c:\\foo", "C:\\Bar\\"]),
            &keys(&["c:\\foo"]),
        );
        assert_eq!(got, segs(&["C:\\Foo", "C:\\Bar", "C:\\Bar\\"]));
    }

    #[test]
    fn sum_extra_adds_up() {
        let dup = |extra| DupInfo {
            key: String::new(),
            display: String::new(),
            extra,
        };
        assert_eq!(sum_extra(&[dup(2), dup(1), dup(3)]), 6);
    }

    #[test]
    fn without_removes_normalized_matches() {
        let remove = to_set(&segs(&["c:\\dead\\"]));
        assert_eq!(
            without(&segs(&["C:\\Dead", "C:\\Live"]), &remove),
            segs(&["C:\\Live"])
        );
    }

    #[test]
    fn plan_build_classifies_dead_by_hive() {
        let entries = [
            entry("C:\\UserDead", EntryState::Dead),
            entry("C:\\SysDead\\", EntryState::Dangling),
            entry("C:\\ShellDead", EntryState::File),
            entry("C:\\Live", EntryState::Ok),
        ];
        let user = segs(&["C:\\userdead", "C:\\Shared"]);
        let system = segs(&["C:\\SysDead", "C:\\shared\\", "C:\\X", "c:\\x"]);
        let plan = Plan::build(&entries, &user, &system, false);

        assert_eq!(plan.dead.len(), 3);
        assert_eq!(plan.user_dead[0].path, "C:\\UserDead");
        assert_eq!(plan.system_dead[0].path, "C:\\SysDead\\");
        assert_eq!(plan.shell_dead[0].path, "C:\\ShellDead");
        assert_eq!(plan.user_cross_dups, segs(&["C:\\Shared"]));
        assert!(plan.user_intra_dups.is_empty());
        assert_eq!(plan.system_intra_dups[0].key, "c:\\x");
        assert!(plan.any_issue());
        assert!(plan.any_actionable());
    }

    #[test]
    fn plan_system_only_is_not_actionable_without_admin() {
        let entries = [entry("C:\\SysDead", EntryState::Dead)];
        let system = segs(&["C:\\SysDead"]);
        assert!(!Plan::build(&entries, &[], &system, false).any_actionable());
        assert!(Plan::build(&entries, &[], &system, true).any_actionable());
    }

    #[test]
    fn apply_skips_system_without_admin() {
        let user = segs(&["C:\\Dead", "C:\\A", "C:\\A"]);
        let system = segs(&["C:\\SysDead", "C:\\B", "C:\\B"]);
        let sel = Selection {
            user_dead: keys(&["c:\\dead"]),
            system_dead: keys(&["c:\\sysdead"]),
            user_dups: keys(&["c:\\a"]),
            system_dups: keys(&["c:\\b"]),
            ..Selection::default()
        };
        let (u, s) = apply(&user, &system, false, &sel);
        assert_eq!(u, segs(&["C:\\A"]));
        assert_eq!(s, system);

        let (_, s) = apply(&user, &system, true, &sel);
        assert_eq!(s, segs(&["C:\\B"]));
    }

    #[test]
    fn apply_cross_dup_recomputed_after_system_dead_removal() {
        // C:\Gone is in both hives and dead. Removing it from System first
        // means it's no longer a cross-registry duplicate in User.
        let user = segs(&["C:\\Gone", "C:\\Shared"]);
        let system = segs(&["C:\\Gone", "C:\\Shared"]);
        let sel = Selection {
            system_dead: keys(&["c:\\gone"]),
            user_cross: keys(&["c:\\gone", "c:\\shared"]),
            ..Selection::default()
        };
        let (u, s) = apply(&user, &system, true, &sel);
        assert_eq!(u, segs(&["C:\\Gone"]));
        assert_eq!(s, segs(&["C:\\Shared"]));
    }

    #[test]
    fn apply_empty_selection_is_noop() {
        let user = segs(&["C:\\A", "C:\\A"]);
        let system = segs(&["C:\\A"]);
        let (u, s) = apply(&user, &system, true, &Selection::default());
        assert_eq!(u, user);
        assert_eq!(s, system);
    }
}
