#include <stdio.h>
#include <Windows.h>

#include "path.h"

#define MAX_REGISTRY_VALUE_SIZE 16383

int main()
{
    char path_str[MAX_REGISTRY_VALUE_SIZE] = { 0 };
    const int size = MAX_REGISTRY_VALUE_SIZE;
    int result = 0;

    result = fetch_path_env_variable(path_str, &size);
    if (result != ERROR_SUCCESS)
    {
        return 1;
    }

    printf("\nPath: [%s]\n", path_str);
    check_path_str(path_str);
    
    return 0;
}