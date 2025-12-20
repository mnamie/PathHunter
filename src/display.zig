const std = @import("std");
const audit = @import("audit.zig");
const args = @import("args.zig");

const PH_VERSION = "v0.2.0";

const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";
const DIM = "\x1b[2m";
const FG_RED = "\x1b[31m";
const FG_GREEN = "\x1b[32m";
const FG_YELLOW = "\x1b[33m";
const FG_CYAN = "\x1b[36m";
const FG_MAGENTA = "\x1b[35m";

// 2 spaces + 68 × U+2500 (─ = E2 94 80)
const DIVIDER = "  " ++ "\xe2\x94\x80" ** 68;
const SEP = "  \xc2\xb7  "; // U+00B7 middle dot

// ── Renderer ──────────────────────────────────────────────────────────────

pub const Renderer = struct {
    plain: bool,
    only_dead: bool,
    col_width: usize,
    src_width: usize,

    pub fn init(entries: []const audit.PathEntry, cfg: args.Config) Renderer {
        var max_col: usize = 0;
        var max_src: usize = 0;
        for (entries) |e| {
            const c = cpCount(e.path);
            if (c > max_col) max_col = c;
            const s = cpCount(e.source);
            if (s > max_src) max_src = s;
        }
        return .{
            .plain = cfg.no_color,
            .only_dead = cfg.only_dead,
            .col_width = @min(max_col, 50),
            .src_width = @min(max_src, 22),
        };
    }

    pub fn render(self: Renderer, writer: anytype, entries: []const audit.PathEntry) !void {
        try renderHeader(self, writer, entries.len);
        for (entries) |e| try renderEntry(self, writer, e);
        try renderSummary(self, writer, entries);
    }
};

// ── Header ────────────────────────────────────────────────────────────────

fn renderHeader(r: Renderer, writer: anytype, count: usize) !void {
    const suffix: []const u8 = if (r.only_dead) "  (showing dead only)" else "";
    if (r.plain) {
        try writer.print("  Path Hunter  {s}\n", .{PH_VERSION});
        try writer.print("  Scanning $PATH \xe2\x80\x94 {} entries{s}\n", .{ count, suffix });
        try writer.print("{s}\n", .{DIVIDER});
    } else {
        try writer.print(FG_CYAN ++ BOLD ++ "  Path Hunter" ++ RESET ++ "  {s}\n", .{PH_VERSION});
        try writer.print("  Scanning " ++ BOLD ++ "$PATH" ++ RESET ++ " \xe2\x80\x94 {} entries{s}\n", .{ count, suffix });
        try writer.print(FG_CYAN ++ "{s}" ++ RESET ++ "\n", .{DIVIDER});
    }
}

// ── Entry ─────────────────────────────────────────────────────────────────

fn renderEntry(r: Renderer, writer: anytype, e: audit.PathEntry) !void {
    if (r.only_dead) {
        switch (e.state) {
            .ok, .symlink, .duplicate => return,
            else => {},
        }
    }

    var path_buf: [640]u8 = undefined;
    const disp_path = padRight(&path_buf, truncateStr(e.path, r.col_width), r.col_width);

    var src_buf: [80]u8 = undefined;
    const disp_src = padRight(&src_buf, e.source, r.src_width);

    var det_buf: [540]u8 = undefined;
    const detail = fmtDetail(&det_buf, e);

    if (r.plain) {
        const icon: []const u8 = switch (e.state) {
            .ok => "\xe2\x9c\x93", // ✓
            .symlink => "~",
            .duplicate => "\xe2\x9a\xa0", // ⚠
            .empty => "!",
            else => "\xe2\x9c\x97", // ✗
        };
        try writer.print("  {s}  {s}  {s}{s}\n", .{ icon, disp_path, disp_src, detail });
        return;
    }

    switch (e.state) {
        .ok => try writer.print(FG_GREEN ++ BOLD ++ "  \xe2\x9c\x93  " ++ RESET ++
            "{s}" ++ FG_CYAN ++ DIM ++ "  {s}" ++ RESET ++ DIM ++ "{s}" ++ RESET ++ "\n", .{ disp_path, disp_src, detail }),

        .symlink => try writer.print(FG_CYAN ++ BOLD ++ "  ~  " ++ RESET ++
            "{s}" ++ FG_CYAN ++ DIM ++ "  {s}" ++ RESET ++ FG_CYAN ++ "{s}" ++ RESET ++ "\n", .{ disp_path, disp_src, detail }),

        .duplicate => try writer.print(FG_YELLOW ++ BOLD ++ "  \xe2\x9a\xa0  " ++ RESET ++
            FG_YELLOW ++ "{s}" ++ RESET ++ FG_CYAN ++ DIM ++ "  {s}" ++ RESET ++
            FG_YELLOW ++ "{s}" ++ RESET ++ "\n", .{ disp_path, disp_src, detail }),

        .dead, .file, .dangling => try writer.print(FG_RED ++ BOLD ++ "  \xe2\x9c\x97  " ++ RESET ++
            FG_RED ++ BOLD ++ "{s}" ++ RESET ++ FG_CYAN ++ DIM ++ "  {s}" ++ RESET ++
            FG_RED ++ DIM ++ "{s}" ++ RESET ++ "\n", .{ disp_path, disp_src, detail }),

        .empty => try writer.print(FG_MAGENTA ++ BOLD ++ "  !  " ++ RESET ++
            FG_MAGENTA ++ "{s}" ++ RESET ++ FG_CYAN ++ DIM ++ "  {s}" ++ RESET ++
            FG_MAGENTA ++ DIM ++ "{s}" ++ RESET ++ "\n", .{ disp_path, disp_src, detail }),
    }
}

fn fmtDetail(buf: []u8, e: audit.PathEntry) []const u8 {
    return switch (e.state) {
        .duplicate => |n| std.fmt.bufPrint(buf, "  [duplicate #{}]", .{n}) catch "",
        .symlink => |t| std.fmt.bufPrint(buf, "  [\xe2\x86\x92 {s}]", .{t}) catch "",
        .dangling => "  [dangling symlink]",
        .file => "  [is a file, not a dir]",
        .empty => "  [means current directory \xe2\x80\x94 security risk]",
        else => "",
    };
}

// ── Summary ───────────────────────────────────────────────────────────────

fn renderSummary(r: Renderer, writer: anytype, entries: []const audit.PathEntry) !void {
    var n_ok: usize = 0;
    var n_sym: usize = 0;
    var n_dead: usize = 0;
    var n_dup: usize = 0;
    var n_empty: usize = 0;
    for (entries) |e| {
        switch (e.state) {
            .ok => n_ok += 1,
            .symlink => n_sym += 1,
            .duplicate => n_dup += 1,
            .empty => n_empty += 1,
            else => n_dead += 1,
        }
    }

    if (r.plain) {
        try writer.print("{s}\n  Summary   ", .{DIVIDER});
        var first = true;
        if (n_ok > 0) {
            try writer.print("{} ok", .{n_ok});
            first = false;
        }
        {
            if (!first) try writer.writeAll(SEP);
            try writer.print("{} dead", .{n_dead});
            first = false;
        }
        if (n_sym > 0) {
            try writer.writeAll(SEP);
            try writer.print("{} symlinks", .{n_sym});
        }
        if (n_dup > 0) {
            try writer.writeAll(SEP);
            try writer.print("{} duplicates", .{n_dup});
        }
        if (n_empty > 0) {
            try writer.writeAll(SEP);
            try writer.print("{} empty", .{n_empty});
        }
        try writer.writeByte('\n');
        return;
    }

    try writer.print(FG_CYAN ++ "{s}" ++ RESET ++ "\n  Summary   ", .{DIVIDER});

    if (n_dead > 0) {
        var first = true;
        if (n_ok > 0) {
            try writer.print("{} ok", .{n_ok});
            try writer.print(DIM ++ SEP ++ RESET, .{});
            first = false;
        }
        try writer.print(FG_RED ++ BOLD ++ "{} dead" ++ RESET, .{n_dead});
        first = false;
        if (n_sym > 0) {
            try writer.print(DIM ++ SEP ++ RESET ++ "{} symlinks", .{n_sym});
        }
        if (n_dup > 0) {
            try writer.print(DIM ++ SEP ++ RESET ++ "{} duplicates", .{n_dup});
        }
        if (n_empty > 0) {
            try writer.print(DIM ++ SEP ++ RESET ++ "{} empty", .{n_empty});
        }
    } else {
        try writer.writeAll(FG_GREEN ++ BOLD);
        var first = true;
        if (n_ok > 0) {
            try writer.print("{} ok", .{n_ok});
            first = false;
        }
        if (n_sym > 0) {
            if (!first) try writer.writeAll(SEP);
            try writer.print("{} symlinks", .{n_sym});
            first = false;
        }
        if (n_dup > 0) {
            if (!first) try writer.writeAll(SEP);
            try writer.print("{} duplicates", .{n_dup});
            first = false;
        }
        if (n_empty > 0) {
            if (!first) try writer.writeAll(SEP);
            try writer.print("{} empty", .{n_empty});
        }
        try writer.writeAll(RESET);
    }

    try writer.writeAll("\n\n");
}

// ── String utilities ──────────────────────────────────────────────────────

fn cpCount(s: []const u8) usize {
    return std.unicode.utf8CountCodepoints(s) catch s.len;
}

// Returns a slice into buf containing s truncated to max_width codepoints,
// with a UTF-8 ellipsis (U+2026) appended if truncation occurred.
fn truncateStr(s: []const u8, max_width: usize) []const u8 {
    // We use a static buffer that callers must not overlap with their own.
    // Since this is only called once per entry render, a file-scope buffer works.
    const Static = struct {
        var buf: [640]u8 = undefined;
    };
    if (cpCount(s) <= max_width) {
        const n = @min(s.len, Static.buf.len);
        @memcpy(Static.buf[0..n], s[0..n]);
        return Static.buf[0..n];
    }

    var count: usize = 0;
    var pos: usize = 0;
    while (pos < s.len and count < max_width - 1) {
        if (s[pos] & 0xC0 != 0x80) count += 1;
        pos += 1;
    }
    while (pos < s.len and s[pos] & 0xC0 == 0x80) pos += 1;

    if (pos + 3 <= Static.buf.len) {
        @memcpy(Static.buf[0..pos], s[0..pos]);
        Static.buf[pos] = 0xE2;
        Static.buf[pos + 1] = 0x80;
        Static.buf[pos + 2] = 0xA6;
        return Static.buf[0 .. pos + 3];
    }
    const n = @min(s.len, Static.buf.len);
    @memcpy(Static.buf[0..n], s[0..n]);
    return Static.buf[0..n];
}

fn padRight(buf: []u8, s: []const u8, width: usize) []const u8 {
    const n = @min(s.len, buf.len);
    @memcpy(buf[0..n], s[0..n]);
    var pos = n;
    const cp = cpCount(s[0..n]);
    var spaces: usize = if (width > cp) width - cp else 0;
    while (spaces > 0 and pos < buf.len) {
        buf[pos] = ' ';
        pos += 1;
        spaces -= 1;
    }
    return buf[0..pos];
}
