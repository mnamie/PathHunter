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
 C:\Program Files (x86)\NVIDIA Corporation\PhysX\Common
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
 C:\Users\micha\.cargo\bin
 C:\Program Files\Git\cmd
 C:\dev
 C:\Program Files\CMake\bin
 C:\Users\micha\AppData\Local\Microsoft\WinGet
 C:\Program Files\Mullvad VPN\resources
 C:\Program Files\PowerShell\7
 C:\ProgramData\mingw64\mingw64\bin
 C:\Users\micha\.local\bin
 C:\dev\nim-2.2.0\bin
 C:\Program Files\LOVE
 C:\Program Files\Go\bin
 C:\Users\micha\Projects\PathHunterRS\target\release
]
Missing path targets:
 [*] All clear
```

## Buid instructions:
1. `cargo build --release`

## FAQ:
* Why not just use some version analogue of `getenv("Path")`?

    This type of functionality fetches the environment variable as it exists in the environment. Many shells inject additional dependencies into the `Path` variable. Fetching from the Windows registry keys ensures we are only checking user defined Path entries. 