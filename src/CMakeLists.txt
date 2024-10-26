set(TARGET_NAME path_hunter)

add_executable(
    ${TARGET_NAME}
    "main.c"
    "path.c"
)

if(MSVC)
  target_compile_options(${TARGET_NAME} PRIVATE /W4 /WX)
endif()

install(TARGETS ${TARGET_NAME} DESTINATION bin)