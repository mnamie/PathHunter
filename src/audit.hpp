#pragma once
#include <optional>
#include <string>
#include <string_view>
#include <vector>
#include "source.hpp"

enum class EntryState {
    Ok,
    Symlink,
    Duplicate,
    Dead,
    File,
    Dangling,
    Empty
};

struct PathEntry {
    std::string                path;
    EntryState                 state        = EntryState::Ok;
    int                        dup_index    = 0;
    std::optional<std::string> symlink_target;
    std::string                source;
};

bool                   path_entry_is_dead(const PathEntry& e);
std::vector<PathEntry> audit_scan(const SourceMap& sm, std::string_view raw_path);
