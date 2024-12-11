#pragma once
#include <cstddef>
#include <string>
#include <string_view>

std::size_t      strip_trailing_slash(std::string_view path);
std::string      ascii_lower(std::string_view s);
std::size_t      utf8_cp_count(std::string_view s);
std::string_view str_ltrim(std::string_view s);
std::string_view str_rtrim(std::string_view s);
