# PathHunter

PathHunter is a quick Windows utility I wrote to check which targets in my Path environment variable no longer exist. 

## Sample output:

```
Path: [C:\Program Files\Eclipse Adoptium\jdk-21.0.4.7-hotspot\bin;C:\Python312\Scripts\;C:\Python312\;C:\Python311\Scripts\;C:\Python311\;C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0\;C:\Windows\System32\OpenSSH\;C:\Program Files (x86)\NVIDIA Corporation\PhysX\Common;C:\ProgramData\chocolatey\bin;C:\WINDOWS\system32;C:\WINDOWS;C:\WINDOWS\System32\Wbem;C:\WINDOWS\System32\WindowsPowerShell\v1.0\;C:\WINDOWS\System32\OpenSSH\;C:\Program Files\Microsoft VS Code\bin;C:\Program Files\dotnet\;C:\Program Files\Microsoft SQL Server\150\Tools\Binn\;C:\Program Files\SDL2\lib\x64;C:\Program Files\PowerShell\7\;C:\Program Files\7-Zip;C:\Program Files\WindowsApps\Microsoft.DesktopAppInstaller_1.23.1911.0_x64__8wekyb3d8bbwe;C:\Users\user\.cargo\bin;C:\Program Files\Git\cmd;C:\dev;C:\Program Files\CMake\bin;C:\Users\user\AppData\Local\Microsoft\WinGet;C:\Program Files\Mullvad VPN\resources;C:\dev\maven\bin;C:\dev\nim\bin;C:\dev\nim;C:\dev\nim\dist\mingw64\bin;C:\dev\zig;C:\dev\zls;C:\Users\user\Projects\PathHunter\bin;C:\Oops\Someone\Left\A\Stale\Target;]

Missing Path targets:
 [*] C:\Oops\Someone\Left\A\Stale\Target
```

## Buid instructions using a UNIX environment or PowerShell:
1. `cargo build --release`

## FAQ:
* Why not just use some version analogue of `getenv("Path")`?

    This type of functionality fetches the environment variable as it exists in the environment. Many shells inject additional dependencies into the `Path` variable. Fetching from the Windows registry keys ensures we are only checking user defined Path entries. 