# PathHunter

Cross-platform PATH auditor, implemented in Rust (stable, edition 2024). Ships as a standalone executable (`ph`) via `cargo build --release`. Zero crate dependencies.

## After editing code

Always run before considering a task done:

```sh
cargo fmt --check                          # formatting
cargo clippy --all-targets -- -D warnings  # lints
cargo test                                 # tests
cargo build                                # compile check (debug)
```

## Dev commands

```sh
cargo run -- --help                                  # run from source
cargo build --release                                # standalone executable (size-optimized profile)
cargo build --release --target x86_64-unknown-linux-musl   # cross-compile (needs `rustup target add` + a linker)
```

## Notes

- This machine is Windows, so `cargo test` alone never exercises the `#[cfg(unix)]` code/tests in `source.rs`/`audit.rs` (shell-config parsing, symlink handling) — they aren't compiled at all. Verify them for real with a Linux toolchain, e.g. a container: `docker run --rm -v "<repo>:/src" -w /src -e CARGO_TARGET_DIR=/tmp/target rust:1-slim cargo test` (the separate target dir avoids clobbering the host's `target/`).
- Rust's stdlib has no registry, `user32`, or `advapi32` bindings. Windows registry read/write (`Path` under `HKCU\Environment` / `HKLM\...\Session Manager\Environment`), the `WM_SETTINGCHANGE` broadcast, the admin check, and console-mode setup are done via raw `#[link(name = "...")] unsafe extern "system"` declarations in `src/winenv.rs` — deliberately no `windows-sys` crate. Everything lives in a `#[cfg(windows)] mod imp`; a `#[cfg(not(windows))] mod imp` provides inert stubs with the same signatures.
- `src/audit.rs`'s Windows filesystem check uses `std::fs::symlink_metadata` + `MetadataExt::file_attributes()` to test `FILE_ATTRIBUTE_REPARSE_POINT` exactly (any reparse point, not just what `is_symlink()` reports), then `fs::metadata` to confirm the target is a directory.
- `expandvars::posix`/`expandvars::windows` in `src/expandvars.rs` are ports of CPython 3.13's `posixpath.expandvars`/`ntpath.expandvars` — hand-written byte scanners (no regex) that replicate the exact match semantics of the original patterns, verified against the real interpreter. Don't "simplify" these without re-checking against CPython's source; the edge cases (`%%` collapsing, unresolved refs staying literal, quoted spans, hyphenated Windows var names) are load-bearing for `clean`'s duplicate detection, and are all covered in that file's tests. The `*_with` variants take a lookup closure so tests never call `env::set_var` (unsafe, and racy under parallel tests).
- Column widths/truncation in `src/display.rs` count `char`s, not bytes (slicing a `&str` mid-UTF-8 panics).
- `src/clean.rs`'s report and `prompt()` share a single `BufWriter<StdoutLock>` so buffered report text is flushed (in order) before each interactive prompt. Don't open a second stdout handle there.
