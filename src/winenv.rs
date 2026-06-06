//! Windows-only registry, console, and broadcast primitives. Rust's stdlib
//! has no registry/user32 bindings, so this declares raw extern calls into
//! advapi32.dll/user32.dll/shell32.dll/kernel32.dll directly rather than
//! pulling in a bindings crate. Everything is inside a `#[cfg(windows)]`
//! module, so none of it is compiled into non-Windows builds; the public
//! wrappers below fall back to inert stubs there.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    type Hkey = isize;
    type Handle = *mut c_void;

    const HKEY_CURRENT_USER: Hkey = 0x80000001u32 as i32 as Hkey;
    const HKEY_LOCAL_MACHINE: Hkey = 0x80000002u32 as i32 as Hkey;
    const KEY_READ: u32 = 0x20019;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_EXPAND_SZ: u32 = 2;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_MORE_DATA: i32 = 234;

    const SYSTEM_ENV_SUBKEY: &str =
        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";
    const USER_ENV_SUBKEY: &str = "Environment";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            hKey: Hkey,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut Hkey,
        ) -> i32;
        fn RegQueryValueExW(
            hKey: Hkey,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            hKey: Hkey,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> i32;
        fn RegCloseKey(hKey: Hkey) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SendMessageTimeoutW(
            hWnd: isize,
            Msg: u32,
            wParam: usize,
            lParam: *const u16,
            fuFlags: u32,
            uTimeout: u32,
            lpdwResult: *mut usize,
        ) -> isize;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn IsUserAnAdmin() -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
        fn SetConsoleCP(wCodePageID: u32) -> i32;
        fn GetStdHandle(nStdHandle: u32) -> Handle;
        fn GetConsoleMode(hConsoleHandle: Handle, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: Handle, dwMode: u32) -> i32;
    }

    /// NUL-terminated UTF-16 copy of `s`.
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Owned registry key, closed on drop.
    struct Key(Hkey);

    impl Drop for Key {
        fn drop(&mut self) {
            unsafe { RegCloseKey(self.0) };
        }
    }

    fn open_key(hive: Hkey, subkey: &str, access: u32) -> Option<Key> {
        let subkey16 = wide(subkey);
        let mut result: Hkey = 0;
        let rc = unsafe { RegOpenKeyExW(hive, subkey16.as_ptr(), 0, access, &mut result) };
        (rc == ERROR_SUCCESS).then_some(Key(result))
    }

    fn query_string_value(key: &Key, name: &str) -> Option<String> {
        let name16 = wide(name);
        let mut capacity: u32 = 2048; // in u16 units
        for _ in 0..8 {
            let mut buf = vec![0u16; capacity as usize];
            let mut size: u32 = capacity * 2; // bytes
            let rc = unsafe {
                RegQueryValueExW(
                    key.0,
                    name16.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    buf.as_mut_ptr().cast(),
                    &mut size,
                )
            };
            if rc == ERROR_MORE_DATA {
                capacity *= 2;
                continue;
            }
            if rc != ERROR_SUCCESS {
                return None;
            }
            let mut len = size as usize / 2;
            while len > 0 && buf[len - 1] == 0 {
                len -= 1;
            }
            return Some(String::from_utf16_lossy(&buf[..len]));
        }
        None
    }

    fn set_expand_string_value(key: &Key, name: &str, value: &str) -> bool {
        let name16 = wide(name);
        let value16 = wide(value); // includes the terminating NUL
        let byte_len = (value16.len() * 2) as u32;
        let rc = unsafe {
            RegSetValueExW(
                key.0,
                name16.as_ptr(),
                0,
                REG_EXPAND_SZ,
                value16.as_ptr().cast(),
                byte_len,
            )
        };
        rc == ERROR_SUCCESS
    }

    fn read_path_value_raw(hive: Hkey, subkey: &str) -> Option<String> {
        let key = open_key(hive, subkey, KEY_READ)?;
        query_string_value(&key, "Path")
    }

    fn write_path_value_raw(hive: Hkey, subkey: &str, value: &str) -> bool {
        match open_key(hive, subkey, KEY_READ | KEY_SET_VALUE) {
            Some(key) => set_expand_string_value(&key, "Path", value),
            None => false,
        }
    }

    fn split_segments(raw: &str) -> Vec<String> {
        raw.split(';')
            .map(|p| p.trim_matches([' ', '\t', '\r', '\n']))
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub fn is_admin() -> bool {
        unsafe { IsUserAnAdmin() != 0 }
    }

    pub fn read_raw_user_path_segments() -> Vec<String> {
        read_path_value_raw(HKEY_CURRENT_USER, USER_ENV_SUBKEY)
            .map(|raw| split_segments(&raw))
            .unwrap_or_default()
    }

    pub fn write_user_path_to_registry(segs: &[String]) -> bool {
        write_path_value_raw(HKEY_CURRENT_USER, USER_ENV_SUBKEY, &segs.join(";"))
    }

    pub fn read_raw_system_path_segments() -> Vec<String> {
        read_path_value_raw(HKEY_LOCAL_MACHINE, SYSTEM_ENV_SUBKEY)
            .map(|raw| split_segments(&raw))
            .unwrap_or_default()
    }

    pub fn write_system_path_to_registry(segs: &[String]) -> bool {
        write_path_value_raw(HKEY_LOCAL_MACHINE, SYSTEM_ENV_SUBKEY, &segs.join(";"))
    }

    pub fn broadcast_env_change() {
        let payload = wide("Environment");
        const HWND_BROADCAST: isize = 0xffff;
        const WM_SETTINGCHANGE: u32 = 0x001a;
        const SMTO_ABORTIFHUNG: u32 = 0x0002;
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                payload.as_ptr(),
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            );
        }
    }

    pub fn setup_console() -> bool {
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
        unsafe {
            SetConsoleOutputCP(65001);
            SetConsoleCP(65001);
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle == INVALID_HANDLE_VALUE {
                return false;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return false;
            }
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
        true
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn is_admin() -> bool {
        false
    }
    pub fn read_raw_user_path_segments() -> Vec<String> {
        Vec::new()
    }
    pub fn write_user_path_to_registry(_: &[String]) -> bool {
        false
    }
    pub fn read_raw_system_path_segments() -> Vec<String> {
        Vec::new()
    }
    pub fn write_system_path_to_registry(_: &[String]) -> bool {
        false
    }
    pub fn broadcast_env_change() {}
    pub fn setup_console() -> bool {
        false
    }
}

pub use imp::*;
