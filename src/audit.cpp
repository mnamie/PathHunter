#include "audit.hpp"
#include "strutil.hpp"
#include <unordered_map>
#include <utility>

bool path_entry_is_dead(const PathEntry& e) {
    return e.state == EntryState::Dead ||
           e.state == EntryState::Dangling ||
           e.state == EntryState::File;
}

/* ── Filesystem classification ───────────────────────────────────────────── */

#ifdef _WIN32

#include <windows.h>

static std::string wcs_to_utf8_audit(const wchar_t* ws, int wlen) {
    int n = WideCharToMultiByte(CP_UTF8, 0, ws, wlen, nullptr, 0, nullptr, nullptr);
    if (n <= 0) return {};
    std::string buf(n, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws, wlen, buf.data(), n, nullptr, nullptr);
    return buf;
}

static std::pair<EntryState, std::optional<std::string>>
check_filesystem(const std::string& path) {
    int wn = MultiByteToWideChar(CP_UTF8, 0, path.c_str(), (int)path.size(), nullptr, 0);
    std::wstring wpath(wn, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, path.c_str(), (int)path.size(), wpath.data(), wn);

    DWORD attrs = GetFileAttributesW(wpath.c_str());
    if (attrs == INVALID_FILE_ATTRIBUTES) {
        HANDLE h = CreateFileW(wpath.c_str(), 0,
                               FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                               nullptr, OPEN_EXISTING,
                               FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
                               nullptr);
        if (h != INVALID_HANDLE_VALUE) { CloseHandle(h); return {EntryState::Dangling, std::nullopt}; }
        return {EntryState::Dead, std::nullopt};
    }

    if (attrs & FILE_ATTRIBUTE_REPARSE_POINT) {
        HANDLE h = CreateFileW(wpath.c_str(), 0,
                               FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                               nullptr, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, nullptr);
        if (h == INVALID_HANDLE_VALUE) return {EntryState::Dangling, std::nullopt};

        wchar_t resolved[4096];
        DWORD n = GetFinalPathNameByHandleW(h, resolved, 4096, 0);
        CloseHandle(h);
        if (n == 0) return {EntryState::Dangling, std::nullopt};

        const wchar_t* start = resolved;
        if (n > 4 && resolved[0] == L'\\' && resolved[1] == L'\\' &&
            resolved[2] == L'?' && resolved[3] == L'\\') {
            start += 4; n -= 4;
        }
        return {EntryState::Symlink, std::make_optional(wcs_to_utf8_audit(start, (int)n))};
    }

    if (attrs & FILE_ATTRIBUTE_DIRECTORY) return {EntryState::Ok, std::nullopt};
    return {EntryState::File, std::nullopt};
}

#else

#include <filesystem>
namespace fs = std::filesystem;

static std::pair<EntryState, std::optional<std::string>>
check_filesystem(const std::string& path) {
    std::error_code ec;
    auto lst = fs::symlink_status(path, ec);
    if (ec) return {EntryState::Dead, std::nullopt};

    if (lst.type() == fs::file_type::symlink) {
        auto st = fs::status(path, ec);
        if (ec || !fs::is_directory(st)) return {EntryState::Dangling, std::nullopt};

        auto target = fs::read_symlink(path, ec);
        if (ec) return {EntryState::Symlink, std::make_optional<std::string>("(resolved)")};
        return {EntryState::Symlink, std::make_optional(target.string())};
    }

    if (fs::is_directory(lst)) return {EntryState::Ok, std::nullopt};
    if (fs::is_regular_file(lst)) return {EntryState::File, std::nullopt};
    return {EntryState::Dead, std::nullopt};
}

#endif

/* ── audit_scan ──────────────────────────────────────────────────────────── */

std::vector<PathEntry> audit_scan(const SourceMap& sm, std::string_view raw_path) {
#ifdef _WIN32
    const char sep = ';';
#else
    const char sep = ':';
#endif

    std::vector<PathEntry> entries;
    std::unordered_map<std::string, int> seen;

    std::size_t start = 0;
    const std::size_t total = raw_path.size();

    while (true) {
        std::size_t end = raw_path.find(sep, start);
        std::size_t seglen = (end == std::string_view::npos) ? total - start : end - start;
        std::string_view seg = raw_path.substr(start, seglen);

        PathEntry e;
        if (seg.empty()) {
            e.path   = "(empty entry)";
            e.state  = EntryState::Empty;
        } else {
            e.path = std::string(seg);
            std::size_t normed = strip_trailing_slash(seg);
            std::string normalized = e.path.substr(0, normed);

#ifdef _WIN32
            std::string key = ascii_lower(normalized);
#else
            std::string key = normalized;
#endif
            e.source = std::string(sm.lookup(key));

            int count = ++seen[key];
            if (count > 1) {
                e.state     = EntryState::Duplicate;
                e.dup_index = count - 1;
            } else {
                auto [state, target] = check_filesystem(normalized);
                e.state          = state;
                e.symlink_target = std::move(target);
            }
        }

        entries.push_back(std::move(e));
        if (end == std::string_view::npos) break;
        start = end + 1;
    }

    return entries;
}
