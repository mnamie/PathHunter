from __future__ import annotations
import os
import sys

from .args import Config
from .audit import audit_scan, path_entry_is_dead


def run_clean(cfg: Config) -> int:
    if sys.platform != "win32":
        print("ph: clean is only supported on Windows")
        return 1

    from .source import (
        SourceMap,
        broadcast_env_change,
        read_raw_user_path_segments,
        write_user_path_to_registry,
    )

    raw = os.environ.get("PATH", "")
    if not raw:
        print("ph: PATH is not set or empty")
        return 1

    sm = SourceMap()
    sm.build()
    entries = audit_scan(sm, raw)

    dead = [e for e in entries if path_entry_is_dead(e)]
    n_user = sum(1 for e in dead if e.source == "User")
    n_system = sum(1 for e in dead if e.source == "System")
    n_unknown = sum(1 for e in dead if e.source not in ("User", "System"))

    if not dead:
        print("No dead entries found.")
        return 0

    print("Dead entries to remove:\n")

    dead_keys: list[str] = []

    if n_user:
        print("  [User]")
        for e in dead:
            if e.source == "User":
                print(f"    ✗  {e.path}")
                dead_keys.append(e.path.lower())
        print()

    if n_system:
        print("  [System]  ⚠ requires administrator — will be skipped")
        for e in dead:
            if e.source == "System":
                print(f"    ✗  {e.path}")
        print()

    if n_unknown:
        print("  [Unknown source — skipped]")
        for e in dead:
            if e.source not in ("User", "System"):
                print(f"    ✗  {e.path}")
        print()

    if not n_user:
        print("No User PATH entries to remove (System entries require administrator).")
        return 0

    noun = "entry" if n_user == 1 else "entries"
    try:
        answer = input(f"Remove {n_user} User PATH {noun}? [y/N] ").strip()
    except (EOFError, KeyboardInterrupt):
        print("\nNo changes made.")
        return 0

    if answer.lower() != "y":
        print("No changes made.")
        return 0

    raw_segs = read_raw_user_path_segments()
    surviving = [seg for seg in raw_segs if seg.rstrip("/\\").lower() not in dead_keys]
    write_user_path_to_registry(surviving)
    broadcast_env_change()
    print(f"Removed {n_user} {noun}. User PATH updated.")
    return 0
