const std = @import("std");
const win = std.os.windows;
const reg = win.advapi32;

const Allocator = std.mem.Allocator;

pub const Option = enum { print, clean };

// Registry key and value we want to pull
// const key = std.unicode.utf8ToUtf16LeStringLiteral("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment");
const key = std.unicode.utf8ToUtf16LeStringLiteral("Environment");
const val = std.unicode.utf8ToUtf16LeStringLiteral("Path");

/// Fetches path environment variable return, converting the winapi utf16le to u8
pub fn fetchPathEnvVariable(allocator: Allocator) ![]u8 {
    // Windows HKEY for resulting registry entry
    var hkey: win.HKEY = undefined;

    const status = reg.RegOpenKeyExW(
        win.HKEY_CURRENT_USER,
        key,
        0,
        win.KEY_READ,
        &hkey,
    );

    // Return error if registry fails to open
    if (status != 0)
        return error.OpenFailed;

    // Deferred registry closing
    defer {
        _ = reg.RegCloseKey(hkey);
    }

    // Fetch data length of result, so we can allocate our buffer
    var data_len: win.DWORD = 0;
    if (reg.RegQueryValueExW(hkey, val, null, null, null, &data_len) != 0)
        return error.QuerySizeFailed;

    // Allocate buffer and defer cleanup
    const buffer = try allocator.alloc(u8, data_len);
    defer allocator.free(buffer);
    const buffer_ptr: ?*u8 = @ptrCast(buffer.ptr);

    // Finally query registry key/value
    if (reg.RegQueryValueExW(hkey, val, null, null, buffer_ptr, &data_len) != 0)
        return error.QueryFailed;

    // winapi works with utf16le; need to convert to utf8 to work with the rest of Zig stdlib
    const product_name_utf16: [*]const u16 = @ptrCast(@alignCast(buffer_ptr));
    const product_name = try std.unicode.utf16LeToUtf8Alloc(allocator, product_name_utf16[0 .. data_len / 2]);

    return product_name;
}

/// Converts a Windows style path with '\'s into a compatible path with '/'s
pub fn pathToCompatiblePath(path: []u8) void {
    std.mem.replaceScalar(u8, path, '\\', '/');
}

/// Checks if a str exists either as a directory or file
pub fn fileExists(path: []const u8) bool {
    _ = std.fs.openFileAbsolute(path, .{}) catch |err| {
        if (err == error.IsDir) {
            return true;
        } else {
            return false;
        }
    };
    return true;
}

/// Parses command line arguments to our CLI flag options
pub fn parseArgsToOption(allocator: Allocator) !Option {
    var args = try std.process.argsWithAllocator(allocator);
    defer args.deinit();

    var option: Option = Option.print;

    while (args.next()) |arg| {
        if (std.mem.eql(u8, arg, "-c")) option = Option.clean;
    }

    return option;
}
