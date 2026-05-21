const std = @import("std");
const builtin = @import("builtin");

// ── Windows Win32 declarations ────────────────────────────────────────────
// Primitive types used so these declarations compile on every host.
// Functions are only reachable when compiling for Windows.

const HKEY_LOCAL_MACHINE: *anyopaque = @ptrFromInt(0x80000002);
const HKEY_CURRENT_USER: *anyopaque = @ptrFromInt(0x80000001);
const HWND_BROADCAST: ?*anyopaque = @ptrFromInt(0xFFFF);
const KEY_READ: u32 = 0x20019;
const KEY_READ_WRITE: u32 = 0x20019 | 0x0002;
const REG_EXPAND_SZ: u32 = 2;
const ERROR_SUCCESS: i32 = 0;
const WM_SETTINGCHANGE: u32 = 0x001A;
const SMTO_ABORTIFHUNG: u32 = 0x0002;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const STD_OUTPUT_HANDLE: u32 = @bitCast(@as(i32, -11));
const CP_UTF8: u32 = 65001;

extern "advapi32" fn RegOpenKeyExW(
    hKey: *anyopaque,
    lpSubKey: [*:0]const u16,
    ulOptions: u32,
    samDesired: u32,
    phkResult: **anyopaque,
) i32;

extern "advapi32" fn RegQueryValueExW(
    hKey: *anyopaque,
    lpValueName: ?[*:0]const u16,
    lpReserved: ?*u32,
    lpType: ?*u32,
    lpData: ?[*]u8,
    lpcbData: ?*u32,
) i32;

extern "advapi32" fn RegSetValueExW(
    hKey: *anyopaque,
    lpValueName: ?[*:0]const u16,
    Reserved: u32,
    dwType: u32,
    lpData: ?[*]const u8,
    cbData: u32,
) i32;

extern "advapi32" fn RegCloseKey(hKey: *anyopaque) i32;

extern "kernel32" fn ExpandEnvironmentStringsW(
    lpSrc: [*:0]const u16,
    lpDst: [*]u16,
    nSize: u32,
) u32;

extern "kernel32" fn SetConsoleOutputCP(wCodePageID: u32) i32;
extern "kernel32" fn SetConsoleCP(wCodePageID: u32) i32;
extern "kernel32" fn GetStdHandle(nStdHandle: u32) ?*anyopaque;
extern "kernel32" fn GetConsoleMode(hConsoleHandle: *anyopaque, lpMode: *u32) i32;
extern "kernel32" fn SetConsoleMode(hConsoleHandle: *anyopaque, dwMode: u32) i32;

extern "user32" fn SendMessageTimeoutW(
    hWnd: ?*anyopaque,
    Msg: u32,
    wParam: usize,
    lParam: isize,
    fuFlags: u32,
    uTimeout: u32,
    lpdwResult: ?*usize,
) isize;

// ── SourceMap ─────────────────────────────────────────────────────────────

pub const SourceMap = struct {
    map: std.StringHashMap([]const u8),
    arena: std.heap.ArenaAllocator,

    pub fn init(backing: std.mem.Allocator) SourceMap {
        return .{
            .map = std.StringHashMap([]const u8).init(backing),
            .arena = std.heap.ArenaAllocator.init(backing),
        };
    }

    pub fn deinit(self: *SourceMap) void {
        self.map.deinit();
        self.arena.deinit();
    }

    pub fn lookup(self: *const SourceMap, path: []const u8) ?[]const u8 {
        return self.map.get(path);
    }

    fn insert(self: *SourceMap, path: []const u8, source: []const u8) !void {
        if (self.map.contains(path)) return; // first source wins
        const k = try self.arena.allocator().dupe(u8, path);
        try self.map.put(k, source);
    }
};

pub fn build(backing: std.mem.Allocator, io: std.Io, environ: std.process.Environ) !SourceMap {
    var sm = SourceMap.init(backing);
    if (builtin.os.tag == .windows) {
        try buildWindows(&sm);
    } else {
        try buildUnix(&sm, io, environ);
    }
    return sm;
}

// ── Windows implementation ────────────────────────────────────────────────

fn buildWindows(sm: *SourceMap) !void {
    const aa = sm.arena.allocator();

    try addRegistryPath(sm, aa, HKEY_LOCAL_MACHINE, std.unicode.utf8ToUtf16LeStringLiteral("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"), "System");

    try addRegistryPath(sm, aa, HKEY_CURRENT_USER, std.unicode.utf8ToUtf16LeStringLiteral("Environment"), "User");
}

fn addRegistryPath(
    sm: *SourceMap,
    aa: std.mem.Allocator,
    root_key: *anyopaque,
    subkey: [*:0]const u16,
    label: []const u8,
) !void {
    var hkey: *anyopaque = undefined;
    if (RegOpenKeyExW(root_key, subkey, 0, KEY_READ, &hkey) != ERROR_SUCCESS) return;
    defer _ = RegCloseKey(hkey);

    const val_name = std.unicode.utf8ToUtf16LeStringLiteral("Path");

    // First call: get required buffer size
    var data_size: u32 = 0;
    var data_type: u32 = 0;
    if (RegQueryValueExW(hkey, val_name, null, &data_type, null, &data_size) != ERROR_SUCCESS) return;

    // Allocate u16 buffer (data_size is in bytes)
    const num_u16 = data_size / 2 + 1;
    const wbuf = try aa.alloc(u16, num_u16);
    @memset(wbuf, 0);

    if (RegQueryValueExW(hkey, val_name, null, &data_type, @ptrCast(wbuf.ptr), &data_size) != ERROR_SUCCESS) return;

    // Find null terminator
    var wlen: usize = 0;
    while (wlen < wbuf.len and wbuf[wlen] != 0) wlen += 1;

    const raw_utf8 = try std.unicode.utf16LeToUtf8Alloc(aa, wbuf[0..wlen]);

    // Split on ';' and process each segment
    var it = std.mem.splitScalar(u8, raw_utf8, ';');
    while (it.next()) |seg| {
        const trimmed = std.mem.trim(u8, seg, " \t");
        if (trimmed.len == 0) continue;

        const expanded = expandEnvVarsWin(aa, trimmed) catch continue;
        if (expanded.len == 0) continue;

        const normed = stripTrailingSlash(expanded);
        if (normed.len == 0) continue;

        // Lowercase for case-insensitive dedup key
        const lower = try aa.dupe(u8, normed);
        asciiLowerInPlace(lower);

        try sm.insert(lower, label);
    }
}

pub fn expandEnvVarsWinPub(aa: std.mem.Allocator, utf8: []const u8) ![]const u8 {
    return expandEnvVarsWin(aa, utf8);
}

fn expandEnvVarsWin(aa: std.mem.Allocator, utf8: []const u8) ![]const u8 {
    const wsrc = try std.unicode.utf8ToUtf16LeAllocZ(aa, utf8);

    var wdst: [32768]u16 = undefined;
    const written = ExpandEnvironmentStringsW(wsrc.ptr, &wdst, @intCast(wdst.len));
    if (written == 0) return utf8;

    const wlen = if (written > 0 and wdst[written - 1] == 0) written - 1 else written;
    return try std.unicode.utf16LeToUtf8Alloc(aa, wdst[0..wlen]);
}

// ── Registry write helpers (used by clean.zig) ────────────────────────────

pub fn readRawUserPathSegments(aa: std.mem.Allocator) ![][]const u8 {
    var hkey: *anyopaque = undefined;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, std.unicode.utf8ToUtf16LeStringLiteral("Environment"), 0, KEY_READ, &hkey) != ERROR_SUCCESS) return &.{};
    defer _ = RegCloseKey(hkey);

    const val_name = std.unicode.utf8ToUtf16LeStringLiteral("Path");
    var data_size: u32 = 0;
    var data_type: u32 = 0;
    if (RegQueryValueExW(hkey, val_name, null, &data_type, null, &data_size) != ERROR_SUCCESS) return &.{};

    const num_u16 = data_size / 2 + 1;
    const wbuf = try aa.alloc(u16, num_u16);
    @memset(wbuf, 0);
    if (RegQueryValueExW(hkey, val_name, null, &data_type, @ptrCast(wbuf.ptr), &data_size) != ERROR_SUCCESS) return &.{};

    var wlen: usize = 0;
    while (wlen < wbuf.len and wbuf[wlen] != 0) wlen += 1;
    const raw_utf8 = try std.unicode.utf16LeToUtf8Alloc(aa, wbuf[0..wlen]);

    var list: std.ArrayList([]const u8) = .empty;
    var it = std.mem.splitScalar(u8, raw_utf8, ';');
    while (it.next()) |seg| {
        const trimmed = std.mem.trim(u8, seg, " \t");
        if (trimmed.len > 0) try list.append(aa, try aa.dupe(u8, trimmed));
    }
    return list.toOwnedSlice(aa);
}

pub fn writeUserPathToRegistry(segments: []const []const u8) !void {
    var buf: [32768]u8 = undefined;
    var pos: usize = 0;
    for (segments, 0..) |seg, i| {
        if (i > 0) {
            if (pos >= buf.len) break;
            buf[pos] = ';';
            pos += 1;
        }
        const n = @min(seg.len, buf.len - pos);
        @memcpy(buf[pos..][0..n], seg[0..n]);
        pos += n;
    }

    const value_utf8 = buf[0..pos];
    var tmp_alloc = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer tmp_alloc.deinit();
    const wvalue = try std.unicode.utf8ToUtf16LeAllocZ(tmp_alloc.allocator(), value_utf8);

    var hkey: *anyopaque = undefined;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, std.unicode.utf8ToUtf16LeStringLiteral("Environment"), 0, KEY_READ_WRITE, &hkey) != ERROR_SUCCESS) return;
    defer _ = RegCloseKey(hkey);

    const byte_len: u32 = @intCast((wvalue.len + 1) * @sizeOf(u16));
    _ = RegSetValueExW(hkey, std.unicode.utf8ToUtf16LeStringLiteral("Path"), 0, REG_EXPAND_SZ, @ptrCast(wvalue.ptr), byte_len);
}

pub fn broadcastEnvChange() void {
    var param = std.unicode.utf8ToUtf16LeStringLiteral("Environment").*;
    var res: usize = 0;
    _ = SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, @bitCast(@intFromPtr(&param)), SMTO_ABORTIFHUNG, 5000, &res);
}

pub fn setupWindowsConsole() bool {
    _ = SetConsoleOutputCP(CP_UTF8);
    _ = SetConsoleCP(CP_UTF8);
    const handle = GetStdHandle(STD_OUTPUT_HANDLE) orelse return false;
    var mode: u32 = 0;
    if (GetConsoleMode(handle, &mode) == 0) return false;
    _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    return true;
}

// ── Unix implementation ───────────────────────────────────────────────────

fn buildUnix(sm: *SourceMap, io: std.Io, environ: std.process.Environ) !void {
    const aa = sm.arena.allocator();

    const home_raw = environ.getPosix("HOME") orelse "";
    const home = stripTrailingSlash(home_raw);

    // /etc files first
    const etc_files = [_][2][]const u8{
        .{ "/etc/environment", "/etc/environment" },
        .{ "/etc/profile.d/apps-bin-path.sh", "/etc/profile.d/" },
    };
    for (etc_files) |pair| {
        parseShellConfig(sm, aa, pair[0], pair[1], home, io) catch {};
    }

    // Home-relative shell config files
    const rel_files = [_][2][]const u8{
        .{ "/.profile", "~/.profile" },
        .{ "/.bash_profile", "~/.bash_profile" },
        .{ "/.bash_login", "~/.bash_login" },
        .{ "/.bashrc", "~/.bashrc" },
        .{ "/.zprofile", "~/.zprofile" },
        .{ "/.zshrc", "~/.zshrc" },
        .{ "/.cargo/env", "~/.cargo/env" },
        .{ "/.nimble/env", "~/.nimble/env" },
    };
    for (rel_files) |pair| {
        const full_path = try std.fmt.allocPrint(aa, "{s}{s}", .{ home, pair[0] });
        parseShellConfig(sm, aa, full_path, pair[1], home, io) catch {};
    }
}

fn parseShellConfig(
    sm: *SourceMap,
    aa: std.mem.Allocator,
    path: []const u8,
    label: []const u8,
    home: []const u8,
    _: std.Io,
) !void {
    const path_z = try aa.dupeZ(u8, path);
    const fd = std.posix.openatZ(std.os.linux.AT.FDCWD, path_z, .{ .ACCMODE = .RDONLY }, 0) catch return;
    defer _ = std.os.linux.close(fd);

    // Read entire file into a single allocation
    var content: std.ArrayList(u8) = .empty;
    var read_buf: [4096]u8 = undefined;
    while (true) {
        const n = try std.posix.read(fd, &read_buf);
        if (n == 0) break;
        try content.appendSlice(aa, read_buf[0..n]);
    }

    var it = std.mem.splitScalar(u8, content.items, '\n');
    while (it.next()) |line_raw| {
        const line = std.mem.trimEnd(u8, line_raw, "\r\n");
        try parsePathLine(sm, aa, line, label, home);
    }
}

fn parsePathLine(
    sm: *SourceMap,
    aa: std.mem.Allocator,
    line: []const u8,
    label: []const u8,
    home: []const u8,
) !void {
    const trimmed = std.mem.trimStart(u8, line, " \t");
    if (trimmed.len == 0 or trimmed[0] == '#') return;

    var value: []const u8 = undefined;
    if (std.mem.startsWith(u8, trimmed, "export PATH=")) {
        value = trimmed["export PATH=".len..];
    } else if (std.mem.startsWith(u8, trimmed, "PATH=")) {
        value = trimmed["PATH=".len..];
    } else return;

    // Strip surrounding quotes
    var val = value;
    if (val.len >= 2 and val[0] == '"' and val[val.len - 1] == '"') {
        val = val[1 .. val.len - 1];
    } else if (val.len >= 2 and val[0] == '\'' and val[val.len - 1] == '\'') {
        val = val[1 .. val.len - 1];
    }

    // Remove inline comment (space + #)
    if (std.mem.indexOf(u8, val, " #")) |idx| val = val[0..idx];

    var it = std.mem.splitScalar(u8, val, ':');
    while (it.next()) |seg| {
        const s = std.mem.trim(u8, seg, " \t");
        if (s.len == 0) continue;
        if (std.mem.eql(u8, s, "$PATH") or
            std.mem.eql(u8, s, "${PATH}") or
            std.mem.eql(u8, s, "$path")) continue;

        const expanded = try expandVarsUnix(aa, s, home);
        if (std.mem.indexOfScalar(u8, expanded, '$') != null) continue; // unexpanded var

        const normed = stripTrailingSlash(expanded);
        if (normed.len == 0) continue;

        try sm.insert(normed, label);
    }
}

fn expandVarsUnix(aa: std.mem.Allocator, s: []const u8, home: []const u8) ![]const u8 {
    // Leading ~/  or bare ~
    if (s.len == 1 and s[0] == '~') return home;
    if (std.mem.startsWith(u8, s, "~/")) {
        return try std.fmt.allocPrint(aa, "{s}{s}", .{ home, s[1..] });
    }

    // Single-pass $HOME / ${HOME} expansion
    var out: std.ArrayList(u8) = .empty;
    var i: usize = 0;
    while (i < s.len) {
        if (s[i] == '$') {
            const rest = s[i + 1 ..];
            const braced = rest.len > 0 and rest[0] == '{';
            const name_start: usize = if (braced) 1 else 0;
            if (std.mem.startsWith(u8, rest[name_start..], "HOME")) {
                const after = name_start + "HOME".len;
                if (!braced or (after < rest.len and rest[after] == '}')) {
                    try out.appendSlice(aa, home);
                    i += 1 + after + @as(usize, if (braced) 1 else 0);
                    continue;
                }
            }
        }
        try out.append(aa, s[i]);
        i += 1;
    }
    return out.toOwnedSlice(aa);
}

// ── Shared path utilities ─────────────────────────────────────────────────

pub fn stripTrailingSlash(path: []const u8) []const u8 {
    if (builtin.os.tag == .windows) {
        // Protect drive roots like C:\
        if (path.len == 3 and path[1] == ':') return path;
        if (path.len > 1 and (path[path.len - 1] == '/' or path[path.len - 1] == '\\'))
            return path[0 .. path.len - 1];
    } else {
        if (path.len > 1 and path[path.len - 1] == '/')
            return path[0 .. path.len - 1];
    }
    return path;
}

pub fn asciiLowerInPlace(s: []u8) void {
    for (s) |*c| c.* = std.ascii.toLower(c.*);
}
