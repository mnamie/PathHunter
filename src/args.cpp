#include "args.hpp"
#include <cstring>
#include <cstdio>

Config parse_args(int argc, char* const* argv) {
    Config cfg;
    for (int i = 0; i < argc; i++) {
        const char* a = argv[i];
        if (std::strcmp(a, "clean") == 0) {
            cfg.command = Command::Clean;
        } else if (std::strcmp(a, "--help") == 0 || std::strcmp(a, "-h") == 0) {
            cfg.command = Command::Help;
            return cfg;
        } else if (std::strcmp(a, "--no-color") == 0 || std::strcmp(a, "-n") == 0) {
            cfg.no_color = true;
        } else if (std::strcmp(a, "--only-dead") == 0 || std::strcmp(a, "-d") == 0) {
            cfg.only_dead = true;
        }
    }
    return cfg;
}

void print_help() {
    std::puts(
        "Usage: ph [clean] [--only-dead] [--no-color] [--help]\n"
        "\n"
        "  clean             Remove dead entries from PATH (Windows only)\n"
        "  --only-dead, -d   Show only dead/missing entries\n"
        "  --no-color,  -n   Plain text output (no ANSI colors)\n"
        "  --help,      -h   Show this help\n"
    );
}
