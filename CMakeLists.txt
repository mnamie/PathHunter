cmake_minimum_required(VERSION 3.15)
project(PathHunter VERSION 0.3.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(PH_SOURCES
    src/main.cpp
    src/args.cpp
    src/strutil.cpp
    src/source.cpp
    src/audit.cpp
    src/display.cpp
    src/clean.cpp
)

add_executable(ph ${PH_SOURCES})
target_include_directories(ph PRIVATE src)

if(MSVC)
    target_compile_options(ph PRIVATE /W4)
else()
    target_compile_options(ph PRIVATE -Wall -Wextra -pedantic)
    if(CMAKE_COMPILER_IS_GNUCXX AND CMAKE_CXX_COMPILER_VERSION VERSION_LESS 9.0)
        target_link_libraries(ph PRIVATE stdc++fs)
    endif()
endif()

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "" FORCE)
endif()

if(WIN32)
    target_link_libraries(ph PRIVATE advapi32 user32)
endif()

set_target_properties(ph PROPERTIES
    OUTPUT_NAME "ph"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_SOURCE_DIR}/bin"
)
