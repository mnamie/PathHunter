use std::{env, process::exit};

use crate::reg;
use crate::path;

const ALLOWED_FLAGS: [&str; 4] = ["-b", "-w", "-s", "-h"];

/// Holds the state of CLI options passed into the program
pub struct CLI {
    pub registry_type: reg::RegistryType,
    pub print_type: path::PrintType,
    pub write: bool,
}

impl CLI {
    /// Parse command line flags into the CLI object for use downstream
    pub fn parse_command_line_flags() -> Self {
        let args: Vec<String> = env::args().collect();
        for arg in &args[1..] {
            if arg == "-h" {
                println!("Usage: ph [FLAGS]\n\nFLAGS:\n\t-h\tPrint usage\n\t-b\tPrint raw Path (default is populated Path)\n\t-w\tWrite cleaned Path to registry (default is display only)\n\t-s\tPrint System Path (default is User Path; requires admin)");
                exit(0);
            }
            if !ALLOWED_FLAGS.contains(&arg.as_str()) {
                println!("Flag {arg} not supported");
                exit(1);
            }
        }

        let mut reg_type: reg::RegistryType = reg::RegistryType::User;
        let mut print_type: path::PrintType = path::PrintType::Cleaned;

        for flag in &args {
            match flag.as_str() {
                "-b" => {
                    print_type = path::PrintType::Base;
                },
                "-s" => {
                    reg_type = reg::RegistryType::Sys;
                },
                _ => {}
            }
        }

        CLI { 
            registry_type: reg_type, 
            print_type: print_type, 
            write: args.iter().any(|a| a == "-w")
        }
    }
}
