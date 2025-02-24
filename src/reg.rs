use winreg::enums::HKEY_CURRENT_USER;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::enums::KEY_ALL_ACCESS;
use winreg::RegKey;

const SYSTEM_KEY: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";
const USER_KEY: &str = "Environment";

#[derive(Debug, Clone, Copy)]
pub enum RegistryType {
    User,
    Sys,
}

fn registry_sub_key_fetch(registry_type: RegistryType) -> RegKey {
    winreg::RegKey::predef(match registry_type {
        RegistryType::Sys => HKEY_LOCAL_MACHINE,
        RegistryType::User => HKEY_CURRENT_USER,
    })
    .open_subkey_with_flags(
        match registry_type {
            RegistryType::Sys => SYSTEM_KEY,
            RegistryType::User => USER_KEY,
        },
        KEY_ALL_ACCESS,
    )
    .expect("Error: Unable to fetch registry subkey")
}

pub fn fetch_path_string(registry_type: RegistryType) -> String {
    registry_sub_key_fetch(registry_type)
        .get_value("Path")
        .expect("Error: Unable to fetch registry path key")
}

pub fn set_path_string(registry_type: RegistryType, target_string: &String) {
    registry_sub_key_fetch(registry_type)
        .set_value("Path", target_string)
        .expect("Error: Unable to write registry")
}
