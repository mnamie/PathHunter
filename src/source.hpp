#pragma once
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

class SourceMap {
public:
    void             build();
    void             insert(std::string key, std::string label);
    std::string_view lookup(std::string_view key) const;

private:
    std::unordered_map<std::string, std::string> map_;
};

#ifdef _WIN32
std::vector<std::string> read_raw_user_path_segments();
bool write_user_path_to_registry(const std::vector<std::string>& segs);
void broadcast_env_change();
bool setup_windows_console();
#endif
