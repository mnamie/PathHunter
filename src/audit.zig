const std = @import("std");
const builtin = @import("builtin");
const source = @import("source.zig");

// ── Windows kernel32 declarations ─────────────────────────────────────────

extern "kernel32" fn GetFileAttributesW(lpFileName: [*:0]const u16) u32;

extern "kernel32" fn CreateFileW(
    lpFileName: [*:0]const u16,
    dwDesiredAccess: u32,
    dwShareMode: u32,
    lpSecurityAttributes: ?*anyopaque,
    dwCreationDisposition: u32,
    dwFlagsAndAttributes: u32,
    hTemplateFile: ?*anyopaque,
) usize;

extern "kernel32" fn CloseHandle(hObject: *anyopaque) i32;

extern "kernel32" fn GetFinalPathNameByHandleW(
    hFile: *anyopaque,
    lpszFilePath: [*]u16,
    cchFilePath: u32,
    dwFlags: u32,
) u32;

const INVALID_HANDLE: usize = ~@as(usize, 0);
const INVALID_FILE_ATTRIBUTES: u32 = 0xFFFF_FFFF;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_WRITE: u32 = 0x2;
const FILE_SHARE_DELETE: u32 = 0x4;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

// ── EntryState ────────────────────────────────────────────────────────────

pub const EntryState = union(enum) {
    ok,
    symlink: []const u8, // resolved target path
    duplicate: u32, // 2nd, 3rd, … occurrence (1-based offset from first)
    dead,
    file, // exists but is a regular file, not a directory
    dangling, // broken symlink
    empty,

    pub fn isDead(self: EntryState) bool {
        return switch (self) {
            .dead, .dangling, .file => true,
            else => false,
        };
    }
};

// ── PathEntry ─────────────────────────────────────────────────────────────

pub const PathEntry = struct {
    path: []const u8,
    state: EntryState,
    source: []const u8,
};

// ── Auditor ───────────────────────────────────────────────────────────────

pub const Auditor = struct {
    seen: std.StringHashMap(u32),
    source_map: *const source.SourceMap,
    allocator: std.mem.Allocator,
    io: std.Io,

    pub fn init(allocator: std.mem.Allocator, sm: *const source.SourceMap, io: std.Io) Auditor {
        return .{
            .seen = std.StringHashMap(u32).init(allocator),
            .source_map = sm,
            .allocator = allocator,
            .io = io,
        };
    }

    pub fn deinit(self: *Auditor) void {
        self.seen.deinit();
    }

    pub fn classify(self: *Auditor, path: []const u8) !PathEntry {
        if (path.len == 0) {
            return PathEntry{
                .path = "(empty entry)",
                .state = .empty,
                .source = "",
            };
        }

        const path_dupe = try self.allocator.dupe(u8, path);
        const normed = source.stripTrailingSlash(path_dupe);

        // Build dedup key: lowercase on Windows for case-insensitive match
        const key = try self.allocator.dupe(u8, normed);
        if (builtin.os.tag == .windows) source.asciiLowerInPlace(key);

        // Source lookup
        const src_label = self.source_map.lookup(key) orelse "";

        // Seen-map: getOrPut
        const gop = try self.seen.getOrPut(key);
        if (!gop.found_existing) {
            gop.value_ptr.* = 1;
        } else {
            gop.value_ptr.* += 1;
        }
        const count = gop.value_ptr.*;

        if (count > 1) {
            return PathEntry{
                .path = path_dupe,
                .state = .{ .duplicate = count - 1 },
                .source = src_label,
            };
        }

        const state = try checkFilesystem(self.allocator, normed, self.io);
        return PathEntry{
            .path = path_dupe,
            .state = state,
            .source = src_label,
        };
    }

    pub fn scan(self: *Auditor, raw_path: []const u8) ![]PathEntry {
        const sep: u8 = if (builtin.os.tag == .windows) ';' else ':';
        var list: std.ArrayList(PathEntry) = .empty;
        var it = std.mem.splitScalar(u8, raw_path, sep);
        while (it.next()) |seg| {
            try list.append(self.allocator, try self.classify(seg));
        }
        return list.toOwnedSlice(self.allocator);
    }
};

// ── Filesystem classification ─────────────────────────────────────────────

fn checkFilesystem(alloc: std.mem.Allocator, path: []const u8, io: std.Io) !EntryState {
    if (builtin.os.tag == .windows) {
        return checkFilesystemWindows(alloc, path);
    } else {
        return checkFilesystemUnix(alloc, path, io);
    }
}

fn checkFilesystemWindows(alloc: std.mem.Allocator, path: []const u8) !EntryState {
    const wpath = try std.unicode.utf8ToUtf16LeAllocZ(alloc, path);

    const attrs = GetFileAttributesW(wpath.ptr);

    if (attrs == INVALID_FILE_ATTRIBUTES) {
        // May be a dangling symlink: try opening the reparse point directly
        const h = CreateFileW(wpath.ptr, 0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, null, OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS, null);
        if (h != INVALID_HANDLE) {
            _ = CloseHandle(@ptrFromInt(h));
            return .dangling;
        }
        return .dead;
    }

    if (attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0) {
        // Symlink — try to follow it
        const h = CreateFileW(wpath.ptr, 0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, null, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, null);
        if (h == INVALID_HANDLE) return .dangling;
        defer _ = CloseHandle(@ptrFromInt(h));

        var resolved: [4096]u16 = undefined;
        const n = GetFinalPathNameByHandleW(@ptrFromInt(h), &resolved, 4096, 0);
        if (n == 0) return .dangling;

        // Strip \\?\ prefix added by GetFinalPathNameByHandleW
        const start: usize = if (n > 4 and
            resolved[0] == '\\' and resolved[1] == '\\' and
            resolved[2] == '?' and resolved[3] == '\\') 4 else 0;

        const target = try std.unicode.utf16LeToUtf8Alloc(alloc, resolved[start..n]);
        return .{ .symlink = target };
    }

    if (attrs & FILE_ATTRIBUTE_DIRECTORY != 0) return .ok;
    return .file;
}

fn checkFilesystemUnix(alloc: std.mem.Allocator, path: []const u8, io: std.Io) !EntryState {
    const dir = std.Io.Dir.cwd();
    const lst = dir.statFile(io, path, .{ .follow_symlinks = false }) catch return .dead;

    if (lst.kind == .sym_link) {
        const tst = dir.statFile(io, path, .{}) catch return .dangling;
        if (tst.kind != .directory) return .dangling;

        var target_buf: [4096]u8 = undefined;
        const n = std.Io.Dir.readLinkAbsolute(io, path, &target_buf) catch {
            return .{ .symlink = try alloc.dupe(u8, "(resolved)") };
        };
        return .{ .symlink = try alloc.dupe(u8, target_buf[0..n]) };
    }

    if (lst.kind == .directory) return .ok;
    if (lst.kind == .file) return .file;
    return .dead;
}

// ── Tests ─────────────────────────────────────────────────────────────────

test "EntryState.isDead" {
    try std.testing.expect((@as(EntryState, .dead)).isDead());
    try std.testing.expect((@as(EntryState, .dangling)).isDead());
    try std.testing.expect((@as(EntryState, .file)).isDead());
    try std.testing.expect(!(@as(EntryState, .ok)).isDead());
    try std.testing.expect(!(@as(EntryState, .empty)).isDead());
    try std.testing.expect(!(EntryState{ .duplicate = 2 }).isDead());
    try std.testing.expect(!(EntryState{ .symlink = "/usr" }).isDead());
}

test "scan splits on separator" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const aa = arena.allocator();
    var sm = source.SourceMap.init(aa);
    var a = Auditor.init(aa, &sm, std.testing.io);
    defer a.deinit();

    if (builtin.os.tag == .windows) {
        const entries = try a.scan("C:\\foo;C:\\bar");
        try std.testing.expectEqual(@as(usize, 2), entries.len);
    } else {
        const entries = try a.scan("/usr/bin:/usr/local/bin");
        try std.testing.expectEqual(@as(usize, 2), entries.len);
    }
}

test "empty segment classified as empty" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const aa = arena.allocator();
    var sm = source.SourceMap.init(aa);
    var a = Auditor.init(aa, &sm, std.testing.io);
    defer a.deinit();
    const e = try a.classify("");
    try std.testing.expect(e.state == .empty);
}

test "duplicate detection" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const aa = arena.allocator();
    var sm = source.SourceMap.init(aa);
    var a = Auditor.init(aa, &sm, std.testing.io);
    defer a.deinit();

    const path = if (builtin.os.tag == .windows) "C:\\Windows\\System32" else "/usr/bin";
    _ = try a.classify(path);
    const e2 = try a.classify(path);
    try std.testing.expect(e2.state == .duplicate);
    try std.testing.expectEqual(@as(u32, 1), e2.state.duplicate);
}
