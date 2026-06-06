use std::collections::{HashMap, HashSet};
use std::fs;

use crate::expandvars;
use crate::winenv;

/// Maps a normalized PATH segment to the config-file/registry label it came
/// from, first-seen wins.
#[derive(Default)]
pub struct SourceMap {
    entries: HashMap<String, String>,
    user_keys: HashSet<String>,
}

impl SourceMap {
    pub fn insert(&mut self, key: &str, label: &str) {
        // First-seen wins for the primary label.
        if !self.entries.contains_key(key) {
            self.entries.insert(key.to_owned(), label.to_owned());
        }
        if label == "User" {
            self.user_keys.insert(key.to_owned());
        }
    }

    pub fn lookup(&self, key: &str) -> &str {
        self.entries.get(key).map_or("", String::as_str)
    }

    pub fn is_user(&self, key: &str) -> bool {
        self.user_keys.contains(key)
    }

    pub fn build(&mut self) {
        if cfg!(windows) {
            build_windows(self);
        } else {
            build_unix(self);
        }
    }
}

const WS: [char; 4] = [' ', '\t', '\r', '\n'];

fn strip_trailing_slashes(s: &str) -> &str {
    s.trim_end_matches('/')
}

fn strip_trailing_seps_win(s: &str) -> &str {
    s.trim_end_matches(['/', '\\'])
}

// ── Unix ─────────────────────────────────────────────────────────────────

const ETC_FILES: [(&str, &str); 3] = [
    ("/etc/environment", "/etc/environment"),
    ("/etc/set-environment", "/etc/set-environment"),
    ("/etc/profile.d/apps-bin-path.sh", "/etc/profile.d/"),
];

const REL_FILES: [(&str, &str); 8] = [
    ("/.profile", "~/.profile"),
    ("/.bash_profile", "~/.bash_profile"),
    ("/.bash_login", "~/.bash_login"),
    ("/.bashrc", "~/.bashrc"),
    ("/.zprofile", "~/.zprofile"),
    ("/.zshrc", "~/.zshrc"),
    ("/.cargo/env", "~/.cargo/env"),
    ("/.nimble/env", "~/.nimble/env"),
];

/// Expand only ~ and $HOME / ${HOME}. Intentionally minimal — matches the
/// prior implementations' behavior.
pub fn expand_vars_unix_tilde(s: &str, home: &str) -> String {
    if s == "~" {
        return home.to_owned();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return format!("{home}/{rest}");
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find('$') {
        out.push_str(&rest[..idx]);
        let after_dollar = &rest[idx + 1..];
        if let Some(tail) = after_dollar.strip_prefix("{HOME}") {
            out.push_str(home);
            rest = tail;
        } else if let Some(tail) = after_dollar.strip_prefix("HOME") {
            out.push_str(home);
            rest = tail;
        } else {
            out.push('$');
            rest = after_dollar;
        }
    }
    out.push_str(rest);
    out
}

pub fn parse_path_line(map: &mut SourceMap, raw_line: &str, label: &str, home: &str) {
    let line = raw_line.trim_matches(WS);
    if line.is_empty() || line.starts_with('#') {
        return;
    }

    let Some(mut value) = line
        .strip_prefix("export PATH=")
        .or_else(|| line.strip_prefix("PATH="))
    else {
        return;
    };

    // Strip surrounding quotes
    let b = value.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        value = &value[1..value.len() - 1];
    }

    // Strip inline comments (must be preceded by a space)
    if let Some(idx) = value.find(" #") {
        value = &value[..idx];
    }

    for raw_seg in value.split(':') {
        let seg = raw_seg.trim_matches(WS);
        if seg.is_empty() || seg == "$PATH" || seg == "${PATH}" || seg == "$path" {
            continue;
        }

        let tilde_expanded = expand_vars_unix_tilde(seg, home);
        if tilde_expanded.is_empty() {
            continue;
        }
        // Expand remaining $VAR / ${VAR} (e.g. $USER, ${XDG_STATE_HOME} on
        // NixOS) using the live environment. Anything still unresolved is
        // filtered below.
        let expanded = expandvars::posix(&tilde_expanded);
        if expanded.contains('$') {
            continue;
        }
        let mut normalized = strip_trailing_slashes(&expanded);
        if normalized.is_empty() {
            normalized = &expanded;
        }
        map.insert(normalized, label);
    }
}

pub fn parse_shell_config(map: &mut SourceMap, path: &str, label: &str, home: &str) {
    let Ok(bytes) = fs::read(path) else { return };
    let content = String::from_utf8_lossy(&bytes);
    for line in content.split('\n') {
        parse_path_line(map, line, label, home);
    }
}

fn build_unix(map: &mut SourceMap) {
    let home_raw = std::env::var_os("HOME")
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_default();
    let home = strip_trailing_slashes(&home_raw);

    for (path, label) in ETC_FILES {
        parse_shell_config(map, path, label, home);
    }
    for (rel, label) in REL_FILES {
        parse_shell_config(map, &format!("{home}{rel}"), label, home);
    }
}

// ── Windows ──────────────────────────────────────────────────────────────

fn read_registry_segments(map: &mut SourceMap, segs: &[String], label: &str) {
    for raw_seg in segs {
        let seg = raw_seg.trim_matches(WS);
        if seg.is_empty() {
            continue;
        }
        let expanded = expandvars::windows(seg);
        let normalized = strip_trailing_seps_win(&expanded);
        if normalized.is_empty() {
            continue;
        }
        map.insert(&normalized.to_ascii_lowercase(), label);
    }
}

fn build_windows(map: &mut SourceMap) {
    read_registry_segments(map, &winenv::read_raw_system_path_segments(), "System");
    read_registry_segments(map, &winenv::read_raw_user_path_segments(), "User");
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_first_seen_wins() {
        let mut map = SourceMap::default();
        map.insert("/usr/bin", "first");
        map.insert("/usr/bin", "second");
        assert_eq!(map.lookup("/usr/bin"), "first");
    }

    #[test]
    fn map_missing_key() {
        assert_eq!(SourceMap::default().lookup("/nonexistent"), "");
    }

    #[test]
    fn map_is_user() {
        let mut map = SourceMap::default();
        map.insert("c:\\a", "System");
        map.insert("c:\\a", "User");
        assert_eq!(map.lookup("c:\\a"), "System");
        assert!(map.is_user("c:\\a"));
        assert!(!map.is_user("c:\\b"));
    }

    #[test]
    fn expand_tilde() {
        assert_eq!(expand_vars_unix_tilde("~", "/home/user"), "/home/user");
        assert_eq!(
            expand_vars_unix_tilde("~/bin", "/home/user"),
            "/home/user/bin"
        );
    }

    #[test]
    fn expand_home_var() {
        assert_eq!(
            expand_vars_unix_tilde("$HOME/bin", "/home/user"),
            "/home/user/bin"
        );
        assert_eq!(
            expand_vars_unix_tilde("${HOME}/bin", "/home/user"),
            "/home/user/bin"
        );
    }

    #[test]
    fn expand_other_vars_pass_through_unchanged() {
        assert!(expand_vars_unix_tilde("$CARGO_HOME/bin", "/home/user").contains("$CARGO_HOME"));
        // Unterminated ${HOME is not expanded.
        assert_eq!(expand_vars_unix_tilde("${HOME/bin", "/h"), "${HOME/bin");
    }

    #[cfg(unix)]
    mod unix {
        use super::super::*;

        #[test]
        fn parse_path_line_export_form() {
            let mut map = SourceMap::default();
            parse_path_line(
                &mut map,
                "export PATH=/usr/bin:/usr/local/bin",
                "~/.bashrc",
                "/home/user",
            );
            assert_eq!(map.lookup("/usr/bin"), "~/.bashrc");
            assert_eq!(map.lookup("/usr/local/bin"), "~/.bashrc");
        }

        #[test]
        fn parse_path_line_no_export() {
            let mut map = SourceMap::default();
            parse_path_line(&mut map, "PATH=/opt/bin", "~/.profile", "/home/user");
            assert_eq!(map.lookup("/opt/bin"), "~/.profile");
        }

        #[test]
        fn parse_path_line_skips_path_var() {
            let mut map = SourceMap::default();
            parse_path_line(
                &mut map,
                "export PATH=/usr/bin:$PATH",
                "~/.bashrc",
                "/home/user",
            );
            assert_eq!(map.lookup("/usr/bin"), "~/.bashrc");
            assert_eq!(map.lookup("$PATH"), "");
        }

        #[test]
        fn parse_path_line_skips_unresolved_var() {
            let mut map = SourceMap::default();
            parse_path_line(
                &mut map,
                "export PATH=$UNKNOWN_PH_TEST_VAR/bin",
                "~/.bashrc",
                "/home/user",
            );
            assert_eq!(map.lookup("$UNKNOWN_PH_TEST_VAR/bin"), "");
        }

        #[test]
        fn parse_path_line_strips_quotes() {
            let mut map = SourceMap::default();
            parse_path_line(
                &mut map,
                "export PATH=\"/usr/bin\"",
                "~/.zshrc",
                "/home/user",
            );
            assert_eq!(map.lookup("/usr/bin"), "~/.zshrc");
        }

        #[test]
        fn parse_path_line_inline_comment() {
            let mut map = SourceMap::default();
            parse_path_line(
                &mut map,
                "export PATH=/usr/bin # added by installer",
                "~/.bashrc",
                "/home/user",
            );
            assert_eq!(map.lookup("/usr/bin"), "~/.bashrc");
        }

        #[test]
        fn parse_path_line_ignores_comment_lines() {
            let mut map = SourceMap::default();
            parse_path_line(&mut map, "# PATH=/usr/bin", "~/.bashrc", "/home/user");
            assert_eq!(map.lookup("/usr/bin"), "");
        }

        #[test]
        fn build_reads_a_real_config_file() {
            let dir = std::env::temp_dir().join(format!("ph-source-test-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let config = dir.join(".bashrc");
            std::fs::write(&config, "export PATH=/custom/tool/bin:$PATH\n").unwrap();

            let mut map = SourceMap::default();
            parse_shell_config(
                &mut map,
                config.to_str().unwrap(),
                "~/.bashrc",
                dir.to_str().unwrap(),
            );
            std::fs::remove_dir_all(&dir).unwrap();
            assert_eq!(map.lookup("/custom/tool/bin"), "~/.bashrc");
        }
    }
}
