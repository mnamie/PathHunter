#include <Windows.h>
#include <sys/stat.h>
#include <string.h>
#include <stdio.h>

#include "path.h"

int fetch_path_env_variable(const char *path_str, const int *buffer_size)
{
    return RegGetValueA(
        HKEY_LOCAL_MACHINE,
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
        "Path",
        RRF_RT_ANY,
        NULL,
        (PVOID) path_str,
        (LPDWORD) buffer_size
    );
}

int check_path_file_exists(const char *path)
{
    struct stat file_stats;
    return stat(path, &file_stats);
}

void check_path_str(char *path_str) 
{
    char* ptr = strtok(path_str, ";");
    int all_clean_flag = 1;
    int exists_flag = 0;

    printf("\nMissing Path targets:\n");

    while (ptr != NULL) 
    {
        exists_flag = check_path_file_exists(ptr);
        if (exists_flag == -1)
        {
            all_clean_flag = 0;
            printf(" [*] %s\n", ptr);
        }
        ptr = strtok(NULL, ";");
    }

    if (all_clean_flag)
    {
        printf("\n [*] None! All clean!\n");
    }
}
