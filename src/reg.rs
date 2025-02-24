use winreg::enums::HKEY_CURRENT_USER;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::enums::KEY_ALL_ACCESS;
use winreg::RegKey;

const SYSTEM_KEY: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";
const USER_KEY: &str = "Environment";

/// Registry type can be either User or Sys, indicating whether to reference
/// the system or user level environment variables
#[derive(Debug, Clone, Copy)]
pub enum RegistryType {
    User,
    Sys,
}

/// Fetches the registry subkey based on the provided RegistryType
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

/// Fetches the Path environment variables as a String
pub fn fetch_path_string(registry_type: RegistryType) -> String {
    registry_sub_key_fetch(registry_type)
        .get_value("Path")
        .expect("Error: Unable to fetch registry key")
}

/// Sets the Path environment variable from an &String
pub fn set_path_string(registry_type: RegistryType, target_string: &String) {
    registry_sub_key_fetch(registry_type)
        .set_value("Path", target_string)
        .expect("Error: Unable to write registry")
}
