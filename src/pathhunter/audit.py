from __future__ import annotations
import os
import stat
import sys
from dataclasses import dataclass
from enum import Enum, auto
from pathlib import Path
from typing import Optional, Protocol


class _SourceMap(Protocol):
    def lookup(self, key: str) -> str: ...
    def is_user(self, key: str) -> bool: ...


class EntryState(Enum):
    OK = auto()
    SYMLINK = auto()
    DUPLICATE = auto()
    DEAD = auto()
    FILE = auto()
    DANGLING = auto()
    EMPTY = auto()


@dataclass
class PathEntry:
    path: str
    state: EntryState = EntryState.OK
    dup_index: int = 0
    symlink_target: Optional[str] = None
    source: str = ""


def path_entry_is_dead(e: PathEntry) -> bool:
    return e.state in (EntryState.DEAD, EntryState.DANGLING, EntryState.FILE)


def _strip_trailing_sep(path: str) -> str:
    if sys.platform == "win32":
        # Preserve drive roots like C:\ (length 3, second char is colon)
        if len(path) == 3 and path[1] == ":":
            return path
        return path.rstrip("/\\")
    stripped = path.rstrip("/")
    return stripped if stripped else path  # preserve lone "/"


def _check_filesystem(path: str) -> tuple[EntryState, Optional[str]]:
    try:
        lst = os.lstat(path)
    except OSError:
        return EntryState.DEAD, None

    if stat.S_ISLNK(lst.st_mode):
        try:
            st = os.stat(path)
        except OSError:
            return EntryState.DANGLING, None
        if not stat.S_ISDIR(st.st_mode):
            return EntryState.DANGLING, None
        try:
            target = str(Path(path).readlink())
        except OSError:
            return EntryState.SYMLINK, "(resolved)"
        return EntryState.SYMLINK, target

    if stat.S_ISDIR(lst.st_mode):
        return EntryState.OK, None
    if stat.S_ISREG(lst.st_mode):
        return EntryState.FILE, None
    return EntryState.DEAD, None


def audit_scan(sm: _SourceMap, raw_path: str) -> list[PathEntry]:
    sep = ";" if sys.platform == "win32" else ":"
    entries: list[PathEntry] = []
    seen: dict[str, int] = {}

    for seg in raw_path.split(sep):
        e = PathEntry(path=seg)

        if not seg:
            e.path = "(empty entry)"
            e.state = EntryState.EMPTY
        else:
            normalized = _strip_trailing_sep(seg)
            key = normalized.lower() if sys.platform == "win32" else normalized
            e.source = sm.lookup(key)

            seen[key] = seen.get(key, 0) + 1
            count = seen[key]
            if count > 1:
                e.state = EntryState.DUPLICATE
                e.dup_index = count - 1
                # If the path is in User registry (even if also in System),
                # attribute this duplicate to User — it's the redundant copy.
                if sm.is_user(key):
                    e.source = "User"
            else:
                e.state, e.symlink_target = _check_filesystem(normalized)

        entries.append(e)

    return entries
