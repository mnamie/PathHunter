//! Ports of CPython 3.13's posixpath.expandvars / ntpath.expandvars.
//! Verified against the actual interpreter (not reconstructed from memory) —
//! both are regex.sub-with-replacer implementations; unresolved references
//! are left in the output unchanged. Rust's stdlib has no regex engine, so
//! this is a hand-written scanner that replicates the exact match semantics
//! of the original patterns:
//!   POSIX:   \$(\w+|\{[^}]*\}?)
//!   Windows: '[^']*'?|%(%|[^%]*%?)|\$(\$|[-\w]+|\{[^}]*\}?)
//!
//! The scanners take the variable lookup as a closure so tests can supply
//! fixed values instead of mutating the process environment (`set_var` is
//! `unsafe` and racy under cargo's parallel test threads).

use std::env;

fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn is_name_char(c: u8) -> bool {
    is_word_char(c) || c == b'-'
}

fn lookup_env(name: &str) -> Option<String> {
    env::var_os(name).map(|v| v.to_string_lossy().into_owned())
}

pub fn posix(s: &str) -> String {
    posix_with(s, lookup_env)
}

pub fn windows(s: &str) -> String {
    windows_with(s, lookup_env)
}

/// Appends either the variable's value or, if unset, the original match text.
fn push_lookup(
    out: &mut Vec<u8>,
    name: &[u8],
    original: &[u8],
    lookup: &impl Fn(&str) -> Option<String>,
) {
    // Names are ASCII-only by construction except inside ${...}/%...%, where
    // any bytes are allowed; slices always fall on UTF-8 boundaries because
    // the delimiters are ASCII.
    let name = std::str::from_utf8(name).unwrap_or_default();
    match lookup(name) {
        Some(val) => out.extend_from_slice(val.as_bytes()),
        None => out.extend_from_slice(original),
    }
}

fn into_string(out: Vec<u8>) -> String {
    // Only whole UTF-8 sequences are ever copied, so this cannot fail.
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

pub fn posix_with(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    if !s.contains('$') {
        return s.to_owned();
    }
    let s = s.as_bytes();
    let mut out = Vec::with_capacity(s.len());

    let mut i = 0;
    while i < s.len() {
        if s[i] == b'$' && i + 1 < s.len() {
            let rest = &s[i + 1..];
            if is_word_char(rest[0]) {
                let j = rest.iter().take_while(|&&c| is_word_char(c)).count();
                let match_len = 1 + j;
                push_lookup(&mut out, &rest[..j], &s[i..i + match_len], &lookup);
                i += match_len;
                continue;
            } else if rest[0] == b'{' {
                let mut j = 1;
                while j < rest.len() && rest[j] != b'}' {
                    j += 1;
                }
                let has_close = j < rest.len();
                let match_len = 1 + if has_close { j + 1 } else { j };
                if has_close {
                    push_lookup(&mut out, &rest[1..j], &s[i..i + match_len], &lookup);
                } else {
                    out.extend_from_slice(&s[i..i + match_len]);
                }
                i += match_len;
                continue;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    into_string(out)
}

pub fn windows_with(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    if !s.contains('$') && !s.contains('%') {
        return s.to_owned();
    }
    let s = s.as_bytes();
    let mut out = Vec::with_capacity(s.len());

    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'\'' {
            // '[^']*'? — quoted spans pass through untouched (ntpath quirk).
            let mut j = i + 1;
            while j < s.len() && s[j] != b'\'' {
                j += 1;
            }
            let end = if j < s.len() { j + 1 } else { j };
            out.extend_from_slice(&s[i..end]);
            i = end;
            continue;
        } else if c == b'%' {
            if i + 1 < s.len() && s[i + 1] == b'%' {
                out.push(b'%');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < s.len() && s[j] != b'%' {
                j += 1;
            }
            let has_close = j < s.len();
            let end = if has_close { j + 1 } else { j };
            if has_close {
                push_lookup(&mut out, &s[i + 1..j], &s[i..end], &lookup);
            } else {
                out.extend_from_slice(&s[i..end]);
            }
            i = end;
            continue;
        } else if c == b'$' && i + 1 < s.len() {
            let rest = &s[i + 1..];
            if rest[0] == b'$' {
                out.push(b'$');
                i += 2;
                continue;
            } else if is_name_char(rest[0]) {
                let j = rest.iter().take_while(|&&c| is_name_char(c)).count();
                let match_len = 1 + j;
                push_lookup(&mut out, &rest[..j], &s[i..i + match_len], &lookup);
                i += match_len;
                continue;
            } else if rest[0] == b'{' {
                let mut j = 1;
                while j < rest.len() && rest[j] != b'}' {
                    j += 1;
                }
                let has_close = j < rest.len();
                let match_len = 1 + if has_close { j + 1 } else { j };
                if has_close {
                    push_lookup(&mut out, &rest[1..j], &s[i..i + match_len], &lookup);
                } else {
                    out.extend_from_slice(&s[i..i + match_len]);
                }
                i += match_len;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    into_string(out)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env(name: &str) -> Option<String> {
        match name {
            "FOO" => Some("BAR".into()),
            "FOO_BAR" => Some("BAZ".into()),
            _ => None,
        }
    }

    fn expect_posix(input: &str, want: &str) {
        assert_eq!(posix_with(input, test_env), want);
    }

    fn expect_windows(input: &str, want: &str) {
        assert_eq!(windows_with(input, test_env), want);
    }

    #[test]
    fn posix_passes_through_strings_with_no_dollar() {
        expect_posix("plain/path", "plain/path");
    }
    #[test]
    fn posix_expands_bare_var() {
        expect_posix("$FOO/bin", "BAR/bin");
    }
    #[test]
    fn posix_expands_braced_var() {
        expect_posix("${FOO}/bin", "BAR/bin");
    }
    #[test]
    fn posix_expands_names_with_underscores() {
        expect_posix("$FOO_BAR/bin", "BAZ/bin");
    }
    #[test]
    fn posix_leaves_unresolved_braced_var_unchanged() {
        expect_posix("${UNKNOWN}/bin", "${UNKNOWN}/bin");
    }
    #[test]
    fn posix_leaves_unresolved_bare_var_unchanged() {
        expect_posix("$UNKNOWN/bin", "$UNKNOWN/bin");
    }
    #[test]
    fn posix_leaves_unterminated_brace_unchanged() {
        expect_posix("${NOCLOSE/bin", "${NOCLOSE/bin");
    }
    #[test]
    fn posix_reads_real_environment() {
        // PATH is always set in the test environment.
        assert_ne!(posix("$PATH"), "$PATH");
    }

    #[test]
    fn windows_passes_through_strings_with_no_dollar_or_percent() {
        expect_windows("plain/path", "plain/path");
    }
    #[test]
    fn windows_expands_percent_var() {
        expect_windows("%FOO%\\bin", "BAR\\bin");
    }
    #[test]
    fn windows_leaves_unresolved_percent_var_unchanged() {
        expect_windows("%UNKNOWN%\\bin", "%UNKNOWN%\\bin");
    }
    #[test]
    fn windows_leaves_percent_var_without_closing_percent_unchanged() {
        expect_windows("%NOCLOSE\\bin", "%NOCLOSE\\bin");
    }
    #[test]
    fn windows_collapses_double_percent() {
        expect_windows("a%%b", "a%b");
    }
    #[test]
    fn windows_lone_percent_is_left_unchanged() {
        expect_windows("100%done", "100%done");
    }
    #[test]
    fn windows_expands_bare_var() {
        expect_windows("$FOO/bin", "BAR/bin");
    }
    #[test]
    fn windows_expands_braced_var() {
        expect_windows("${FOO}/bin", "BAR/bin");
    }
    #[test]
    fn windows_leaves_unresolved_braced_var_unchanged() {
        expect_windows("${UNKNOWN}/bin", "${UNKNOWN}/bin");
    }
    #[test]
    fn windows_leaves_unresolved_bare_var_unchanged() {
        expect_windows("$UNKNOWN/bin", "$UNKNOWN/bin");
    }
    #[test]
    fn windows_collapses_double_dollar() {
        expect_windows("$$literal", "$literal");
    }
    #[test]
    fn windows_var_name_includes_hyphens() {
        expect_windows("$FOO-BAR/bin", "$FOO-BAR/bin");
    }
    #[test]
    fn windows_leaves_unterminated_brace_unchanged() {
        expect_windows("${NOCLOSE/bin", "${NOCLOSE/bin");
    }
    #[test]
    fn windows_quoted_span_passes_through() {
        expect_windows("'%FOO%'\\%FOO%", "'%FOO%'\\BAR");
    }
    #[test]
    fn non_ascii_passes_through() {
        expect_windows("C:\\Ünïcødé\\%FOO%", "C:\\Ünïcødé\\BAR");
        expect_posix("/opt/ñ/$FOO", "/opt/ñ/BAR");
    }
}
