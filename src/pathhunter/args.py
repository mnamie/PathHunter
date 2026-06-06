from __future__ import annotations
from dataclasses import dataclass
from enum import Enum, auto
from ._version import __version__


class Command(Enum):
    AUDIT = auto()
    CLEAN = auto()
    HELP = auto()


@dataclass
class Config:
    command: Command = Command.AUDIT
    no_color: bool = False
    only_dead: bool = False


def parse_args(argv: list[str]) -> Config:
    cfg = Config()
    for arg in argv:
        match arg:
            case "clean":
                cfg.command = Command.CLEAN
            case "--help" | "-h":
                cfg.command = Command.HELP
                return cfg
            case "--no-color" | "-n":
                cfg.no_color = True
            case "--only-dead" | "-d":
                cfg.only_dead = True
            case _:
                pass
    return cfg


def print_help() -> None:
    print(
        f"Path Hunter  {__version__}\n\n"
        "Usage: ph [clean] [--only-dead] [--no-color] [--help]\n\n"
        "  clean             Remove dead entries from PATH (Windows only)\n"
        "  --only-dead, -d   Show only dead/missing entries\n"
        "  --no-color,  -n   Plain text output (no ANSI colors)\n"
        "  --help,      -h   Show this help\n"
    )
