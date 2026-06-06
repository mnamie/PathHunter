use std::io::{self, Write};

use crate::args::{Config, VERSION};
use crate::audit::{EntryState, PathEntry};

// ANSI escape sequences
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";

// 68 × U+2500 (─)
const DIVIDER_LINE: &str = "────────────────────────────────────────────────────────────────────";
const SEP: &str = "  ·  "; // U+00B7 MIDDLE DOT

const MAX_PATH_COL: usize = 50;
const MAX_SRC_COL: usize = 22;

fn width(s: &str) -> usize {
    s.chars().count()
}

/// Shortens `s` to at most `max` characters, ending in "…" when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if width(s) <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn pad_right(s: &str, w: usize) -> String {
    format!("{s:<w$}")
}

fn compute_widths(entries: &[PathEntry]) -> (usize, usize) {
    let col = entries.iter().map(|e| width(&e.path)).max().unwrap_or(0);
    let src = entries.iter().map(|e| width(&e.source)).max().unwrap_or(0);
    (col.min(MAX_PATH_COL), src.min(MAX_SRC_COL))
}

pub fn fmt_detail(e: &PathEntry) -> String {
    match &e.state {
        EntryState::Duplicate(idx) => format!("  [duplicate #{idx}]"),
        EntryState::Symlink(target) => format!("  [→ {target}]"),
        EntryState::Dangling => "  [dangling symlink]".to_owned(),
        EntryState::File => "  [is a file, not a dir]".to_owned(),
        EntryState::Empty => "  [means current directory — security risk]".to_owned(),
        _ => String::new(),
    }
}

fn render_header(w: &mut impl Write, count: usize, plain: bool, only_dead: bool) -> io::Result<()> {
    let suffix = if only_dead {
        "  (showing dead only)"
    } else {
        ""
    };
    if plain {
        writeln!(w, "  Path Hunter  {VERSION}")?;
        writeln!(w, "  Scanning $PATH — {count} entries{suffix}")?;
        writeln!(w, "  {DIVIDER_LINE}")?;
    } else {
        writeln!(w, "{CYAN}{BOLD}  Path Hunter{RESET}  {VERSION}")?;
        writeln!(w, "  Scanning {BOLD}$PATH{RESET} — {count} entries{suffix}")?;
        writeln!(w, "{CYAN}  {DIVIDER_LINE}{RESET}")?;
    }
    Ok(())
}

fn plain_icon(state: &EntryState) -> &'static str {
    match state {
        EntryState::Ok => "✓",
        EntryState::Symlink(_) => "~",
        EntryState::Duplicate(_) => "⚠",
        EntryState::Empty => "!",
        _ => "✗", // all dead states
    }
}

fn render_entry(
    w: &mut impl Write,
    e: &PathEntry,
    col_width: usize,
    src_width: usize,
    plain: bool,
    only_dead: bool,
) -> io::Result<()> {
    if only_dead
        && matches!(
            e.state,
            EntryState::Ok | EntryState::Symlink(_) | EntryState::Duplicate(_)
        )
    {
        return Ok(());
    }

    let path_col = pad_right(&truncate(&e.path, col_width), col_width);
    let src_col = pad_right(&e.source, src_width);
    let det = fmt_detail(e);

    if plain {
        return writeln!(w, "  {}  {path_col}  {src_col}{det}", plain_icon(&e.state));
    }

    match e.state {
        EntryState::Ok => writeln!(
            w,
            "{GREEN}{BOLD}  ✓  {RESET}{path_col}{CYAN}{DIM}  {src_col}{RESET}{DIM}{det}{RESET}"
        ),
        EntryState::Symlink(_) => writeln!(
            w,
            "{CYAN}{BOLD}  ~  {RESET}{path_col}{CYAN}{DIM}  {src_col}{RESET}{CYAN}{det}{RESET}"
        ),
        EntryState::Duplicate(_) => {
            writeln!(
                w,
                "{YELLOW}{BOLD}  ⚠  {RESET}{YELLOW}{path_col}{RESET}{CYAN}{DIM}  {src_col}{RESET}{YELLOW}{det}{RESET}"
            )
        }
        EntryState::Dead | EntryState::File | EntryState::Dangling => {
            writeln!(
                w,
                "{RED}{BOLD}  ✗  {RESET}{RED}{BOLD}{path_col}{RESET}{CYAN}{DIM}  {src_col}{RESET}{RED}{DIM}{det}{RESET}"
            )
        }
        EntryState::Empty => {
            writeln!(
                w,
                "{MAGENTA}{BOLD}  !  {RESET}{MAGENTA}{path_col}{RESET}{CYAN}{DIM}  {src_col}{RESET}{MAGENTA}{DIM}{det}{RESET}"
            )
        }
    }
}

fn render_summary(w: &mut impl Write, entries: &[PathEntry], plain: bool) -> io::Result<()> {
    let (mut n_ok, mut n_sym, mut n_dead, mut n_dup, mut n_empty) = (0, 0, 0, 0, 0);
    for e in entries {
        match e.state {
            EntryState::Ok => n_ok += 1,
            EntryState::Symlink(_) => n_sym += 1,
            EntryState::Duplicate(_) => n_dup += 1,
            EntryState::Empty => n_empty += 1,
            _ => n_dead += 1,
        }
    }

    if plain {
        writeln!(w, "  {DIVIDER_LINE}")?;
        write!(w, "  Summary   ")?;
        if n_ok > 0 {
            write!(w, "{n_ok} ok{SEP}")?;
        }
        write!(w, "{n_dead} dead")?; // always shown
        if n_sym > 0 {
            write!(w, "{SEP}{n_sym} symlinks")?;
        }
        if n_dup > 0 {
            write!(w, "{SEP}{n_dup} duplicates")?;
        }
        if n_empty > 0 {
            write!(w, "{SEP}{n_empty} empty")?;
        }
        return writeln!(w);
    }

    writeln!(w, "{CYAN}  {DIVIDER_LINE}{RESET}")?;

    if n_dead > 0 {
        let colored_sep = format!("{DIM}{SEP}{RESET}");
        write!(w, "  Summary   ")?;
        if n_ok > 0 {
            write!(w, "{n_ok} ok{colored_sep}")?;
        }
        write!(w, "{RED}{BOLD}{n_dead} dead{RESET}")?;
        if n_sym > 0 {
            write!(w, "{colored_sep}{n_sym} symlinks")?;
        }
        if n_dup > 0 {
            write!(w, "{colored_sep}{n_dup} duplicates")?;
        }
        if n_empty > 0 {
            write!(w, "{colored_sep}{n_empty} empty")?;
        }
        writeln!(w)?;
    } else {
        let mut parts = Vec::new();
        if n_ok > 0 {
            parts.push(format!("{n_ok} ok"));
        }
        if n_sym > 0 {
            parts.push(format!("{n_sym} symlinks"));
        }
        if n_dup > 0 {
            parts.push(format!("{n_dup} duplicates"));
        }
        if n_empty > 0 {
            parts.push(format!("{n_empty} empty"));
        }
        writeln!(w, "  Summary   {GREEN}{BOLD}{}{RESET}", parts.join(SEP))?;
    }

    writeln!(w) // trailing blank line in colored mode
}

pub fn render(w: &mut impl Write, entries: &[PathEntry], cfg: &Config) -> io::Result<()> {
    let (col_width, src_width) = compute_widths(entries);
    render_header(w, entries.len(), cfg.no_color, cfg.only_dead)?;
    for e in entries {
        render_entry(w, e, col_width, src_width, cfg.no_color, cfg.only_dead)?;
    }
    render_summary(w, entries, cfg.no_color)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, state: EntryState) -> PathEntry {
        PathEntry {
            path: path.to_owned(),
            state,
            source: String::new(),
        }
    }

    fn render_to_string(entries: &[PathEntry], no_color: bool, only_dead: bool) -> String {
        let cfg = Config {
            no_color,
            only_dead,
            ..Config::default()
        };
        let mut out = Vec::new();
        render(&mut out, entries, &cfg).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn divider_is_68_wide() {
        assert_eq!(width(DIVIDER_LINE), 68);
        assert!(DIVIDER_LINE.chars().all(|c| c == '─'));
    }

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
    }

    #[test]
    fn truncate_non_ascii_on_char_boundary() {
        assert_eq!(truncate("ñññññ", 3), "ññ…");
    }

    #[test]
    fn pad_right_pads_and_never_truncates() {
        assert_eq!(pad_right("hi", 5), "hi   ");
        assert_eq!(pad_right("hello", 3), "hello");
    }

    #[test]
    fn fmt_detail_ok() {
        assert_eq!(fmt_detail(&entry("/usr/bin", EntryState::Ok)), "");
    }

    #[test]
    fn fmt_detail_duplicate() {
        assert_eq!(
            fmt_detail(&entry("/usr/bin", EntryState::Duplicate(1))),
            "  [duplicate #1]"
        );
    }

    #[test]
    fn fmt_detail_symlink() {
        assert!(
            fmt_detail(&entry("/usr/bin", EntryState::Symlink("/real/bin".into())))
                .contains("/real/bin")
        );
    }

    #[test]
    fn fmt_detail_dangling() {
        assert!(fmt_detail(&entry("/x", EntryState::Dangling)).contains("dangling"));
    }

    #[test]
    fn fmt_detail_empty() {
        assert!(
            fmt_detail(&entry("(empty entry)", EntryState::Empty)).contains("current directory")
        );
    }

    #[test]
    fn render_plain_header() {
        let out = render_to_string(&[entry("/usr/bin", EntryState::Ok)], true, false);
        assert!(out.contains("Path Hunter"));
        assert!(out.contains("1 entries"));
    }

    #[test]
    fn render_plain_summary_always_shows_dead() {
        let out = render_to_string(&[entry("/usr/bin", EntryState::Ok)], true, false);
        assert!(out.contains("0 dead"));
    }

    #[test]
    fn render_plain_summary_shows_ok_when_nonzero() {
        let out = render_to_string(&[entry("/usr/bin", EntryState::Ok)], true, false);
        assert!(out.contains("1 ok"));
    }

    #[test]
    fn render_only_dead_filters() {
        let out = render_to_string(
            &[
                entry("/usr/bin", EntryState::Ok),
                entry("/dead/bin", EntryState::Dead),
            ],
            true,
            true,
        );
        assert!(out.contains("/dead/bin"));
        assert!(!out.contains("/usr/bin"));
    }

    #[test]
    fn render_colored() {
        let entries = [
            entry("/usr/bin", EntryState::Ok),
            entry("/bad/path", EntryState::Dead),
            entry("/usr/bin", EntryState::Duplicate(1)),
        ];
        let out = render_to_string(&entries, false, false);
        assert!(out.contains("Path Hunter"));
        assert!(out.contains("\x1b[")); // ANSI codes present
    }

    #[test]
    fn render_colored_all_healthy_summary() {
        let entries = [
            entry("/usr/bin", EntryState::Ok),
            entry("/usr/bin", EntryState::Duplicate(1)),
        ];
        let out = render_to_string(&entries, false, false);
        assert!(out.contains(&format!(
            "  Summary   {GREEN}{BOLD}1 ok{SEP}1 duplicates{RESET}\n\n"
        )));
    }
}
