#include <stdio.h>
#include <Windows.h>
#include <stdlib.h>

#include "path.h"

#define MAX_REGISTRY_VALUE_SIZE 16383

int main()
{
    char pathStr[MAX_REGISTRY_VALUE_SIZE] = { 0 };
    char validationPathStr[MAX_REGISTRY_VALUE_SIZE] = { 0 };
    const int size = MAX_REGISTRY_VALUE_SIZE;
    int result = 0;
    
    result = FetchPathEnvVariable(pathStr, &size);
    if (result != ERROR_SUCCESS)
    {
        return 1;
    }

    memcpy(validationPathStr, pathStr, MAX_REGISTRY_VALUE_SIZE);
    
    PrintCleanedPathStr(pathStr);
    CheckPathStr(validationPathStr);
    
    return 0;
}