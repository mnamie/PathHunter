#include "clean.hpp"
#include "audit.hpp"
#include "source.hpp"
#include "strutil.hpp"
#include <algorithm>
#include <cstdlib>

#ifndef _WIN32

int run_clean(std::ostream& out, const Config& cfg) {
    (void)cfg;
    out << "ph: clean is only supported on Windows\n";
    return 1;
}

#else /* Windows ────────────────────────────────────────────────────────── */

#include <iostream>

int run_clean(std::ostream& out, const Config& cfg) {
    const char* cross = cfg.no_color ? "x" : "\xe2\x9c\x97";
    const char* warn  = cfg.no_color ? "!" : "\xe2\x9a\xa0";
    const char* dash  = cfg.no_color ? "-" : "\xe2\x80\x94";

    const char* raw_path = std::getenv("PATH");
    if (!raw_path || !*raw_path) {
        out << "ph: PATH is not set or empty\n";
        return 1;
    }

    SourceMap sm;
    sm.build();

    auto entries = audit_scan(sm, raw_path);

    std::size_t n_user = 0, n_system = 0, n_unknown = 0;
    for (const auto& e : entries) {
        if (!path_entry_is_dead(e)) continue;
        if      (e.source == "User")   n_user++;
        else if (e.source == "System") n_system++;
        else                            n_unknown++;
    }

    std::size_t n_dead = n_user + n_system + n_unknown;
    if (n_dead == 0) {
        out << "No dead entries found.\n";
        return 0;
    }

    out << "Dead entries to remove:\n\n";

    std::vector<std::string> dead_keys;

    if (n_user > 0) {
        out << "  [User]\n";
        for (const auto& e : entries) {
            if (!path_entry_is_dead(e)) continue;
            if (e.source != "User") continue;
            out << "    " << cross << "  " << e.path << '\n';
            dead_keys.push_back(ascii_lower(e.path));
        }
        out << '\n';
    }

    if (n_system > 0) {
        out << "  [System]  " << warn << " requires administrator " << dash << " will be skipped\n";
        for (const auto& e : entries) {
            if (!path_entry_is_dead(e)) continue;
            if (e.source != "System") continue;
            out << "    " << cross << "  " << e.path << '\n';
        }
        out << '\n';
    }

    if (n_unknown > 0) {
        out << "  [Unknown source " << dash << " skipped]\n";
        for (const auto& e : entries) {
            if (!path_entry_is_dead(e)) continue;
            if (e.source == "User" || e.source == "System") continue;
            out << "    " << cross << "  " << e.path << '\n';
        }
        out << '\n';
    }

    if (n_user == 0) {
        out << "No User PATH entries to remove (System entries require administrator).\n";
        return 0;
    }

    out << "Remove " << n_user << " User PATH entr"
        << (n_user == 1 ? "y" : "ies") << "? [y/N] " << std::flush;

    std::string answer;
    std::getline(std::cin, answer);

    if (answer != "y" && answer != "Y") {
        out << "No changes made.\n";
        return 0;
    }

    auto raw_segs = read_raw_user_path_segments();
    std::vector<std::string> surviving;
    for (const auto& seg : raw_segs) {
        std::size_t normed = strip_trailing_slash(seg);
        std::string key = ascii_lower(seg.substr(0, normed));
        if (std::find(dead_keys.begin(), dead_keys.end(), key) == dead_keys.end())
            surviving.push_back(seg);
    }

    write_user_path_to_registry(surviving);
    broadcast_env_change();

    out << "Removed " << n_user << " entr"
        << (n_user == 1 ? "y" : "ies") << ". User PATH updated.\n";
    return 0;
}

#endif /* _WIN32 */
