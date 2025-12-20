const std = @import("std");
const utils = @import("./utils.zig");

const win = std.os.windows;
const reg = win.advapi32;

const Allocator = std.mem.Allocator;

/// Path Environment object
pub const PathEnv = struct {
    allocator: Allocator,
    option: utils.Option,
    path_str: []u8,
    validation_path_str: []u8,
    cleaned_path_str: []u8,
    is_clean: bool = true,

    /// Path Environment initializer
    pub fn init(allocator: Allocator, option: utils.Option) !PathEnv {
        const path_str = try utils.fetchPathEnvVariable(allocator);
        const validation_path_str = try utils.fetchPathEnvVariable(allocator);
        const cleaned_path_str = try utils.fetchPathEnvVariable(allocator);

        utils.pathToCompatiblePath(path_str);
        utils.pathToCompatiblePath(validation_path_str);
        utils.pathToCompatiblePath(cleaned_path_str);

        return PathEnv{ .allocator = allocator, .option = option, .path_str = path_str, .validation_path_str = validation_path_str, .cleaned_path_str = cleaned_path_str };
    }

    /// Deinitialize owned resources
    pub fn deinit(self: *PathEnv) void {
        self.allocator.free(self.path_str);
        self.allocator.free(self.validation_path_str);
        self.allocator.free(self.cleaned_path_str);
    }

    /// Print cleaned path environment variable
    fn printCleanPath(self: *PathEnv) void {
        std.debug.print("Path: [\n", .{});

        var iter = std.mem.splitAny(u8, self.path_str, ";");
        while (iter.next()) |word| {
            std.debug.print("\t{s}\n", .{word});
        }

        std.debug.print("]\n", .{});
    }

    /// Print missing path environment variable targets
    fn printMissingPathTargets(self: *PathEnv) !void {
        std.debug.print("Missing path targets: [\n", .{});

        self.is_clean = true;

        var iter = std.mem.splitAny(u8, self.validation_path_str, ";");
        while (iter.next()) |word| {
            if (word.len <= 1) continue;
            if (std.mem.containsAtLeast(u8, word, 1, "%")) continue;
            if (!utils.fileExists(word)) {
                std.debug.print("\t{s}\n", .{word});
                self.cleaned_path_str = try std.mem.replaceOwned(u8, self.allocator, self.cleaned_path_str, word, "");
                self.is_clean = false;
            }
        }
        if (self.is_clean) std.debug.print("\t[*] All path targets exist\n", .{});
        std.debug.print("]\n", .{});
    }

    /// Conditionally clean path environment variable, writing to registry, depending on if there were issues
    fn conditionallyCleanPath(self: *PathEnv) void {
        if (self.is_clean) return;
    }

    /// Launch whole process
    pub fn run(self: *PathEnv) !void {
        self.printCleanPath();
        try self.printMissingPathTargets();
        self.conditionallyCleanPath();
    }
};
