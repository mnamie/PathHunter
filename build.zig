const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const exe = b.addExecutable(.{
        .name = "ph",
        .root_module = exe_mod,
    });

    if (target.result.os.tag == .windows) {
        exe_mod.linkSystemLibrary("advapi32", .{});
        exe_mod.linkSystemLibrary("user32", .{});
        exe_mod.link_libc = true;
    }

    b.installArtifact(exe);

    const test_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    const unit_tests = b.addTest(.{
        .root_module = test_mod,
    });

    if (target.result.os.tag == .windows) {
        test_mod.linkSystemLibrary("advapi32", .{});
        test_mod.linkSystemLibrary("user32", .{});
        test_mod.link_libc = true;
    }

    const run_unit_tests = b.addRunArtifact(unit_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_unit_tests.step);
}
