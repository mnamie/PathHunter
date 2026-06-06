# Path Hunter

A cross-platform `PATH` auditor. Scans each entry in your `PATH` and reports dead directories, duplicates, symlinks, and empty entries — with colored output and, where possible, the config source that added each entry.

**Windows** — annotates entries as `User` or `System` based on the registry.
**Unix** — traces entries back to the shell config file that set them (`.bashrc`, `.zshrc`, etc.).

## Requirements

[Rust](https://rustup.rs) (stable, 2024 edition) to build. No crate dependencies; the resulting binary has no runtime dependencies.

## Install

Build a standalone executable:

```sh
cargo build --release
./target/release/ph --help
```

Cross-compiling needs the target's standard library and a linker for it:

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

For macOS targets from a non-Mac host, [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) is the easiest route.

## Development

```sh
cargo run -- --help                          # run from source
cargo test                                   # run tests
cargo fmt --check                            # formatting check
cargo clippy --all-targets -- -D warnings    # lints
```

## Usage

```
ph [clean] [--only-dead] [--no-color] [--help]

  clean             Remove dead entries from PATH (Windows only)
  --only-dead, -d   Show only dead/missing entries
  --no-color,  -n   Plain text output (no ANSI colors)
  --help,      -h   Show this help
```

## Architecture

```
src/
├── main.rs         Entry point — TTY/console detection, dispatch
├── args.rs         parse() → Config { command, only_dead, no_color }
├── source.rs       SourceMap: maps a normalized path back to where it was set
│                     Unix    — scans shell config files (.bashrc, .zshrc, …)
│                     Windows — delegates registry reads to winenv.rs
├── winenv.rs       Windows-only: raw advapi32/user32/shell32/kernel32 extern calls for
│                     registry read/write, WM_SETTINGCHANGE broadcast, admin
│                     check, console setup — cfg-gated out of non-Windows builds
├── expandvars.rs   posix() / windows() — hand-written scanners replicating
│                     CPython's posixpath/ntpath expandvars (no regex dependency)
├── audit.rs        scan(): classifies each PATH entry into an EntryState enum
├── display.rs      render(): columnar ANSI output + summary line
└── clean.rs        Windows-only: previews dead entries, confirms, rewrites registry
```

**Data flow (audit)**

```
$PATH string
    └─ audit::scan()              splits on ; (Windows) or : (Unix)
           └─ classify each segment
                  ├─ SourceMap::lookup()          annotates with config source
                  └─ symlink_metadata/metadata    → EntryState
    └─ Vec<PathEntry>
           └─ display::render()   prints header, rows, summary
```

**Data flow (clean — Windows only)**

```
$PATH string
    └─ audit::scan()                              same audit pass
    └─ filter dead entries by source
    └─ preview + confirm prompt
    └─ winenv::read_raw_user_path_segments()      reads unexpanded registry value
    └─ filter surviving segments
    └─ winenv::write_user_path_to_registry()      writes back
    └─ winenv::broadcast_env_change()             WM_SETTINGCHANGE to running processes
```

## Output legend

| Symbol | Meaning |
|--------|---------|
| `✓` | Directory exists |
| `~` | Symlink to an existing directory |
| `⚠` | Duplicate entry |
| `✗` | Dead — directory missing, dangling symlink, or a file |
| `!` | Empty entry (means current directory — security risk) |
