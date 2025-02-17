cmake_minimum_required(VERSION 3.20)
project(PathHunter)

set(CMAKE_SYSTEM "Windows")

message("Generated with config types: ${CMAKE_CONFIGURATION_TYPES}")
message("Source dir: ${CMAKE_SOURCE_DIR}")
set(CMAKE_INSTALL_PREFIX "${CMAKE_SOURCE_DIR}")

set(TARGET_NAME path_hunter)
add_executable(
    ${TARGET_NAME}
    src/main.c
    src/path.c
)

if(MSVC)
  target_compile_options(${TARGET_NAME} PRIVATE /W4 /WX)
endif()

install(TARGETS ${TARGET_NAME} DESTINATION bin)