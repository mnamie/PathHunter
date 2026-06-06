use std::collections::HashSet;
use std::io::{self, BufWriter, StdoutLock, Write};

use crate::audit::{self, PathEntry};
use crate::expandvars;
use crate::source::SourceMap;
use crate::winenv;

type Out<'a> = BufWriter<StdoutLock<'a>>;

fn strip_trailing_seps_win(s: &str) -> &str {
    s.trim_end_matches(['/', '\\'])
}

fn norm(s: &str) -> String {
    strip_trailing_seps_win(&expandvars::windows(s)).to_ascii_lowercase()
}

fn dead_key(e: &PathEntry) -> String {
    strip_trailing_seps_win(&e.path).to_ascii_lowercase()
}

#[derive(Debug)]
struct DupInfo {
    display: String,
    extra: usize,
}

/// Returns (display_path, extra_count) for paths appearing more than once,
/// in first-occurrence order.
fn intra_dup_info(segs: &[String]) -> Vec<DupInfo> {
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
        .map(|(_, display, count)| DupInfo {
            display,
            extra: count - 1,
        })
        .collect()
}

/// Keeps the first occurrence of each path, dropping subsequent duplicates.
fn dedup_segs(segs: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    segs.iter()
        .filter(|s| seen.insert(norm(s)))
        .cloned()
        .collect()
}

fn sum_extra(dups: &[DupInfo]) -> usize {
    dups.iter().map(|d| d.extra).sum()
}

fn to_set(segs: &[String]) -> HashSet<String> {
    segs.iter().map(|s| norm(s)).collect()
}

/// Segments whose normalized form is not in `remove`.
fn without(segs: &[String], remove: &HashSet<String>) -> Vec<String> {
    segs.iter()
        .filter(|s| !remove.contains(&norm(s)))
        .cloned()
        .collect()
}

/// Returns true if the user answers y. Exits the process on Ctrl-C/EOF,
/// mirroring the prior implementations' SystemExit(0). Writes (and flushes)
/// `msg` through the same writer used for the rest of the report, so
/// buffered report text always appears before the prompt that follows it.
fn prompt(w: &mut Out, msg: &str) -> bool {
    let _ = w.write_all(msg.as_bytes());
    let _ = w.flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(n) if n > 0 => {
            let trimmed = line.trim_matches([' ', '\t', '\r', '\n']);
            trimmed == "y" || trimmed == "Y"
        }
        _ => {
            let _ = w.write_all(b"\n");
            let _ = w.flush();
            std::process::exit(0);
        }
    }
}

fn entry_noun(n: usize) -> &'static str {
    if n == 1 { "entry" } else { "entries" }
}

fn extra_noun(n: usize) -> &'static str {
    if n == 1 { "copy" } else { "copies" }
}

pub fn run() -> io::Result<u8> {
    let mut w = BufWriter::new(io::stdout().lock());
    let result = run_with(&mut w);
    w.flush()?;
    result
}

fn run_with(w: &mut Out) -> io::Result<u8> {
    if !cfg!(windows) {
        writeln!(w, "ph: clean is only supported on Windows")?;
        return Ok(1);
    }

    let raw = std::env::var_os("PATH")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if raw.is_empty() {
        writeln!(w, "ph: PATH is not set or empty")?;
        return Ok(1);
    }

    let mut sm = SourceMap::default();
    sm.build();
    let entries = audit::scan(&sm, &raw);

    let admin = winenv::is_admin();

    let mut user_segs = winenv::read_raw_user_path_segments();
    let mut system_segs = winenv::read_raw_system_path_segments();
    let expanded_user = to_set(&user_segs);
    let mut expanded_system = to_set(&system_segs);

    // ── Classify dead entries ───────────────────────────────────────────
    let dead: Vec<&PathEntry> = entries.iter().filter(|e| e.state.is_dead()).collect();
    let mut user_dead = Vec::new();
    let mut system_dead = Vec::new();
    let mut shell_dead = Vec::new();
    for e in &dead {
        let key = dead_key(e);
        if expanded_user.contains(&key) {
            user_dead.push(*e);
        } else if expanded_system.contains(&key) {
            system_dead.push(*e);
        } else {
            shell_dead.push(*e);
        }
    }

    // ── Cross-registry duplicates: User entries already in System PATH ──
    let mut user_cross_dups: Vec<String> = user_segs
        .iter()
        .filter(|s| expanded_system.contains(&norm(s)))
        .cloned()
        .collect();

    // ── Intra-registry duplicates ────────────────────────────────────────
    let mut user_intra_dups = intra_dup_info(&user_segs);
    let mut system_intra_dups = intra_dup_info(&system_segs);

    let any_actionable = !user_dead.is_empty()
        || !user_cross_dups.is_empty()
        || !user_intra_dups.is_empty()
        || (admin && (!system_dead.is_empty() || !system_intra_dups.is_empty()));
    let any_issue = !dead.is_empty()
        || !user_cross_dups.is_empty()
        || !user_intra_dups.is_empty()
        || !system_intra_dups.is_empty();

    if !any_issue {
        writeln!(w, "Nothing to clean.")?;
        return Ok(0);
    }

    // ── Report ───────────────────────────────────────────────────────────
    let admin_suffix = if admin {
        ""
    } else {
        "  ⚠ requires administrator — will be skipped"
    };

    if !dead.is_empty() {
        write!(w, "Dead entries:\n\n")?;
        if !user_dead.is_empty() {
            writeln!(w, "  [User]")?;
            for e in &user_dead {
                writeln!(w, "    ✗  {}", e.path)?;
            }
            writeln!(w)?;
        }
        if !system_dead.is_empty() {
            writeln!(w, "  [System]{admin_suffix}")?;
            for e in &system_dead {
                writeln!(w, "    ✗  {}", e.path)?;
            }
            writeln!(w)?;
        }
        if !shell_dead.is_empty() {
            writeln!(w, "  [Shell-injected — not in registry, skipped]")?;
            for e in &shell_dead {
                writeln!(w, "    ✗  {}", e.path)?;
            }
            writeln!(w)?;
        }
    }

    if !user_cross_dups.is_empty() {
        write!(
            w,
            "Redundant User PATH entries (already in System PATH):\n\n"
        )?;
        for s in &user_cross_dups {
            writeln!(w, "    ⚠  {s}")?;
        }
        writeln!(w)?;
    }

    if !user_intra_dups.is_empty() {
        let total = sum_extra(&user_intra_dups);
        write!(
            w,
            "Duplicate entries within User PATH ({total} extra {}):\n\n",
            extra_noun(total)
        )?;
        for d in &user_intra_dups {
            writeln!(w, "    ⚠  {}  ({} copies)", d.display, d.extra + 1)?;
        }
        writeln!(w)?;
    }

    if !system_intra_dups.is_empty() {
        let total = sum_extra(&system_intra_dups);
        write!(
            w,
            "Duplicate entries within System PATH{admin_suffix} ({total} extra {}):\n\n",
            extra_noun(total)
        )?;
        for d in &system_intra_dups {
            writeln!(w, "    ⚠  {}  ({} copies)", d.display, d.extra + 1)?;
        }
        writeln!(w)?;
    }

    if !any_actionable {
        if !system_dead.is_empty() || !system_intra_dups.is_empty() {
            writeln!(w, "Re-run as administrator to remove System PATH entries.")?;
        }
        return Ok(0);
    }

    let mut changed = false;

    // ── Dead: User ────────────────────────────────────────────────────────
    if !user_dead.is_empty() {
        let n = user_dead.len();
        if prompt(
            w,
            &format!("Remove {n} dead User PATH {}? [y/N] ", entry_noun(n)),
        ) {
            let dead_keys: HashSet<String> = user_dead.iter().map(|e| dead_key(e)).collect();
            user_segs = without(&user_segs, &dead_keys);
            winenv::write_user_path_to_registry(&user_segs);
            changed = true;
            writeln!(w, "Removed {n} dead {}.", entry_noun(n))?;
        }
    }

    // ── Dead: System ──────────────────────────────────────────────────────
    if admin && !system_dead.is_empty() {
        let n = system_dead.len();
        if prompt(
            w,
            &format!("Remove {n} dead System PATH {}? [y/N] ", entry_noun(n)),
        ) {
            let dead_keys: HashSet<String> = system_dead.iter().map(|e| dead_key(e)).collect();
            system_segs = without(&system_segs, &dead_keys);
            winenv::write_system_path_to_registry(&system_segs);
            changed = true;
            writeln!(w, "Removed {n} dead {}.", entry_noun(n))?;
        }
    }

    // ── Cross-registry duplicates: User→System ─────────────────────────────
    if !user_cross_dups.is_empty() {
        // Recompute after possible dead removal above
        expanded_system = to_set(&system_segs);
        user_cross_dups = user_segs
            .iter()
            .filter(|s| expanded_system.contains(&norm(s)))
            .cloned()
            .collect();
        if !user_cross_dups.is_empty() {
            let n = user_cross_dups.len();
            if prompt(
                w,
                &format!(
                    "Remove {n} User PATH {} already in System PATH? [y/N] ",
                    entry_noun(n)
                ),
            ) {
                user_segs = without(&user_segs, &to_set(&user_cross_dups));
                winenv::write_user_path_to_registry(&user_segs);
                changed = true;
                writeln!(w, "Removed {n} redundant {}.", entry_noun(n))?;
            }
        }
    }

    // ── Intra-duplicates: User ──────────────────────────────────────────────
    user_intra_dups = intra_dup_info(&user_segs);
    if !user_intra_dups.is_empty() {
        let total = sum_extra(&user_intra_dups);
        let noun = extra_noun(total);
        if prompt(
            w,
            &format!("Remove {total} extra {noun} within User PATH? [y/N] "),
        ) {
            user_segs = dedup_segs(&user_segs);
            winenv::write_user_path_to_registry(&user_segs);
            changed = true;
            writeln!(w, "Removed {total} extra {noun}.")?;
        }
    }

    // ── Intra-duplicates: System ─────────────────────────────────────────────
    if admin {
        system_intra_dups = intra_dup_info(&system_segs);
        if !system_intra_dups.is_empty() {
            let total = sum_extra(&system_intra_dups);
            let noun = extra_noun(total);
            if prompt(
                w,
                &format!("Remove {total} extra {noun} within System PATH? [y/N] "),
            ) {
                system_segs = dedup_segs(&system_segs);
                winenv::write_system_path_to_registry(&system_segs);
                changed = true;
                writeln!(w, "Removed {total} extra {noun}.")?;
            }
        }
    }

    if changed {
        winenv::broadcast_env_change();
        writeln!(
            w,
            "PATH updated. Restart your shell for changes to take effect."
        )?;
    } else if !system_dead.is_empty() && !admin {
        writeln!(w, "Re-run as administrator to remove System PATH entries.")?;
    }

    Ok(0)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn intra_dup_info_no_duplicates() {
        assert!(intra_dup_info(&segs(&["C:\\a", "C:\\b", "C:\\c"])).is_empty());
    }

    #[test]
    fn intra_dup_info_finds_duplicates() {
        let got = intra_dup_info(&segs(&["C:\\a", "C:\\b", "C:\\a", "C:\\a"]));
        assert_eq!(got.len(), 1);
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
    fn sum_extra_adds_up() {
        let dups = [
            DupInfo {
                display: String::new(),
                extra: 2,
            },
            DupInfo {
                display: String::new(),
                extra: 1,
            },
            DupInfo {
                display: String::new(),
                extra: 3,
            },
        ];
        assert_eq!(sum_extra(&dups), 6);
    }

    #[test]
    fn without_removes_normalized_matches() {
        let remove = to_set(&segs(&["c:\\dead\\"]));
        assert_eq!(
            without(&segs(&["C:\\Dead", "C:\\Live"]), &remove),
            segs(&["C:\\Live"])
        );
    }
}
