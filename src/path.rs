use winreg::enums::HKEY_LOCAL_MACHINE;

#[derive(Debug)]
pub enum PrintType {
    Base,
    Cleaned,
}

#[derive(Debug)]
pub struct PathEnvVar {
    str: String,
    vec: Vec<String>,
    cleaned_vec: Vec<String>,
    print_type: PrintType,
}

impl PathEnvVar {
    pub fn new(print_type: PrintType) -> Self {
        let path_str = fetch_path_string();
        let path_vec = split_path_string_to_vec(&path_str);
        let cleaned_vec = clean_path_vec(&path_vec);
        PathEnvVar {
            str: path_str,
            vec: path_vec,
            cleaned_vec: cleaned_vec,
            print_type: print_type,
        }
    }

    pub fn print(self: &Self) {
        println!("Path: [");
        match self.print_type {
            PrintType::Base => {
                for path in self.vec.clone() {
                    println!(" {}", path);
                }
            },
            PrintType::Cleaned => {
                for path in self.cleaned_vec.clone() {
                    println!(" {}", path);
                }
            },
        }
        println!("]");
    }

    pub fn validate(self: &Self) {
        let mut all_clear: bool = true;
        println!("Missing path targets:");
        for path in self.cleaned_vec.clone() {
            if !std::fs::metadata(&path).is_ok() {
                all_clear = false;
                println!(" [*] {}", path);
            };
        }
        if all_clear {println!(" [*] All clear")}
    }
}

fn fetch_path_string() -> String {
    winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
        .expect("Error: Unable to fetch registry subkey")
        .get_value("Path")
        .expect("Error: Unable to fetch registry path key")
}

fn split_path_string_to_vec(path_str: &str) -> Vec<String> {
    path_str
        .split(";")
        .filter(|s| *s != "")
        .map(|s| s.to_owned())
        .collect()
}

fn clean_path_vec(path_vec: &Vec<String>) -> Vec<String> {
    let mut res: Vec<String> = vec![];
    for path in path_vec.clone().iter_mut() {
        let mut cleaned_str: String = path.to_owned();
        for (key, value) in std::env::vars() {
            cleaned_str = cleaned_str.replace(&format!("%{}%", key.to_uppercase()), &value);
        }
        res.push(cleaned_str);
    }
    res
}