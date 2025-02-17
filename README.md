# PathHunter

PathHunter is a quick Windows utility I wrote to check which targets in my Path environment variable no longer exist. 

## Sample output:

```
Path: [
 C:\Windows\system32
 C:\Windows
 C:\Windows\System32\Wbem
 C:\Windows\System32\WindowsPowerShell\v1.0\
 C:\Windows\System32\OpenSSH\
 C:\ProgramData\chocolatey\bin
 C:\WINDOWS\system32
 C:\WINDOWS
 C:\WINDOWS\System32\Wbem
 C:\WINDOWS\System32\WindowsPowerShell\v1.0\
 C:\WINDOWS\System32\OpenSSH\
 C:\Program Files\Microsoft VS Code\bin
 C:\Program Files\dotnet\
 C:\Program Files\Microsoft SQL Server\150\Tools\Binn\
 C:\Program Files\SDL2\lib\x64
 C:\Program Files\7-Zip
 C:\Program Files\Git\cmd
 C:\dev
 C:\Program Files\CMake\bin
 C:\Program Files\PowerShell\7
 C:\ProgramData\mingw64\mingw64\bin
 C:\dev\nim-2.2.0\bin
 C:\Program Files\Docker\Docker\resources\bin
]

Missing Path targets:
 [*] C:\dev\nim-2.2.0\bin
```

## Buid instructions using a UNIX environment or PowerShell:
1. Make build directory with: `mkdir build`
2. Move into build directory: `cd build`
3. Generate build files: `cmake ..`
4. Build: `cmake --build . --config Release`
5. (Optional) Install to `bin/`: `cmake --install . --config Release`

## FAQ:
* Why not just use getenv("Path")?

    `getenv(...)` fetches the environment variable as it exists in the runtime environment. Many shells inject additional dependencies into the `Path` variable. Fetching from the Windows registry keys ensures we are only checking user space Path variables. 