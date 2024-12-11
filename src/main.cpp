#include "args.hpp"
#include "source.hpp"
#include "audit.hpp"
#include "display.hpp"
#include "clean.hpp"
#include <cstdlib>
#include <iostream>

#ifdef _WIN32
#  include <io.h>
#  define IS_TTY() (_isatty(1) != 0)
#else
#  include <unistd.h>
#  define IS_TTY() (isatty(STDOUT_FILENO) != 0)
#endif

int main(int argc, char* argv[]) try {
    Config cfg = parse_args(argc - 1, argv + 1);

    if (!cfg.no_color) {
#ifdef _WIN32
        if (!setup_windows_console()) cfg.no_color = true;
#else
        if (!IS_TTY()) cfg.no_color = true;
#endif
    }

    if (cfg.command == Command::Help)  { print_help(); return 0; }
    if (cfg.command == Command::Clean) return run_clean(std::cout, cfg);

    const char* raw_path = std::getenv("PATH");
    if (!raw_path || !*raw_path) {
        std::cerr << "ph: PATH is not set or empty\n";
        return 1;
    }

    SourceMap sm;
    sm.build();

    auto entries = audit_scan(sm, raw_path);
    render(std::cout, entries, cfg);
    return 0;
} catch (const std::bad_alloc&) {
    std::fputs("ph: out of memory\n", stderr);
    return 1;
}
