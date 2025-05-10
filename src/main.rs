mod cli;
mod path;
mod reg;

fn main() {
    // Parse command line flags
    let cli_flags = cli::parse_command_line_flags();

    // Interpret command line flags to runtime options
    let mut reg_type: Option<reg::RegistryType> = None;
    let mut print_type: Option<path::PrintType> = None;
    cli::interpret_command_line_flags(&cli_flags, &mut reg_type, &mut print_type);

    // Initialize path struct with options
    let mut path = path::PathEnvVar::new(print_type.unwrap(), reg_type.unwrap());

    // Print and validate
    path.print();
    path.validate();

    // Optionally; write a cleaned path when `-w` flag passed
    if cli_flags.contains(&String::from("-w")) {
        path.write_clean_path();
    }
}
