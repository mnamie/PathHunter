#include "source.hpp"
#include "strutil.hpp"
#include <cstdlib>
#include <cstring>

void SourceMap::insert(std::string key, std::string label) {
    map_.emplace(std::move(key), std::move(label));
}

std::string_view SourceMap::lookup(std::string_view key) const {
    auto it = map_.find(std::string(key));
    if (it == map_.end()) return {};
    return it->second;
}

/* ── Platform implementations ───────────────────────────────────────────── */

#ifdef _WIN32

#include <windows.h>
#include <sstream>

static std::string wcs_to_utf8(const wchar_t* ws, int wlen) {
    if (wlen < 0) wlen = (int)wcslen(ws);
    if (wlen == 0) return {};
    int n = WideCharToMultiByte(CP_UTF8, 0, ws, wlen, nullptr, 0, nullptr, nullptr);
    if (n <= 0) return {};
    std::string buf(n, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws, wlen, buf.data(), n, nullptr, nullptr);
    return buf;
}

static std::wstring utf8_to_wcs(std::string_view s) {
    int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), nullptr, 0);
    if (n <= 0) return {};
    std::wstring buf(n, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), buf.data(), n);
    return buf;
}

static std::string expand_env_vars_win(std::string_view utf8) {
    std::wstring wsrc = utf8_to_wcs(utf8);
    if (wsrc.empty()) return std::string(utf8);
    wchar_t wdst[32768];
    DWORD written = ExpandEnvironmentStringsW(wsrc.c_str(), wdst, 32768);
    if (written == 0 || written > 32768) return std::string(utf8);
    return wcs_to_utf8(wdst, (int)(written - 1));
}

static void add_registry_path(
    SourceMap&      sm,
    HKEY            root,
    const wchar_t*  subkey,
    std::string_view label)
{
    HKEY hkey;
    if (RegOpenKeyExW(root, subkey, 0, KEY_READ, &hkey) != ERROR_SUCCESS)
        return;

    DWORD data_size = 0, data_type = 0;
    if (RegQueryValueExW(hkey, L"Path", nullptr, &data_type, nullptr, &data_size) != ERROR_SUCCESS) {
        RegCloseKey(hkey);
        return;
    }

    DWORD num_wchars = data_size / 2 + 1;
    std::wstring wbuf(num_wchars, L'\0');
    RegQueryValueExW(hkey, L"Path", nullptr, &data_type, (LPBYTE)wbuf.data(), &data_size);
    RegCloseKey(hkey);

    int wlen = 0;
    while (wlen < (int)num_wchars && wbuf[wlen]) wlen++;
    std::string raw = wcs_to_utf8(wbuf.data(), wlen);

    std::istringstream ss(raw);
    std::string tok;
    while (std::getline(ss, tok, ';')) {
        std::string_view sv = str_ltrim(tok);
        sv = str_rtrim(sv);
        if (sv.empty()) continue;

        std::string expanded = expand_env_vars_win(sv);
        if (expanded.empty()) continue;

        std::size_t normed = strip_trailing_slash(expanded);
        if (normed == 0) continue;

        std::string key = ascii_lower(std::string_view(expanded).substr(0, normed));
        sm.insert(std::move(key), std::string(label));
    }
}

static void build_windows(SourceMap& sm) {
    add_registry_path(sm, HKEY_LOCAL_MACHINE,
        L"SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "System");
    add_registry_path(sm, HKEY_CURRENT_USER, L"Environment", "User");
}

/* ── Registry write helpers (used by clean.cpp) ─────────────────────────── */

std::vector<std::string> read_raw_user_path_segments() {
    std::vector<std::string> result;

    HKEY hkey;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Environment", 0, KEY_READ, &hkey) != ERROR_SUCCESS)
        return result;

    DWORD data_size = 0, data_type = 0;
    if (RegQueryValueExW(hkey, L"Path", nullptr, &data_type, nullptr, &data_size) != ERROR_SUCCESS) {
        RegCloseKey(hkey);
        return result;
    }

    DWORD num_wchars = data_size / 2 + 1;
    std::wstring wbuf(num_wchars, L'\0');
    RegQueryValueExW(hkey, L"Path", nullptr, &data_type, (LPBYTE)wbuf.data(), &data_size);
    RegCloseKey(hkey);

    int wlen = 0;
    while (wlen < (int)num_wchars && wbuf[wlen]) wlen++;
    std::string raw = wcs_to_utf8(wbuf.data(), wlen);

    std::istringstream ss(raw);
    std::string tok;
    while (std::getline(ss, tok, ';')) {
        std::string_view sv = str_ltrim(tok);
        sv = str_rtrim(sv);
        if (!sv.empty())
            result.emplace_back(sv);
    }
    return result;
}

bool write_user_path_to_registry(const std::vector<std::string>& segs) {
    std::string joined;
    for (std::size_t i = 0; i < segs.size(); i++) {
        if (i > 0) joined += ';';
        joined += segs[i];
    }

    std::wstring wval = utf8_to_wcs(joined);
    if (wval.empty()) return false;

    HKEY hkey;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Environment", 0,
                      KEY_READ | KEY_SET_VALUE, &hkey) != ERROR_SUCCESS)
        return false;

    DWORD byte_len = (DWORD)((wval.size() + 1) * sizeof(wchar_t));
    bool ok = RegSetValueExW(hkey, L"Path", 0, REG_EXPAND_SZ,
                             (const BYTE*)wval.c_str(), byte_len) == ERROR_SUCCESS;
    RegCloseKey(hkey);
    return ok;
}

void broadcast_env_change() {
    DWORD_PTR result = 0;
    SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0,
                        (LPARAM)L"Environment",
                        SMTO_ABORTIFHUNG, 5000, &result);
}

bool setup_windows_console() {
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
    HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);
    if (h == INVALID_HANDLE_VALUE) return false;
    DWORD mode = 0;
    if (!GetConsoleMode(h, &mode)) return false;
    SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    return true;
}

#else /* POSIX ────────────────────────────────────────────────────────────── */

#include <fstream>

static std::string expand_vars_unix(std::string_view s, std::string_view home) {
    if (s == "~") return std::string(home);
    if (s.size() >= 2 && s[0] == '~' && s[1] == '/')
        return std::string(home) + std::string(s.substr(1));

    std::string result;
    result.reserve(s.size() + home.size());
    for (std::size_t i = 0; i < s.size(); ) {
        if (s[i] == '$') {
            bool braced = (i + 1 < s.size() && s[i + 1] == '{');
            std::size_t name_start = i + 1 + (braced ? 1u : 0u);
            if (name_start + 4 <= s.size() && s.substr(name_start, 4) == "HOME") {
                std::size_t after = name_start + 4;
                if (!braced || (after < s.size() && s[after] == '}')) {
                    result += home;
                    i = after + (braced ? 1u : 0u);
                    continue;
                }
            }
        }
        result += s[i++];
    }
    return result;
}

static void parse_path_line(
    SourceMap&       sm,
    std::string_view line,
    std::string_view label,
    std::string_view home)
{
    line = str_ltrim(line);
    line = str_rtrim(line);
    if (line.empty() || line[0] == '#') return;

    std::string_view value;
    if (line.size() > 12 && line.substr(0, 12) == "export PATH=") {
        value = line.substr(12);
    } else if (line.size() > 5 && line.substr(0, 5) == "PATH=") {
        value = line.substr(5);
    } else {
        return;
    }

    if (value.size() >= 2 &&
        ((value.front() == '"'  && value.back() == '"') ||
         (value.front() == '\'' && value.back() == '\''))) {
        value = value.substr(1, value.size() - 2);
    }

    for (std::size_t i = 0; i + 1 < value.size(); i++) {
        if (value[i] == ' ' && value[i + 1] == '#') { value = value.substr(0, i); break; }
    }

    std::size_t start = 0;
    for (std::size_t i = 0; i <= value.size(); i++) {
        if (i == value.size() || value[i] == ':') {
            std::string_view seg = value.substr(start, i - start);
            start = i + 1;

            seg = str_ltrim(seg);
            seg = str_rtrim(seg);
            if (seg.empty()) continue;

            if (seg == "$PATH" || seg == "${PATH}" || seg == "$path") continue;

            std::string expanded = expand_vars_unix(seg, home);
            if (expanded.empty()) continue;

            if (expanded.find('$') != std::string::npos) continue;

            std::size_t normed = strip_trailing_slash(expanded);
            if (normed == 0) continue;

            sm.insert(expanded.substr(0, normed), std::string(label));
        }
    }
}

static void parse_shell_config(
    SourceMap&       sm,
    std::string_view path,
    std::string_view label,
    std::string_view home)
{
    std::string path_str(path);
    std::ifstream f(path_str);
    if (!f) return;
    std::string line;
    while (std::getline(f, line))
        parse_path_line(sm, line, label, home);
}

static void build_unix(SourceMap& sm) {
    const char* home_raw = std::getenv("HOME");
    if (!home_raw) home_raw = "";
    std::string_view home_sv(home_raw);
    std::string home(home_sv.substr(0, strip_trailing_slash(home_sv)));

    static const struct { const char* path; const char* label; } etc_files[] = {
        { "/etc/environment",                "/etc/environment" },
        { "/etc/profile.d/apps-bin-path.sh", "/etc/profile.d/" },
    };
    for (const auto& f : etc_files)
        parse_shell_config(sm, f.path, f.label, home);

    static const struct { const char* suffix; const char* label; } rel_files[] = {
        { "/.profile",      "~/.profile"      },
        { "/.bash_profile", "~/.bash_profile" },
        { "/.bash_login",   "~/.bash_login"   },
        { "/.bashrc",       "~/.bashrc"        },
        { "/.zprofile",     "~/.zprofile"      },
        { "/.zshrc",        "~/.zshrc"         },
        { "/.cargo/env",    "~/.cargo/env"     },
        { "/.nimble/env",   "~/.nimble/env"    },
    };
    for (const auto& f : rel_files) {
        std::string full = home + f.suffix;
        parse_shell_config(sm, full, f.label, home);
    }
}

#endif /* _WIN32 */

void SourceMap::build() {
#ifdef _WIN32
    build_windows(*this);
#else
    build_unix(*this);
#endif
}
