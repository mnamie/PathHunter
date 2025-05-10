use std::{env, process::exit};

use crate::reg;
use crate::path;

const ALLOWED_FLAGS: [&'static str; 4] = ["-b", "-w", "-s", "-h"];

/// Parses command line arguments
pub fn parse_command_line_flags() -> Vec<String> {
    let args: Vec<String> = env::args().collect();
    for arg in &args[1..] {
        if arg == "-h" {
            println!("Usage: ph [FLAGS]\n\nFLAGS:\n\t-h\tPrint usage\n\t-b\tPrint pre-populated Path (default is populated Path)\n\t-w\tWrite cleaned Path to registry (default is display only)\n\t-s\tPrint System Path (default is User Path; requires admin)");
            exit(0);
        }
        if !ALLOWED_FLAGS.contains(&(*arg).as_str()) {
            println!("Flag {arg} not supported");
            exit(1);
        }
    }
    args
}

// Interpret command line flags passed at runtime
pub fn interpret_command_line_flags(
    cli_flags: &Vec<String>, 
    reg_type: &mut Option<reg::RegistryType>, 
    print_type: &mut Option<path::PrintType>
) {
    for flag in cli_flags {
        match flag.as_str() {
            "-b" => {
                *print_type = Some(path::PrintType::Base);
            },
            "-s" => {
                *reg_type = Some(reg::RegistryType::Sys);
            },
            _ => {
                *print_type = Some(path::PrintType::Cleaned);
                *reg_type = Some(reg::RegistryType::User)
            }
        }
    }
}