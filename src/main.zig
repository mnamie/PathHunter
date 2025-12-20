const std = @import("std");
const path = @import("./path.zig");
const utils = @import("./utils.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    const option: utils.Option = try utils.parseArgsToOption(allocator);

    var path_checker: path.PathEnv = try path.PathEnv.init(allocator, option);
    defer path_checker.deinit();
    try path_checker.run();
}
