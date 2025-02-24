mod cli;
mod path;
mod reg;

fn main() {
    let cli_flags = cli::parse_command_line_flags();

    let print_type = if cli_flags.contains(&("-b".to_owned())) {
        path::PrintType::Base
    } else {
        path::PrintType::Cleaned
    };

    let reg_type = if cli_flags.contains(&("-s".to_owned())) {
        reg::RegistryType::Sys
    } else {
        reg::RegistryType::User
    };

    let mut path = path::PathEnvVar::new(print_type, reg_type);

    path.print();
    path.validate();

    if cli_flags.contains(&("-w".to_owned())) {
        path.write_clean_path();
    }
}
