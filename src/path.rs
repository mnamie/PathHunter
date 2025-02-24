use crate::reg;

#[derive(Debug, Clone, Copy)]
pub enum PrintType {
    Base,
    Cleaned,
}

#[derive(Debug, Clone)]
pub struct PathEnvVar {
    vec: Vec<String>,
    cleaned_vec: Vec<String>,
    new_vec: Vec<String>,
    print_type: PrintType,
    reg_type: reg::RegistryType,
}

impl PathEnvVar {
    pub fn new(print_type: PrintType, reg_type: reg::RegistryType) -> Self {
        let path_str = reg::fetch_path_string(reg_type);
        let path_vec = split_path_string_to_vec(&path_str);
        let cleaned_vec = clean_path_vec(&path_vec);
        PathEnvVar {
            vec: path_vec,
            cleaned_vec: cleaned_vec,
            new_vec: vec![],
            print_type: print_type,
            reg_type: reg_type,
        }
    }

    pub fn print(self: &Self) {
        let iter_target = match self.print_type {
            PrintType::Base => &self.vec,
            PrintType::Cleaned => &self.cleaned_vec,
        };
        println!("\nPATH: [");
        for path in iter_target.iter() {
            println!(" {}", path);
        }
        println!("]");
    }

    pub fn validate(self: &mut Self) {
        let mut all_clear: bool = true;
        println!("\nMissing path targets:");
        for path in self.cleaned_vec.clone() {
            if !std::fs::metadata(&path).is_ok() {
                all_clear = false;
                println!(" [*] {}", path);
            } else {
                self.new_vec.push(path);
            };
        }
        if all_clear {
            println!(" [*] All clear")
        }
    }

    pub fn write_clean_path(&self) {
        reg::set_path_string(self.reg_type, &self.new_vec.join(";"));
    }
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
