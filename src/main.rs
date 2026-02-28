mod cli;
mod path;
mod reg;
mod utils;

use cli::CLI;

fn main() {
    // Parse command line flags
    let cli = CLI::parse_command_line_flags();

    // Initialize path struct with options
    let mut path = path::PathEnvVar::new(&cli);

    // Print and validate
    path.print();
    path.validate();

    // Optionally; write a cleaned path when `-w` flag passed
    if cli.write {
        path.write_clean_path();
    }
}
