#pragma once
#include <string_view>

enum class Command { Audit, Clean, Help };

struct Config {
    Command command   = Command::Audit;
    bool    no_color  = false;
    bool    only_dead = false;
};

inline constexpr std::string_view PH_VERSION = "v0.3.0";

Config parse_args(int argc, char* const* argv);
void   print_help();
