use std::io::{self, Write};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Audit,
    Clean,
    Help,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub command: Command,
    pub no_color: bool,
    pub only_dead: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            command: Command::Audit,
            no_color: false,
            only_dead: false,
        }
    }
}

pub fn parse<S: AsRef<str>>(argv: &[S]) -> Config {
    let mut cfg = Config::default();
    for arg in argv {
        match arg.as_ref() {
            "clean" => cfg.command = Command::Clean,
            "--help" | "-h" => {
                cfg.command = Command::Help;
                return cfg;
            }
            "--no-color" | "-n" => cfg.no_color = true,
            "--only-dead" | "-d" => cfg.only_dead = true,
            _ => {}
        }
    }
    cfg
}

pub fn print_help(w: &mut impl Write) -> io::Result<()> {
    write!(
        w,
        "Path Hunter  {VERSION}

Usage: ph [clean] [--only-dead] [--no-color] [--help]

  clean             Remove dead entries from PATH (Windows only)
  --only-dead, -d   Show only dead/missing entries
  --no-color,  -n   Plain text output (no ANSI colors)
  --help,      -h   Show this help

"
    )
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default() {
        let cfg = parse::<&str>(&[]);
        assert_eq!(cfg.command, Command::Audit);
        assert!(!cfg.no_color);
        assert!(!cfg.only_dead);
    }

    #[test]
    fn parse_clean() {
        assert_eq!(parse(&["clean"]).command, Command::Clean);
    }

    #[test]
    fn parse_help_short_circuits() {
        let cfg = parse(&["--only-dead", "--help", "clean"]);
        assert_eq!(cfg.command, Command::Help);
        assert!(cfg.only_dead);
    }

    #[test]
    fn parse_flags() {
        let cfg = parse(&["--no-color", "-d"]);
        assert!(cfg.no_color);
        assert!(cfg.only_dead);
    }

    #[test]
    fn parse_unknown_args_ignored() {
        assert_eq!(parse(&["--bogus"]).command, Command::Audit);
    }

    #[test]
    fn print_help_output() {
        let mut out = Vec::new();
        print_help(&mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("Path Hunter"));
        assert!(out.contains("clean"));
    }
}
