cmake_minimum_required(VERSION 3.20)
project(PathHunter)

message("Generated with config types: ${CMAKE_CONFIGURATION_TYPES}")
message("Source dir: ${CMAKE_SOURCE_DIR}")
set(CMAKE_INSTALL_PREFIX "${CMAKE_SOURCE_DIR}")

add_subdirectory(src)