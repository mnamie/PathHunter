
pub fn split_path_string_to_vec(path_str: &str) -> Vec<String> {
    path_str
        .split(";")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

pub fn clean_path_vec(path_vec: &Vec<String>) -> Vec<String> {
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
