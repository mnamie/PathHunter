from __future__ import annotations
import os
import sys

from .args import Command, parse_args


def main() -> int:
    cfg = parse_args(sys.argv[1:])

    if cfg.command == Command.HELP:
        from .args import print_help

        print_help()
        return 0

    # Color/TTY detection
    if not cfg.no_color:
        if sys.platform == "win32":
            from .source import setup_windows_console

            if not setup_windows_console():
                cfg.no_color = True
        elif not sys.stdout.isatty():
            cfg.no_color = True

    if cfg.command == Command.CLEAN:
        from .clean import run_clean

        return run_clean(cfg)

    raw = os.environ.get("PATH", "")
    if not raw:
        print("ph: PATH is not set or empty", file=sys.stderr)
        return 1

    from .source import SourceMap
    from .audit import audit_scan
    from .display import render

    sm = SourceMap()
    sm.build()
    entries = audit_scan(sm, raw)
    render(sys.stdout, entries, cfg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
