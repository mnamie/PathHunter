#include "display.hpp"
#include "strutil.hpp"
#include <cstdio>
#include <cstring>

#define RESET   "\x1b[0m"
#define BOLD    "\x1b[1m"
#define DIM     "\x1b[2m"
#define RED     "\x1b[31m"
#define GREEN   "\x1b[32m"
#define YELLOW  "\x1b[33m"
#define CYAN    "\x1b[36m"
#define MAGENTA "\x1b[35m"

/* 2 spaces + 68 × U+2500 (─) */
#define DIVIDER \
    "  \xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80\xe2\x94\x80" \
    "\xe2\x94\x80\xe2\x94\x80"

#define SEP "  \xc2\xb7  "

#define MAX_PATH_COL 50
#define MAX_SRC_COL  22

/* ── String layout helpers ───────────────────────────────────────────────── */

static std::size_t truncate_str(
    char*       buf,
    std::size_t buf_cap,
    const char* s,
    std::size_t slen,
    std::size_t max_cp)
{
    if (utf8_cp_count(std::string_view(s, slen)) <= max_cp) {
        std::size_t n = slen < buf_cap ? slen : buf_cap - 1;
        std::memcpy(buf, s, n);
        return n;
    }
    std::size_t count = 0, pos = 0;
    while (pos < slen && count < max_cp - 1) {
        if (((unsigned char)s[pos] & 0xC0u) != 0x80u) count++;
        pos++;
    }
    while (pos < slen && ((unsigned char)s[pos] & 0xC0u) == 0x80u) pos++;

    std::size_t n = pos < buf_cap - 3 ? pos : buf_cap - 4;
    std::memcpy(buf, s, n);
    if (n + 3 < buf_cap) {
        buf[n]     = (char)0xE2;
        buf[n + 1] = (char)0x80;
        buf[n + 2] = (char)0xA6;
        n += 3;
    }
    return n;
}

static std::size_t pad_right(
    char*       buf,
    std::size_t buf_cap,
    const char* s,
    std::size_t slen,
    std::size_t width)
{
    std::size_t n = slen < buf_cap ? slen : buf_cap - 1;
    std::memcpy(buf, s, n);
    std::size_t cp     = utf8_cp_count(std::string_view(s, n));
    std::size_t spaces = width > cp ? width - cp : 0;
    std::size_t pos    = n;
    while (spaces-- > 0 && pos + 1 < buf_cap)
        buf[pos++] = ' ';
    return pos;
}

/* ── Column widths ───────────────────────────────────────────────────────── */

static std::pair<std::size_t, std::size_t>
compute_widths(const std::vector<PathEntry>& entries) {
    std::size_t col_width = 0, src_width = 0;
    for (const auto& e : entries) {
        std::size_t c = utf8_cp_count(e.path);
        std::size_t s = utf8_cp_count(e.source);
        if (c > col_width) col_width = c;
        if (s > src_width) src_width = s;
    }
    if (col_width > MAX_PATH_COL) col_width = MAX_PATH_COL;
    if (src_width > MAX_SRC_COL)  src_width = MAX_SRC_COL;
    return {col_width, src_width};
}

/* ── Detail string ───────────────────────────────────────────────────────── */

static std::string fmt_detail(const PathEntry& e) {
    char buf[576];
    switch (e.state) {
    case EntryState::Duplicate:
        std::snprintf(buf, sizeof buf, "  [duplicate #%d]", e.dup_index);
        return buf;
    case EntryState::Symlink: {
        const char* tgt = e.symlink_target.has_value() ? e.symlink_target->c_str() : "";
        std::snprintf(buf, sizeof buf, "  [\xe2\x86\x92 %s]", tgt);
        return buf;
    }
    case EntryState::Dangling:
        return "  [dangling symlink]";
    case EntryState::File:
        return "  [is a file, not a dir]";
    case EntryState::Empty:
        return "  [means current directory \xe2\x80\x94 security risk]";
    default:
        return {};
    }
}

/* ── Header ──────────────────────────────────────────────────────────────── */

static void render_header(std::ostream& out, std::size_t count, bool plain, bool only_dead) {
    const char* suffix = only_dead ? "  (showing dead only)" : "";
    if (plain) {
        out << "  Path Hunter  " << PH_VERSION << '\n';
        out << "  Scanning $PATH \xe2\x80\x94 " << count << " entries" << suffix << '\n';
        out << DIVIDER "\n";
    } else {
        out << CYAN BOLD "  Path Hunter" RESET "  " << PH_VERSION << '\n';
        out << "  Scanning " BOLD "$PATH" RESET " \xe2\x80\x94 " << count << " entries" << suffix << '\n';
        out << CYAN DIVIDER RESET "\n";
    }
}

/* ── Single entry ────────────────────────────────────────────────────────── */

static void render_entry(
    std::ostream&    out,
    const PathEntry& e,
    std::size_t      col_width,
    std::size_t      src_width,
    bool             plain,
    bool             only_dead)
{
    if (only_dead) {
        switch (e.state) {
        case EntryState::Ok:
        case EntryState::Symlink:
        case EntryState::Duplicate:
            return;
        default:
            break;
        }
    }

    char path_buf[704];
    char trun_buf[704];
    char src_buf[128];

    std::size_t tlen = truncate_str(trun_buf, sizeof trun_buf,
                                    e.path.c_str(), e.path.size(), col_width);
    std::size_t plen = pad_right(path_buf, sizeof path_buf, trun_buf, tlen, col_width);
    path_buf[plen] = '\0';

    std::size_t padded = pad_right(src_buf, sizeof src_buf,
                                   e.source.c_str(), e.source.size(), src_width);
    src_buf[padded] = '\0';

    std::string det = fmt_detail(e);

    if (plain) {
        const char* icon;
        switch (e.state) {
        case EntryState::Ok:        icon = "\xe2\x9c\x93"; break;
        case EntryState::Symlink:   icon = "~";             break;
        case EntryState::Duplicate: icon = "\xe2\x9a\xa0"; break;
        case EntryState::Empty:     icon = "!";             break;
        default:                    icon = "\xe2\x9c\x97"; break;
        }
        out << "  " << icon << "  " << path_buf << "  " << src_buf << det << '\n';
        return;
    }

    switch (e.state) {
    case EntryState::Ok:
        out << GREEN BOLD "  \xe2\x9c\x93  " RESET
            << path_buf << CYAN DIM "  " << src_buf << RESET DIM << det << RESET "\n";
        break;
    case EntryState::Symlink:
        out << CYAN BOLD "  ~  " RESET
            << path_buf << CYAN DIM "  " << src_buf << RESET CYAN << det << RESET "\n";
        break;
    case EntryState::Duplicate:
        out << YELLOW BOLD "  \xe2\x9a\xa0  " RESET YELLOW
            << path_buf << RESET CYAN DIM "  " << src_buf << RESET YELLOW << det << RESET "\n";
        break;
    case EntryState::Dead:
    case EntryState::File:
    case EntryState::Dangling:
        out << RED BOLD "  \xe2\x9c\x97  " RESET RED BOLD
            << path_buf << RESET CYAN DIM "  " << src_buf << RESET RED DIM << det << RESET "\n";
        break;
    case EntryState::Empty:
        out << MAGENTA BOLD "  !  " RESET MAGENTA
            << path_buf << RESET CYAN DIM "  " << src_buf << RESET MAGENTA DIM << det << RESET "\n";
        break;
    }
}

/* ── Summary ─────────────────────────────────────────────────────────────── */

static void render_summary(
    std::ostream&                out,
    const std::vector<PathEntry>& entries,
    bool                         plain)
{
    std::size_t n_ok = 0, n_sym = 0, n_dead = 0, n_dup = 0, n_empty = 0;
    for (const auto& e : entries) {
        switch (e.state) {
        case EntryState::Ok:        n_ok++;    break;
        case EntryState::Symlink:   n_sym++;   break;
        case EntryState::Duplicate: n_dup++;   break;
        case EntryState::Empty:     n_empty++; break;
        default:                    n_dead++;  break;
        }
    }

    if (plain) {
        out << DIVIDER "\n  Summary   ";
        bool first = true;
        if (n_ok)    { out << n_ok << " ok"; first = false; }
        if (!first)   out << SEP;
        out << n_dead << " dead";
        if (n_sym)   out << SEP << n_sym   << " symlinks";
        if (n_dup)   out << SEP << n_dup   << " duplicates";
        if (n_empty) out << SEP << n_empty << " empty";
        out << '\n';
        return;
    }

    out << CYAN DIVIDER RESET "\n  Summary   ";

    if (n_dead > 0) {
        if (n_ok)    out << n_ok << " ok" DIM SEP RESET;
        out << RED BOLD << n_dead << " dead" RESET;
        if (n_sym)   out << DIM SEP RESET << n_sym   << " symlinks";
        if (n_dup)   out << DIM SEP RESET << n_dup   << " duplicates";
        if (n_empty) out << DIM SEP RESET << n_empty << " empty";
    } else {
        out << GREEN BOLD;
        bool first = true;
        if (n_ok)    { out << n_ok << " ok";                         first = false; }
        if (n_sym)   { if (!first) out << SEP; out << n_sym   << " symlinks";   first = false; }
        if (n_dup)   { if (!first) out << SEP; out << n_dup   << " duplicates"; first = false; }
        if (n_empty) { if (!first) out << SEP; out << n_empty << " empty"; }
        out << RESET;
    }
    out << "\n\n";
}

/* ── Public entry point ──────────────────────────────────────────────────── */

void render(std::ostream& out, const std::vector<PathEntry>& entries, const Config& cfg) {
    auto [col_width, src_width] = compute_widths(entries);
    render_header(out, entries.size(), cfg.no_color, cfg.only_dead);
    for (const auto& e : entries)
        render_entry(out, e, col_width, src_width, cfg.no_color, cfg.only_dead);
    render_summary(out, entries, cfg.no_color);
}
