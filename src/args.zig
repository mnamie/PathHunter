const std = @import("std");

pub const Command = enum { audit, clean, help };

pub const Config = struct {
    command: Command = .audit,
    no_color: bool = false,
    only_dead: bool = false,
};

pub fn parseArgs(args: []const [:0]u8) Config {
    var cfg = Config{};
    for (args) |arg| {
        if (std.mem.eql(u8, arg, "clean")) {
            cfg.command = .clean;
        } else if (std.mem.eql(u8, arg, "--help") or std.mem.eql(u8, arg, "-h")) {
            cfg.command = .help;
            return cfg;
        } else if (std.mem.eql(u8, arg, "--no-color") or std.mem.eql(u8, arg, "-n")) {
            cfg.no_color = true;
        } else if (std.mem.eql(u8, arg, "--only-dead") or std.mem.eql(u8, arg, "-d")) {
            cfg.only_dead = true;
        }
    }
    return cfg;
}

pub fn printHelp(writer: anytype) !void {
    try writer.writeAll(
        \\Usage: ph [clean] [--no-color] [--help]
        \\
        \\  clean            Remove dead PATH entries (Windows only)
        \\  --only-dead, -d  Show only dead/missing entries (audit mode)
        \\  --no-color,  -n  Plain text output (no ANSI colors)
        \\  --help,      -h  Show this help
        \\
        \\
    );
}

test "defaults" {
    const cfg = parseArgs(&.{});
    try std.testing.expect(cfg.command == .audit);
    try std.testing.expect(!cfg.no_color);
    try std.testing.expect(!cfg.only_dead);
}

test "--help short-circuits" {
    var args = [_][:0]u8{ @constCast("--help"), @constCast("--no-color") };
    const cfg = parseArgs(&args);
    try std.testing.expect(cfg.command == .help);
    try std.testing.expect(!cfg.no_color);
}

test "clean --no-color" {
    var args = [_][:0]u8{ @constCast("clean"), @constCast("--no-color") };
    const cfg = parseArgs(&args);
    try std.testing.expect(cfg.command == .clean);
    try std.testing.expect(cfg.no_color);
}

test "-d" {
    var args = [_][:0]u8{@constCast("-d")};
    const cfg = parseArgs(&args);
    try std.testing.expect(cfg.only_dead);
}
