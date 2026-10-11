use std::collections::HashSet;
use std::io::{self, BufWriter, StdoutLock, Write};

use pathhunter::audit;
use pathhunter::cleanplan::{
    Plan, cross_dups, dead_key, dedup_segs, entry_noun, extra_noun, intra_dup_info, sum_extra,
    to_set, without,
};
use pathhunter::source::SourceMap;
use pathhunter::winenv;

type Out<'a> = BufWriter<StdoutLock<'a>>;

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

    let plan = Plan::build(&entries, &user_segs, &system_segs, admin);
    let any_actionable = plan.any_actionable();
    let any_issue = plan.any_issue();
    let Plan {
        dead,
        user_dead,
        system_dead,
        shell_dead,
        mut user_cross_dups,
        mut user_intra_dups,
        mut system_intra_dups,
        ..
    } = plan;

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
            let dead_keys: HashSet<String> = user_dead.iter().map(dead_key).collect();
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
            let dead_keys: HashSet<String> = system_dead.iter().map(dead_key).collect();
            system_segs = without(&system_segs, &dead_keys);
            winenv::write_system_path_to_registry(&system_segs);
            changed = true;
            writeln!(w, "Removed {n} dead {}.", entry_noun(n))?;
        }
    }

    // ── Cross-registry duplicates: User→System ─────────────────────────────
    if !user_cross_dups.is_empty() {
        // Recompute after possible dead removal above
        user_cross_dups = cross_dups(&user_segs, &system_segs);
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
