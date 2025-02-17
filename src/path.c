#include <Windows.h>
#include <sys/stat.h>
#include <string.h>
#include <stdio.h>

#include "path.h"

int FetchPathEnvVariable(const char *path_str, const int *buffer_size)
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

int CheckPathFileExists(const char *path)
{
    struct stat file_stats;
    return stat(path, &file_stats);
}

void PrintCleanedPathStr(char *path_str)
{
    char *token = NULL;
    char *nextToken = NULL;

    printf("\nPath: [\n");

    token = strtok_s(path_str, ";", &nextToken);
    while (token != NULL) 
    {
        printf(" %s\n", token);
        token = strtok_s(NULL, ";", &nextToken);
    }
    
    printf("]\n");
}

void CheckPathStr(char *path_str) 
{
    char *token = NULL;
    char *next_token = NULL;
    int allCleanFlag = 1;
    int existsFlag = 0;

    printf("\nMissing Path targets:\n");

    token = strtok_s(path_str, ";", &next_token);
    while (token != NULL) 
    {
        existsFlag = CheckPathFileExists(token);
        if (existsFlag == -1)
        {
            allCleanFlag = 0;
            printf(" [*] %s\n", token);
        }
        token = strtok_s(NULL, ";", &next_token);
    }

    if (existsFlag)
    {
        printf("\n [*] None! All clean!\n");
    }
}