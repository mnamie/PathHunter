#ifndef PATH_H
#define PATH_H

int fetch_path_env_variable(const char *path_str, const int *buffer_size);
static int check_path_file_exists(const char *path);
void check_path_str(char *path_str);

#endif