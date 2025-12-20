const std = @import("std");
const builtin = @import("builtin");
const args = @import("args.zig");
const audit = @import("audit.zig");
const source = @import("source.zig");

pub fn runClean(cfg: args.Config, writer: anytype, allocator: std.mem.Allocator) !void {
    if (builtin.os.tag != .windows) {
        try writer.writeAll("ph: clean is only supported on Windows\n");
        return error.UnsupportedPlatform;
    }
    try runCleanWindows(cfg, writer, allocator);
}

fn runCleanWindows(cfg: args.Config, writer: anytype, backing: std.mem.Allocator) !void {
    const cross: []const u8 = if (cfg.no_color) "x" else "\xe2\x9c\x97"; // ✗
    const warn: []const u8 = if (cfg.no_color) "!" else "\xe2\x9a\xa0"; // ⚠
    const dash: []const u8 = if (cfg.no_color) "-" else "\xe2\x80\x94"; // —

    var arena = std.heap.ArenaAllocator.init(backing);
    defer arena.deinit();
    const aa = arena.allocator();

    const raw_path = std.process.getEnvVarOwned(aa, "PATH") catch {
        try writer.writeAll("ph: PATH is not set or empty\n");
        return error.MissingEnv;
    };
    try runCleanWithPath(raw_path, cross, warn, dash, writer, aa);
}

fn runCleanWithPath(
    raw_path: []const u8,
    cross: []const u8,
    warn: []const u8,
    dash: []const u8,
    writer: anytype,
    aa: std.mem.Allocator,
) !void {
    var sm = try source.build(aa);
    defer sm.deinit();
    var auditor = audit.Auditor.init(aa, &sm);
    defer auditor.deinit();

    const entries = try auditor.scan(raw_path);

    // Count dead entries by source
    var n_user: usize = 0;
    var n_system: usize = 0;
    var n_unknown: usize = 0;
    for (entries) |e| {
        if (!e.state.isDead()) continue;
        if (std.mem.eql(u8, e.source, "User")) n_user += 1 else if (std.mem.eql(u8, e.source, "System")) n_system += 1 else n_unknown += 1;
    }

    const n_dead = n_user + n_system + n_unknown;
    if (n_dead == 0) {
        try writer.writeAll("No dead entries found.\n");
        return;
    }

    try writer.writeAll("Dead entries to remove:\n\n");

    // Collect dead User paths (lowercased for registry comparison)
    var dead_keys = std.ArrayList([]const u8){};

    if (n_user > 0) {
        try writer.writeAll("  [User]\n");
        for (entries) |e| {
            if (!e.state.isDead()) continue;
            if (!std.mem.eql(u8, e.source, "User")) continue;
            try writer.print("    {s}  {s}\n", .{ cross, e.path });
            const lower = try aa.dupe(u8, e.path);
            source.asciiLowerInPlace(lower);
            try dead_keys.append(aa, lower);
        }
        try writer.writeByte('\n');
    }

    if (n_system > 0) {
        try writer.print("  [System]  {s} requires administrator {s} will be skipped\n", .{ warn, dash });
        for (entries) |e| {
            if (!e.state.isDead()) continue;
            if (!std.mem.eql(u8, e.source, "System")) continue;
            try writer.print("    {s}  {s}\n", .{ cross, e.path });
        }
        try writer.writeByte('\n');
    }

    if (n_unknown > 0) {
        try writer.print("  [Unknown source {s} skipped]\n", .{dash});
        for (entries) |e| {
            if (!e.state.isDead()) continue;
            if (std.mem.eql(u8, e.source, "User") or std.mem.eql(u8, e.source, "System")) continue;
            try writer.print("    {s}  {s}\n", .{ cross, e.path });
        }
        try writer.writeByte('\n');
    }

    if (n_user == 0) {
        try writer.writeAll("No User PATH entries to remove (System entries require administrator).\n");
        return;
    }

    const plural: []const u8 = if (n_user == 1) "y" else "ies";
    try writer.print("Remove {} User PATH entr{s}? [y/N] ", .{ n_user, plural });

    // Flush buffered writer so the prompt appears before we block on stdin
    try writer.flush();

    // Read confirmation from stdin
    var stdin_buf: [64]u8 = undefined;
    var sr = std.fs.File.stdin().readerStreaming(&stdin_buf);
    const answer = blk: {
        const line = sr.interface.takeDelimiterExclusive('\n') catch |e| switch (e) {
            error.EndOfStream => break :blk "",
            else => return e,
        };
        break :blk std.mem.trim(u8, line, " \t\r\n");
    };

    if (!std.mem.eql(u8, answer, "y") and !std.mem.eql(u8, answer, "Y")) {
        try writer.writeAll("No changes made.\n");
        return;
    }

    // Read raw registry segments, filter out dead ones
    const raw_segs = try source.readRawUserPathSegments(aa);
    var surviving = std.ArrayList([]const u8){};

    for (raw_segs) |seg| {
        // Expand and normalize to get the lowercase key
        const expanded = source.expandEnvVarsWinPub(aa, seg) catch seg;
        const normed = source.stripTrailingSlash(expanded);
        const lower = try aa.dupe(u8, normed);
        source.asciiLowerInPlace(lower);

        var is_dead = false;
        for (dead_keys.items) |dk| {
            if (std.mem.eql(u8, lower, dk)) {
                is_dead = true;
                break;
            }
        }
        if (!is_dead) try surviving.append(aa, seg);
    }

    try source.writeUserPathToRegistry(surviving.items);
    source.broadcastEnvChange();

    const rm_plural: []const u8 = if (n_user == 1) "y" else "ies";
    try writer.print("Removed {} entr{s}. User PATH updated.\n", .{ n_user, rm_plural });
}
