# Path Hunter

A cross-platform `PATH` auditor. Scans each entry in your `PATH` and reports dead directories, duplicates, symlinks, and empty entries — with colored output and, where possible, the config source that added each entry.

**Windows** — annotates entries as `User` or `System` based on the registry.  
**Unix** — traces entries back to the shell config file that set them (`.bashrc`, `.zshrc`, etc.).

## Building

Requires [Zig 0.15](https://ziglang.org/download/).

```sh
zig build
./zig-out/bin/ph [clean] [--no-color] [--only-dead] [--help]
```

Run tests:

```sh
zig build test
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
├── main.zig     Entry point — arg parsing, I/O setup, dispatch
├── args.zig     parseArgs → Config { command, only_dead, no_color }
├── audit.zig    Auditor: classifies each PATH entry via EntryState tagged union
├── clean.zig    Windows-only: previews dead entries, confirms, rewrites registry
├── source.zig   SourceMap: maps a normalized path back to where it was set
│                  Windows — reads HKLM/HKCU registry keys
│                  Unix    — scans shell config files (.bashrc, .zshrc, etc.)
└── display.zig  Renderer: columnar ANSI output + summary line
```

**Data flow (audit)**

```
$PATH string
    └─ Auditor.scan()              splits on ; (Windows) or : (Unix)
           └─ Auditor.classify()
                  ├─ SourceMap.lookup()    annotates with config source
                  └─ checkFilesystem()     → EntryState (ok/dead/symlink/…)
    └─ []PathEntry
           └─ Renderer.render()    prints header, rows, summary
```

**Data flow (clean — Windows only)**

```
$PATH string
    └─ Auditor.scan()                  same audit pass
    └─ filter dead entries by source
    └─ preview + confirm prompt
    └─ readRawUserPathSegments()       reads unexpanded registry value
    └─ filter surviving segments
    └─ writeUserPathToRegistry()       writes back
    └─ broadcastEnvChange()            WM_SETTINGCHANGE to running processes
```

## Output legend

| Symbol | Meaning |
|--------|---------|
| `✓` | Directory exists |
| `~` | Symlink to an existing directory |
| `⚠` | Duplicate entry |
| `✗` | Dead — directory missing, dangling symlink, or a file |
| `!` | Empty entry (means current directory — security risk) |
