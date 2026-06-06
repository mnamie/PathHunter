from __future__ import annotations
import os
import sys


class SourceMap:
    def __init__(self) -> None:
        self._map: dict[str, str] = {}
        self._user: set[str] = set()

    def insert(self, key: str, label: str) -> None:
        self._map.setdefault(key, label)  # first-seen wins for primary label
        if label == "User":
            self._user.add(key)

    def lookup(self, key: str) -> str:
        return self._map.get(key, "")

    def is_user(self, key: str) -> bool:
        return key in self._user

    def build(self) -> None:
        if sys.platform == "win32":
            _build_windows(self)
        else:
            _build_unix(self)


# ── Unix ──────────────────────────────────────────────────────────────────────

_ETC_FILES = [
    ("/etc/environment", "/etc/environment"),
    ("/etc/set-environment", "/etc/set-environment"),
    ("/etc/profile.d/apps-bin-path.sh", "/etc/profile.d/"),
]

_REL_FILES = [
    ("/.profile", "~/.profile"),
    ("/.bash_profile", "~/.bash_profile"),
    ("/.bash_login", "~/.bash_login"),
    ("/.bashrc", "~/.bashrc"),
    ("/.zprofile", "~/.zprofile"),
    ("/.zshrc", "~/.zshrc"),
    ("/.cargo/env", "~/.cargo/env"),
    ("/.nimble/env", "~/.nimble/env"),
]


def _expand_vars_unix(s: str, home: str) -> str:
    """Expand only ~ and $HOME / ${HOME}. Intentionally minimal — matches C++ behavior."""
    if s == "~":
        return home
    if s.startswith("~/"):
        return home + s[1:]
    result = []
    i = 0
    while i < len(s):
        if s[i] == "$":
            braced = i + 1 < len(s) and s[i + 1] == "{"
            name_start = i + 1 + (1 if braced else 0)
            if s[name_start : name_start + 4] == "HOME":
                after = name_start + 4
                if not braced or (after < len(s) and s[after] == "}"):
                    result.append(home)
                    i = after + (1 if braced else 0)
                    continue
        result.append(s[i])
        i += 1
    return "".join(result)


def _parse_path_line(sm: SourceMap, line: str, label: str, home: str) -> None:
    line = line.strip()
    if not line or line[0] == "#":
        return

    if line.startswith("export PATH="):
        value = line[12:]
    elif line.startswith("PATH="):
        value = line[5:]
    else:
        return

    # Strip surrounding quotes
    if len(value) >= 2 and value[0] in ('"', "'") and value[-1] == value[0]:
        value = value[1:-1]

    # Strip inline comments (must be preceded by a space)
    space_hash = value.find(" #")
    if space_hash != -1:
        value = value[:space_hash]

    for seg in value.split(":"):
        seg = seg.strip()
        if not seg:
            continue
        if seg in ("$PATH", "${PATH}", "$path"):
            continue
        expanded = _expand_vars_unix(seg, home)
        if not expanded:
            continue
        # Expand remaining $VAR / ${VAR} (e.g. $USER, ${XDG_STATE_HOME} on NixOS)
        # using the live environment. Anything still unresolved is filtered below.
        expanded = os.path.expandvars(expanded)
        if "$" in expanded:
            continue
        normalized = expanded.rstrip("/") or expanded
        sm.insert(normalized, label)


def _parse_shell_config(sm: SourceMap, path: str, label: str, home: str) -> None:
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                _parse_path_line(sm, line.rstrip("\n"), label, home)
    except OSError:
        pass


def _build_unix(sm: SourceMap) -> None:
    home_raw = os.environ.get("HOME", "")
    home = home_raw.rstrip("/")

    for path, label in _ETC_FILES:
        _parse_shell_config(sm, path, label, home)

    for suffix, label in _REL_FILES:
        _parse_shell_config(sm, home + suffix, label, home)


# ── Windows ───────────────────────────────────────────────────────────────────

if sys.platform == "win32":
    import winreg as _winreg
    import ctypes as _ctypes

    def _read_registry_path(sm: SourceMap, root: int, subkey: str, label: str) -> None:
        try:
            with _winreg.OpenKey(root, subkey, 0, _winreg.KEY_READ) as key:
                raw, _ = _winreg.QueryValueEx(key, "Path")
        except OSError:
            return
        for seg in raw.split(";"):
            seg = seg.strip()
            if not seg:
                continue
            expanded = os.path.expandvars(seg)
            if not expanded:
                continue
            normalized = expanded.rstrip("/\\")
            if not normalized:
                continue
            sm.insert(normalized.lower(), label)

    def _build_windows(sm: SourceMap) -> None:
        _read_registry_path(
            sm,
            _winreg.HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            "System",
        )
        _read_registry_path(sm, _winreg.HKEY_CURRENT_USER, "Environment", "User")

    def read_raw_user_path_segments() -> list[str]:
        try:
            with _winreg.OpenKey(
                _winreg.HKEY_CURRENT_USER, "Environment", 0, _winreg.KEY_READ
            ) as key:
                raw, _ = _winreg.QueryValueEx(key, "Path")
        except OSError:
            return []
        return [s.strip() for s in raw.split(";") if s.strip()]

    def write_user_path_to_registry(segs: list[str]) -> bool:
        joined = ";".join(segs)
        try:
            with _winreg.OpenKey(
                _winreg.HKEY_CURRENT_USER,
                "Environment",
                0,
                _winreg.KEY_READ | _winreg.KEY_SET_VALUE,
            ) as key:
                _winreg.SetValueEx(key, "Path", 0, _winreg.REG_EXPAND_SZ, joined)
            return True
        except OSError:
            return False

    _SYSTEM_ENV_KEY = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

    def is_admin() -> bool:
        try:
            return bool(_ctypes.windll.shell32.IsUserAnAdmin())
        except OSError:
            return False

    def read_raw_system_path_segments() -> list[str]:
        try:
            with _winreg.OpenKey(
                _winreg.HKEY_LOCAL_MACHINE, _SYSTEM_ENV_KEY, 0, _winreg.KEY_READ
            ) as key:
                raw, _ = _winreg.QueryValueEx(key, "Path")
        except OSError:
            return []
        return [s.strip() for s in raw.split(";") if s.strip()]

    def write_system_path_to_registry(segs: list[str]) -> bool:
        joined = ";".join(segs)
        try:
            with _winreg.OpenKey(
                _winreg.HKEY_LOCAL_MACHINE,
                _SYSTEM_ENV_KEY,
                0,
                _winreg.KEY_READ | _winreg.KEY_SET_VALUE,
            ) as key:
                _winreg.SetValueEx(key, "Path", 0, _winreg.REG_EXPAND_SZ, joined)
            return True
        except OSError:
            return False

    def broadcast_env_change() -> None:
        HWND_BROADCAST = 0xFFFF
        WM_SETTINGCHANGE = 0x001A
        SMTO_ABORTIFHUNG = 0x0002
        result = _ctypes.c_ulong(0)
        _ctypes.windll.user32.SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            "Environment",
            SMTO_ABORTIFHUNG,
            5000,
            _ctypes.byref(result),
        )

    def setup_windows_console() -> bool:
        kernel32 = _ctypes.windll.kernel32
        kernel32.SetConsoleOutputCP(65001)
        kernel32.SetConsoleCP(65001)
        ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004
        STD_OUTPUT_HANDLE = -11
        handle = kernel32.GetStdHandle(STD_OUTPUT_HANDLE)
        if handle == -1:
            return False
        mode = _ctypes.c_ulong(0)
        if not kernel32.GetConsoleMode(handle, _ctypes.byref(mode)):
            return False
        kernel32.SetConsoleMode(handle, mode.value | ENABLE_VIRTUAL_TERMINAL_PROCESSING)
        return True

else:
    # Stubs so clean.py can import these names on non-Windows without branching
    def is_admin() -> bool:
        return False

    def read_raw_user_path_segments() -> list[str]:
        return []

    def write_user_path_to_registry(segs: list[str]) -> bool:
        return False

    def read_raw_system_path_segments() -> list[str]:
        return []

    def write_system_path_to_registry(segs: list[str]) -> bool:
        return False

    def broadcast_env_change() -> None:
        pass

    def setup_windows_console() -> bool:
        return False
