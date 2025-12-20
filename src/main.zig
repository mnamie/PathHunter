const std = @import("std");
const builtin = @import("builtin");

const args_mod = @import("args.zig");
const source = @import("source.zig");
const audit = @import("audit.zig");
const display = @import("display.zig");
const clean = @import("clean.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const argv = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, argv);

    var cfg = args_mod.parseArgs(argv[1..]);

    // Set up buffered stdout with the 0.15 File.Writer API
    const stdout_file = std.fs.File.stdout();
    var stdout_buf: [8192]u8 = undefined;
    var fw = stdout_file.writer(&stdout_buf);
    defer fw.interface.flush() catch {};
    const writer = &fw.interface;

    // Windows: UTF-8 console + ANSI virtual terminal processing
    if (builtin.os.tag == .windows and !cfg.no_color) {
        if (!source.setupWindowsConsole()) cfg.no_color = true;
    }

    // Unix: disable colour when stdout is not a TTY
    if (builtin.os.tag != .windows and !cfg.no_color) {
        if (!std.posix.isatty(stdout_file.handle)) cfg.no_color = true;
    }

    switch (cfg.command) {
        .help => {
            try args_mod.printHelp(writer);
            return;
        },
        .clean => {
            try clean.runClean(cfg, writer, allocator);
            return;
        },
        .audit => {},
    }

    // ── Audit ──────────────────────────────────────────────────────────────

    var arena = std.heap.ArenaAllocator.init(allocator);
    defer arena.deinit();
    const aa = arena.allocator();

    const raw_path: []const u8 = blk: {
        if (builtin.os.tag != .windows) {
            if (std.posix.getenv("PATH")) |p| break :blk p;
        } else {
            if (std.process.getEnvVarOwned(aa, "PATH") catch null) |p| break :blk p;
        }
        try writer.writeAll("ph: PATH is not set or empty\n");
        fw.interface.flush() catch {};
        std.process.exit(1);
    };

    var sm = try source.build(aa);
    defer sm.deinit();

    var auditor = audit.Auditor.init(aa, &sm);
    defer auditor.deinit();

    const entries = try auditor.scan(raw_path);

    const renderer = display.Renderer.init(entries, cfg);
    try renderer.render(writer, entries);
}

// Pull in all module tests
test {
    _ = @import("args.zig");
    _ = @import("audit.zig");
    _ = @import("source.zig");
    _ = @import("display.zig");
    _ = @import("clean.zig");
}
