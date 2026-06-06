from __future__ import annotations
from typing import TextIO

from ._version import __version__
from .args import Config
from .audit import EntryState, PathEntry

# ANSI escape sequences
RESET = "\x1b[0m"
BOLD = "\x1b[1m"
DIM = "\x1b[2m"
RED = "\x1b[31m"
GREEN = "\x1b[32m"
YELLOW = "\x1b[33m"
CYAN = "\x1b[36m"
MAGENTA = "\x1b[35m"

DIVIDER = "  " + "─" * 68  # 2 spaces + 68 × U+2500 (─)
SEP = "  ·  "  # U+00B7 MIDDLE DOT

MAX_PATH_COL = 50
MAX_SRC_COL = 22


def _truncate(s: str, max_cp: int) -> str:
    if len(s) <= max_cp:
        return s
    return s[: max_cp - 1] + "…"  # …


def _pad_right(s: str, width: int) -> str:
    pad = width - len(s)
    return s + " " * pad if pad > 0 else s


def _compute_widths(entries: list[PathEntry]) -> tuple[int, int]:
    col = 0
    src = 0
    for e in entries:
        if len(e.path) > col:
            col = len(e.path)
        if len(e.source) > src:
            src = len(e.source)
    return min(col, MAX_PATH_COL), min(src, MAX_SRC_COL)


def _fmt_detail(e: PathEntry) -> str:
    match e.state:
        case EntryState.DUPLICATE:
            return f"  [duplicate #{e.dup_index}]"
        case EntryState.SYMLINK:
            tgt = e.symlink_target or ""
            return f"  [→ {tgt}]"
        case EntryState.DANGLING:
            return "  [dangling symlink]"
        case EntryState.FILE:
            return "  [is a file, not a dir]"
        case EntryState.EMPTY:
            return "  [means current directory — security risk]"
        case _:
            return ""


def _render_header(out: TextIO, count: int, plain: bool, only_dead: bool) -> None:
    suffix = "  (showing dead only)" if only_dead else ""
    if plain:
        print(f"  Path Hunter  {__version__}", file=out)
        print(f"  Scanning $PATH — {count} entries{suffix}", file=out)
        print(DIVIDER, file=out)
    else:
        print(f"{CYAN}{BOLD}  Path Hunter{RESET}  {__version__}", file=out)
        print(f"  Scanning {BOLD}$PATH{RESET} — {count} entries{suffix}", file=out)
        print(f"{CYAN}{DIVIDER}{RESET}", file=out)


def _render_entry(
    out: TextIO,
    e: PathEntry,
    col_width: int,
    src_width: int,
    plain: bool,
    only_dead: bool,
) -> None:
    if only_dead and e.state in (
        EntryState.OK,
        EntryState.SYMLINK,
        EntryState.DUPLICATE,
    ):
        return

    path_col = _pad_right(_truncate(e.path, col_width), col_width)
    src_col = _pad_right(e.source, src_width)
    det = _fmt_detail(e)

    if plain:
        icon = {
            EntryState.OK: "✓",  # ✓
            EntryState.SYMLINK: "~",
            EntryState.DUPLICATE: "⚠",  # ⚠
            EntryState.EMPTY: "!",
        }.get(e.state, "✗")  # ✗ for all dead states
        print(f"  {icon}  {path_col}  {src_col}{det}", file=out)
        return

    match e.state:
        case EntryState.OK:
            print(
                f"{GREEN}{BOLD}  ✓  {RESET}"
                f"{path_col}"
                f"{CYAN}{DIM}  {src_col}{RESET}"
                f"{DIM}{det}{RESET}",
                file=out,
            )
        case EntryState.SYMLINK:
            print(
                f"{CYAN}{BOLD}  ~  {RESET}"
                f"{path_col}"
                f"{CYAN}{DIM}  {src_col}{RESET}"
                f"{CYAN}{det}{RESET}",
                file=out,
            )
        case EntryState.DUPLICATE:
            print(
                f"{YELLOW}{BOLD}  ⚠  {RESET}"
                f"{YELLOW}{path_col}{RESET}"
                f"{CYAN}{DIM}  {src_col}{RESET}"
                f"{YELLOW}{det}{RESET}",
                file=out,
            )
        case EntryState.DEAD | EntryState.FILE | EntryState.DANGLING:
            print(
                f"{RED}{BOLD}  ✗  {RESET}"
                f"{RED}{BOLD}{path_col}{RESET}"
                f"{CYAN}{DIM}  {src_col}{RESET}"
                f"{RED}{DIM}{det}{RESET}",
                file=out,
            )
        case EntryState.EMPTY:
            print(
                f"{MAGENTA}{BOLD}  !  {RESET}"
                f"{MAGENTA}{path_col}{RESET}"
                f"{CYAN}{DIM}  {src_col}{RESET}"
                f"{MAGENTA}{DIM}{det}{RESET}",
                file=out,
            )
        case _:
            pass


def _render_summary(out: TextIO, entries: list[PathEntry], plain: bool) -> None:
    n_ok = n_sym = n_dead = n_dup = n_empty = 0
    for e in entries:
        match e.state:
            case EntryState.OK:
                n_ok += 1
            case EntryState.SYMLINK:
                n_sym += 1
            case EntryState.DUPLICATE:
                n_dup += 1
            case EntryState.EMPTY:
                n_empty += 1
            case _:
                n_dead += 1

    if plain:
        print(DIVIDER, file=out)
        parts: list[str] = []
        if n_ok:
            parts.append(f"{n_ok} ok")
        parts.append(f"{n_dead} dead")  # always shown
        if n_sym:
            parts.append(f"{n_sym} symlinks")
        if n_dup:
            parts.append(f"{n_dup} duplicates")
        if n_empty:
            parts.append(f"{n_empty} empty")
        print(f"  Summary   {SEP.join(parts)}", file=out)
        return

    print(f"{CYAN}{DIVIDER}{RESET}", file=out)

    if n_dead > 0:
        parts = []
        if n_ok:
            parts.append(f"{n_ok} ok")
        parts.append(f"{RED}{BOLD}{n_dead} dead{RESET}")
        if n_sym:
            parts.append(f"{n_sym} symlinks")
        if n_dup:
            parts.append(f"{n_dup} duplicates")
        if n_empty:
            parts.append(f"{n_empty} empty")
        colored_sep = f"{DIM}{SEP}{RESET}"
        print(f"  Summary   {colored_sep.join(parts)}", file=out)
    else:
        parts = []
        if n_ok:
            parts.append(f"{n_ok} ok")
        if n_sym:
            parts.append(f"{n_sym} symlinks")
        if n_dup:
            parts.append(f"{n_dup} duplicates")
        if n_empty:
            parts.append(f"{n_empty} empty")
        print(f"  Summary   {GREEN}{BOLD}{SEP.join(parts)}{RESET}", file=out)

    print(file=out)  # trailing blank line in colored mode


def render(out: TextIO, entries: list[PathEntry], cfg: Config) -> None:
    col_width, src_width = _compute_widths(entries)
    _render_header(out, len(entries), cfg.no_color, cfg.only_dead)
    for e in entries:
        _render_entry(out, e, col_width, src_width, cfg.no_color, cfg.only_dead)
    _render_summary(out, entries, cfg.no_color)
