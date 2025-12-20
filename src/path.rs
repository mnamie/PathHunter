use crate::reg;

/// PrintType denotes whether the clean or base Path should be printed
#[derive(Debug, Clone, Copy)]
pub enum PrintType {
    Base,
    Cleaned,
}

/// PathEnvVar contains the various versions of the Path environment variable
/// that we want to use for displaying and/or writing.
#[derive(Debug, Clone)]
pub struct PathEnvVar {
    vec: Vec<String>,
    cleaned_vec: Vec<String>,
    new_vec: Vec<String>,
    print_type: PrintType,
    reg_type: reg::RegistryType,
    is_clean: bool,
}

impl PathEnvVar {
    /// Create a new instance of PathEnvVar based on PrintType and RegistryType
    pub fn new(print_type: PrintType, reg_type: reg::RegistryType) -> Self {
        let path_str = reg::fetch_path_string(reg_type);
        let path_vec = split_path_string_to_vec(&path_str);
        let cleaned_vec = clean_path_vec(&path_vec);
        PathEnvVar {
            vec: path_vec,
            cleaned_vec,
            new_vec: vec![],
            print_type,
            reg_type,
            is_clean: true,
        }
    }

    /// Print the Path, as dicatated by the configured PrintType
    pub fn print(&self) {
        let iter_target = match self.print_type {
            PrintType::Base => &self.vec,
            PrintType::Cleaned => &self.cleaned_vec,
        };
        println!("\nPath: [");
        for path in iter_target.iter() {
            println!(" {}", path);
        }
        println!("]");
    }

    /// Validate the Path for missing path targets
    pub fn validate(&mut self) {
        println!("\nMissing path targets:");
        for path in self.cleaned_vec.clone() {
            if std::fs::metadata(&path).is_err() {
                self.is_clean = false;
                println!(" [*] {}", path);
            } else if !self.new_vec.contains(&path) {
                self.new_vec.push(path);
            };
        }
        if self.is_clean {
            println!(" [*] All clear")
        }
    }

    /// After removing dead links and duplicates, write the corrected Path
    pub fn write_clean_path(&self) {
        if !self.is_clean {
            reg::set_path_string(self.reg_type, &self.new_vec.join(";"));
            println!("\n[*] Path has been cleaned");
        }
    }
}

fn split_path_string_to_vec(path_str: &str) -> Vec<String> {
    path_str
        .split(";")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

fn clean_path_vec(path_vec: &Vec<String>) -> Vec<String> {
    let mut res: Vec<String> = vec![];
    for path in path_vec {
        let mut cleaned_str: String = path.to_owned();
        for (key, value) in std::env::vars() {
            cleaned_str = path.replace(&format!("%{}%", key.to_uppercase()), &value);
        }
        res.push(cleaned_str);
    }
    res
}
