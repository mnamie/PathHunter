use std::collections::HashMap;
use std::fs;

use crate::source::SourceMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryState {
    Ok,
    /// Link target text.
    Symlink(String),
    /// 1-based duplicate index.
    Duplicate(usize),
    Dead,
    File,
    Dangling,
    Empty,
}

impl EntryState {
    pub fn is_dead(&self) -> bool {
        matches!(
            self,
            EntryState::Dead | EntryState::Dangling | EntryState::File
        )
    }
}

#[derive(Debug, Clone)]
pub struct PathEntry {
    pub path: String,
    pub state: EntryState,
    pub source: String,
}

const SEP: char = if cfg!(windows) { ';' } else { ':' };

fn strip_trailing_sep(path: &str) -> &str {
    if cfg!(windows) {
        // Preserve drive roots like C:\ (length 3, second char is colon)
        if path.len() == 3 && path.as_bytes()[1] == b':' {
            return path;
        }
        return path.trim_end_matches(['/', '\\']);
    }
    let stripped = path.trim_end_matches('/');
    if stripped.is_empty() { path } else { stripped } // preserve lone "/"
}

fn read_link_text(path: &str) -> String {
    fs::read_link(path).map_or_else(
        |_| "(resolved)".to_owned(),
        |t| t.to_string_lossy().into_owned(),
    )
}

#[cfg(windows)]
fn check_filesystem(path: &str) -> EntryState {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

    let Ok(lst) = fs::symlink_metadata(path) else {
        return EntryState::Dead;
    };
    let attrs = lst.file_attributes();
    if attrs & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return if attrs & FILE_ATTRIBUTE_DIRECTORY != 0 {
            EntryState::Ok
        } else {
            EntryState::File
        };
    }

    // Reparse point (symlink/junction): verify the target actually resolves
    // to a directory, then report the target text best-effort.
    match fs::metadata(path) {
        Ok(st) if st.is_dir() => EntryState::Symlink(read_link_text(path)),
        _ => EntryState::Dangling,
    }
}

#[cfg(not(windows))]
fn check_filesystem(path: &str) -> EntryState {
    let Ok(lst) = fs::symlink_metadata(path) else {
        return EntryState::Dead;
    };
    let ft = lst.file_type();
    if ft.is_symlink() {
        return match fs::metadata(path) {
            Ok(st) if st.is_dir() => EntryState::Symlink(read_link_text(path)),
            _ => EntryState::Dangling,
        };
    }
    if ft.is_dir() {
        EntryState::Ok
    } else if ft.is_file() {
        EntryState::File
    } else {
        EntryState::Dead
    }
}

/// Splits `raw_path` and classifies each entry, annotating it with the
/// source label from `sm`.
pub fn scan(sm: &SourceMap, raw_path: &str) -> Vec<PathEntry> {
    let mut entries = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();

    for seg in raw_path.split(SEP) {
        if seg.is_empty() {
            entries.push(PathEntry {
                path: "(empty entry)".to_owned(),
                state: EntryState::Empty,
                source: String::new(),
            });
            continue;
        }

        let normalized = strip_trailing_sep(seg);
        let key = if cfg!(windows) {
            normalized.to_ascii_lowercase()
        } else {
            normalized.to_owned()
        };

        let source = sm.lookup(&key);
        let count = seen.entry(key.clone()).or_insert(0);
        *count += 1;

        let (state, source) = if *count > 1 {
            // If the path is in User registry (even if also in System),
            // attribute this duplicate to User — it's the redundant copy.
            let src = if sm.is_user(&key) { "User" } else { source };
            (EntryState::Duplicate(*count - 1), src)
        } else {
            (check_filesystem(normalized), source)
        };
        entries.push(PathEntry {
            path: seg.to_owned(),
            state,
            source: source.to_owned(),
        });
    }

    entries
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Fresh, uniquely named temp directory, removed on drop.
    pub struct TmpDir(pub PathBuf);

    impl TmpDir {
        pub fn new() -> TmpDir {
            static N: AtomicUsize = AtomicUsize::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("ph-test-{}-{n}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            TmpDir(fs::canonicalize(&p).unwrap_or(p))
        }
        pub fn path(&self) -> String {
            let s = self.0.to_string_lossy().into_owned();
            // canonicalize on Windows yields a \\?\ verbatim path; strip it so
            // the entry looks like a normal PATH segment.
            s.strip_prefix(r"\\?\").map(str::to_owned).unwrap_or(s)
        }
        pub fn join(&self, name: &str) -> String {
            PathBuf::from(self.path())
                .join(name)
                .to_string_lossy()
                .into_owned()
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn scan_empty(raw: &str) -> Vec<PathEntry> {
        scan(&SourceMap::default(), raw)
    }

    #[test]
    fn ok_entry() {
        let tmp = TmpDir::new();
        let entries = scan_empty(&tmp.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, EntryState::Ok);
    }

    #[test]
    fn dead_entry() {
        let entries = scan_empty("/this/path/does/not/exist/ever");
        assert_eq!(entries[0].state, EntryState::Dead);
    }

    #[test]
    fn empty_entry() {
        let raw = format!("/usr/bin{SEP}{SEP}/usr/local/bin");
        let entries = scan_empty(&raw);
        assert_eq!(entries[1].state, EntryState::Empty);
        assert_eq!(entries[1].path, "(empty entry)");
    }

    #[test]
    fn duplicate_entry() {
        let tmp = TmpDir::new();
        let p = tmp.path();
        let entries = scan_empty(&format!("{p}{SEP}{p}"));
        assert_eq!(entries[0].state, EntryState::Ok);
        assert_eq!(entries[1].state, EntryState::Duplicate(1));
    }

    #[test]
    fn third_occurrence_is_dup_index_2() {
        let tmp = TmpDir::new();
        let p = tmp.path();
        let entries = scan_empty(&format!("{p}{SEP}{p}{SEP}{p}"));
        assert_eq!(entries[2].state, EntryState::Duplicate(2));
    }

    #[test]
    fn trailing_separator_is_duplicate() {
        let tmp = TmpDir::new();
        let p = tmp.path();
        let slash = if cfg!(windows) { '\\' } else { '/' };
        let entries = scan_empty(&format!("{p}{SEP}{p}{slash}"));
        assert_eq!(entries[1].state, EntryState::Duplicate(1));
    }

    #[cfg(windows)]
    #[test]
    fn windows_duplicates_are_case_insensitive() {
        let tmp = TmpDir::new();
        let p = tmp.path();
        let entries = scan_empty(&format!("{p};{}", p.to_ascii_uppercase()));
        assert_eq!(entries[1].state, EntryState::Duplicate(1));
    }

    #[test]
    fn user_duplicate_attributed_to_user() {
        let tmp = TmpDir::new();
        let p = tmp.path();
        let key = if cfg!(windows) {
            p.to_ascii_lowercase()
        } else {
            p.clone()
        };
        let mut sm = SourceMap::default();
        sm.insert(&key, "System");
        sm.insert(&key, "User");
        let entries = scan(&sm, &format!("{p}{SEP}{p}"));
        assert_eq!(entries[0].source, "System");
        assert_eq!(entries[1].source, "User");
    }

    #[test]
    fn file_entry() {
        let tmp = TmpDir::new();
        let f = tmp.join("somefile.txt");
        fs::write(&f, "hi").unwrap();
        let entries = scan_empty(&f);
        assert_eq!(entries[0].state, EntryState::File);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks() {
        let tmp = TmpDir::new();
        fs::create_dir(tmp.join("real")).unwrap();
        std::os::unix::fs::symlink("real", tmp.join("link")).unwrap();
        let entries = scan_empty(&tmp.join("link"));
        match &entries[0].state {
            EntryState::Symlink(target) => assert!(!target.is_empty()),
            other => panic!("expected symlink, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn dangling_symlink() {
        let tmp = TmpDir::new();
        std::os::unix::fs::symlink("nowhere", tmp.join("dangling")).unwrap();
        let entries = scan_empty(&tmp.join("dangling"));
        assert_eq!(entries[0].state, EntryState::Dangling);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_file_is_dangling() {
        let tmp = TmpDir::new();
        fs::write(tmp.join("f"), "x").unwrap();
        std::os::unix::fs::symlink("f", tmp.join("flink")).unwrap();
        let entries = scan_empty(&tmp.join("flink"));
        assert_eq!(entries[0].state, EntryState::Dangling);
    }

    #[test]
    fn is_dead() {
        assert!(EntryState::Dead.is_dead());
        assert!(EntryState::Dangling.is_dead());
        assert!(EntryState::File.is_dead());
        assert!(!EntryState::Ok.is_dead());
        assert!(!EntryState::Symlink("x".into()).is_dead());
        assert!(!EntryState::Duplicate(1).is_dead());
        assert!(!EntryState::Empty.is_dead());
    }

    #[test]
    fn strip_trailing_sep_preserves_roots() {
        if cfg!(windows) {
            assert_eq!(strip_trailing_sep("C:\\"), "C:\\");
            assert_eq!(strip_trailing_sep("C:\\foo\\\\"), "C:\\foo");
            assert_eq!(strip_trailing_sep("C:\\foo/"), "C:\\foo");
        } else {
            assert_eq!(strip_trailing_sep("/"), "/");
            assert_eq!(strip_trailing_sep("/usr/bin//"), "/usr/bin");
        }
    }
}
