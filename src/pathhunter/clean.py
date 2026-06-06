from __future__ import annotations
import os
import sys

from .args import Config
from .audit import audit_scan, path_entry_is_dead


def _norm(s: str) -> str:
    return os.path.expandvars(s).rstrip("/\\").lower()


def _intra_dup_info(segs: list[str]) -> list[tuple[str, int]]:
    """Return (display_path, extra_count) for paths appearing more than once."""
    counts: dict[str, int] = {}
    first_display: dict[str, str] = {}
    for s in segs:
        key = _norm(s)
        if key not in first_display:
            first_display[key] = os.path.expandvars(s).rstrip("/\\")
        counts[key] = counts.get(key, 0) + 1
    return [(first_display[k], counts[k] - 1) for k in counts if counts[k] > 1]


def _dedup_segs(segs: list[str]) -> list[str]:
    """Keep first occurrence of each path, drop subsequent duplicates."""
    seen: set[str] = set()
    result = []
    for s in segs:
        key = _norm(s)
        if key not in seen:
            seen.add(key)
            result.append(s)
    return result


def _prompt(msg: str) -> bool:
    """Return True if user answers y, False otherwise. Raises SystemExit on Ctrl-C."""
    try:
        return input(msg).strip().lower() == "y"
    except (EOFError, KeyboardInterrupt):
        print()
        raise SystemExit(0)


def run_clean(cfg: Config) -> int:
    if sys.platform != "win32":
        print("ph: clean is only supported on Windows")
        return 1

    from .source import (
        SourceMap,
        broadcast_env_change,
        is_admin,
        read_raw_system_path_segments,
        read_raw_user_path_segments,
        write_system_path_to_registry,
        write_user_path_to_registry,
    )

    raw = os.environ.get("PATH", "")
    if not raw:
        print("ph: PATH is not set or empty")
        return 1

    sm = SourceMap()
    sm.build()
    entries = audit_scan(sm, raw)

    admin = is_admin()

    user_segs = read_raw_user_path_segments()
    system_segs = read_raw_system_path_segments()
    expanded_user = {_norm(s) for s in user_segs}
    expanded_system = {_norm(s) for s in system_segs}

    # ── Classify dead entries ─────────────────────────────────────────────────
    dead = [e for e in entries if path_entry_is_dead(e)]
    user_dead: list = []
    system_dead: list = []
    shell_dead: list = []
    for e in dead:
        key = e.path.rstrip("/\\").lower()
        if key in expanded_user:
            user_dead.append(e)
        elif key in expanded_system:
            system_dead.append(e)
        else:
            shell_dead.append(e)

    # ── Cross-registry duplicates: User entries already in System PATH ────────
    user_cross_dups = [s for s in user_segs if _norm(s) in expanded_system]

    # ── Intra-registry duplicates ─────────────────────────────────────────────
    user_intra_dups = _intra_dup_info(user_segs)
    system_intra_dups = _intra_dup_info(system_segs)

    any_actionable = bool(
        user_dead
        or user_cross_dups
        or user_intra_dups
        or (admin and (system_dead or system_intra_dups))
    )
    any_issue = bool(dead or user_cross_dups or user_intra_dups or system_intra_dups)

    if not any_issue:
        print("Nothing to clean.")
        return 0

    # ── Report ────────────────────────────────────────────────────────────────
    if dead:
        print("Dead entries:\n")
        if user_dead:
            print("  [User]")
            for e in user_dead:
                print(f"    ✗  {e.path}")
            print()
        if system_dead:
            suffix = "" if admin else "  ⚠ requires administrator — will be skipped"
            print(f"  [System]{suffix}")
            for e in system_dead:
                print(f"    ✗  {e.path}")
            print()
        if shell_dead:
            print("  [Shell-injected — not in registry, skipped]")
            for e in shell_dead:
                print(f"    ✗  {e.path}")
            print()

    if user_cross_dups:
        print("Redundant User PATH entries (already in System PATH):\n")
        for s in user_cross_dups:
            print(f"    ⚠  {s}")
        print()

    if user_intra_dups:
        total_extra = sum(x for _, x in user_intra_dups)
        print(f"Duplicate entries within User PATH ({total_extra} extra {'copy' if total_extra == 1 else 'copies'}):\n")
        for path, extra in user_intra_dups:
            print(f"    ⚠  {path}  ({extra + 1} copies)")
        print()

    if system_intra_dups:
        total_extra = sum(x for _, x in system_intra_dups)
        suffix = "" if admin else "  ⚠ requires administrator — will be skipped"
        print(f"Duplicate entries within System PATH{suffix} ({total_extra} extra {'copy' if total_extra == 1 else 'copies'}):\n")
        for path, extra in system_intra_dups:
            print(f"    ⚠  {path}  ({extra + 1} copies)")
        print()

    if not any_actionable:
        if system_dead or system_intra_dups:
            print("Re-run as administrator to remove System PATH entries.")
        return 0

    changed = False

    # ── Dead: User ────────────────────────────────────────────────────────────
    if user_dead:
        n = len(user_dead)
        noun = "entry" if n == 1 else "entries"
        if _prompt(f"Remove {n} dead User PATH {noun}? [y/N] "):
            dead_keys = {e.path.rstrip("/\\").lower() for e in user_dead}
            user_segs = [s for s in user_segs if _norm(s) not in dead_keys]
            write_user_path_to_registry(user_segs)
            changed = True
            print(f"Removed {n} dead {noun}.")

    # ── Dead: System ─────────────────────────────────────────────────────────
    if admin and system_dead:
        n = len(system_dead)
        noun = "entry" if n == 1 else "entries"
        if _prompt(f"Remove {n} dead System PATH {noun}? [y/N] "):
            dead_keys = {e.path.rstrip("/\\").lower() for e in system_dead}
            system_segs = [s for s in system_segs if _norm(s) not in dead_keys]
            write_system_path_to_registry(system_segs)
            changed = True
            print(f"Removed {n} dead {noun}.")

    # ── Cross-registry duplicates: User→System ────────────────────────────────
    if user_cross_dups:
        # Recompute after possible dead removal above
        expanded_system_now = {_norm(s) for s in system_segs}
        user_cross_dups = [s for s in user_segs if _norm(s) in expanded_system_now]
        if user_cross_dups:
            n = len(user_cross_dups)
            noun = "entry" if n == 1 else "entries"
            if _prompt(f"Remove {n} User PATH {noun} already in System PATH? [y/N] "):
                dup_keys = {_norm(s) for s in user_cross_dups}
                user_segs = [s for s in user_segs if _norm(s) not in dup_keys]
                write_user_path_to_registry(user_segs)
                changed = True
                print(f"Removed {n} redundant {noun}.")

    # ── Intra-duplicates: User ────────────────────────────────────────────────
    user_intra_dups = _intra_dup_info(user_segs)
    if user_intra_dups:
        total_extra = sum(x for _, x in user_intra_dups)
        noun = "copy" if total_extra == 1 else "copies"
        if _prompt(f"Remove {total_extra} extra {noun} within User PATH? [y/N] "):
            user_segs = _dedup_segs(user_segs)
            write_user_path_to_registry(user_segs)
            changed = True
            print(f"Removed {total_extra} extra {noun}.")

    # ── Intra-duplicates: System ──────────────────────────────────────────────
    if admin:
        system_intra_dups = _intra_dup_info(system_segs)
        if system_intra_dups:
            total_extra = sum(x for _, x in system_intra_dups)
            noun = "copy" if total_extra == 1 else "copies"
            if _prompt(f"Remove {total_extra} extra {noun} within System PATH? [y/N] "):
                system_segs = _dedup_segs(system_segs)
                write_system_path_to_registry(system_segs)
                changed = True
                print(f"Removed {total_extra} extra {noun}.")

    if changed:
        broadcast_env_change()
        print("PATH updated. Restart your shell for changes to take effect.")
    elif system_dead and not admin:
        print("Re-run as administrator to remove System PATH entries.")

    return 0
