# Path Hunter

A cross-platform `PATH` auditor. Scans each entry in your `PATH` and reports dead directories, duplicates, symlinks, and empty entries — with colored output and, where possible, the config source that added each entry.

**Windows** — annotates entries as `User` or `System` based on the registry.  
**Unix** — traces entries back to the shell config file that set them (`.bashrc`, `.zshrc`, etc.).

## Building

Requires a C99 compiler (`gcc`, `clang`, or MSVC) and `make`.

```sh
make
./ph [clean] [--no-color] [--only-dead] [--help]
```

On Windows (MinGW):

```sh
make
ph.exe [clean] [--no-color] [--only-dead] [--help]
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
├── main.c      Entry point — arg parsing, TTY detection, dispatch
├── args.h/.c   parse_args() → Config { command, only_dead, no_color }
├── arena.h/.c  Bump allocator — all per-scan strings live here
├── strutil.h/.c  Path normalization, UTF-8 codepoint counting
├── source.h/.c SourceMap: maps a normalized path back to where it was set
│                 Windows — reads HKLM/HKCU registry keys
│                 Unix    — scans shell config files (.bashrc, .zshrc, …)
├── audit.h/.c  audit_scan(): classifies each PATH entry via EntryState
├── display.h/.c  render(): columnar ANSI output + summary line
└── clean.h/.c  Windows-only: previews dead entries, confirms, rewrites registry
```

**Data flow (audit)**

```
$PATH string
    └─ audit_scan()              splits on ; (Windows) or : (Unix)
           └─ classify each segment
                  ├─ source_map_lookup()   annotates with config source
                  └─ lstat / GetFileAttributesW → EntryState
    └─ PathEntry[]
           └─ render()           prints header, rows, summary
```

**Data flow (clean — Windows only)**

```
$PATH string
    └─ audit_scan()                   same audit pass
    └─ filter dead entries by source
    └─ preview + confirm prompt
    └─ read_raw_user_path_segments()  reads unexpanded registry value
    └─ filter surviving segments
    └─ write_user_path_to_registry()  writes back
    └─ broadcast_env_change()         WM_SETTINGCHANGE to running processes
```

## Output legend

| Symbol | Meaning |
|--------|---------|
| `✓` | Directory exists |
| `~` | Symlink to an existing directory |
| `⚠` | Duplicate entry |
| `✗` | Dead — directory missing, dangling symlink, or a file |
| `!` | Empty entry (means current directory — security risk) |
