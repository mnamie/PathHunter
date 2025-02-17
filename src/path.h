#ifndef PATH_H
#define PATH_H

int FetchPathEnvVariable(const char *path_str, const int *buffer_size);
void PrintCleanedPathStr(char *path_str);
void CheckPathStr(char *path_str);

#endif