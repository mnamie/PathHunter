add_executable(
    path_hunter
    "main.c"
    "path.c"
)

install(TARGETS path_hunter DESTINATION bin)