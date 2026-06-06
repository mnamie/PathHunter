# Path Hunter

A cross-platform `PATH` auditor. Scans each entry in your `PATH` and reports dead directories, duplicates, symlinks, and empty entries — with colored output and, where possible, the config source that added each entry.

**Windows** — annotates entries as `User` or `System` based on the registry.  
**Unix** — traces entries back to the shell config file that set them (`.bashrc`, `.zshrc`, etc.).

## Requirements

Python 3.11+ and [uv](https://docs.astral.sh/uv/).

## Install

```sh
uv tool install .
ph --help
```

## Development

```sh
uv sync                  # create .venv and install dev deps
uv run ph                # run from source
uv run pytest            # run tests
uv run ruff check src/   # lint
uv run pyright src/      # type check (same engine as Pylance)
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
src/pathhunter/
├── __main__.py   Entry point — TTY detection, dispatch
├── args.py       parse_args() → Config { command, only_dead, no_color }
├── source.py     SourceMap: maps a normalized path back to where it was set
│                   Windows — reads HKLM/HKCU registry keys (winreg)
│                   Unix    — scans shell config files (.bashrc, .zshrc, …)
├── audit.py      audit_scan(): classifies each PATH entry via EntryState
├── display.py    render(): columnar ANSI output + summary line
└── clean.py      Windows-only: previews dead entries, confirms, rewrites registry
```

**Data flow (audit)**

```
$PATH string
    └─ audit_scan()              splits on ; (Windows) or : (Unix)
           └─ classify each segment
                  ├─ SourceMap.lookup()   annotates with config source
                  └─ os.lstat() → EntryState
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
