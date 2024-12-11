#include "strutil.hpp"
#include <cctype>

std::size_t strip_trailing_slash(std::string_view path) {
    std::size_t len = path.size();
#ifdef _WIN32
    if (len == 3 && path[1] == ':')
        return len;
    while (len > 1 && (path[len - 1] == '/' || path[len - 1] == '\\'))
        len--;
#else
    while (len > 1 && path[len - 1] == '/')
        len--;
#endif
    return len;
}

std::string ascii_lower(std::string_view s) {
    std::string result(s);
    for (char& c : result)
        c = (char)std::tolower((unsigned char)c);
    return result;
}

std::size_t utf8_cp_count(std::string_view s) {
    std::size_t count = 0;
    for (unsigned char c : s)
        if ((c & 0xC0u) != 0x80u)
            count++;
    return count;
}

std::string_view str_ltrim(std::string_view s) {
    std::size_t i = 0;
    while (i < s.size() && (s[i] == ' ' || s[i] == '\t'))
        i++;
    return s.substr(i);
}

std::string_view str_rtrim(std::string_view s) {
    std::size_t n = s.size();
    while (n > 0) {
        char c = s[n - 1];
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n')
            n--;
        else
            break;
    }
    return s.substr(0, n);
}
