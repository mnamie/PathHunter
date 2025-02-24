mod cli;
mod path;
mod reg;

fn main() {
    let cli_flags = cli::parse_command_line_flags();

    let print_type = if cli_flags.contains(&String::from("-b")) {
        path::PrintType::Base
    } else {
        path::PrintType::Cleaned
    };

    let reg_type = if cli_flags.contains(&String::from("-s")) {
        reg::RegistryType::Sys
    } else {
        reg::RegistryType::User
    };

    let mut path = path::PathEnvVar::new(print_type, reg_type);

    path.print();
    path.validate();

    if cli_flags.contains(&String::from("-w")) {
        path.write_clean_path();
    }
}
