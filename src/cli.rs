use std::{env, process::exit};

const ALLOWED_FLAGS: [&str; 4] = ["-b", "-w", "-s", "-h"];

pub fn parse_command_line_flags() -> Vec<String> {
    let args: Vec<String> = env::args().collect();
    for arg in &args[1..] {
        if arg == "-h" {
            println!("Usage: ph [FLAGS]\n\nFLAGS:\n\t-h\tPrint usage\n\t-b\tPrint pre-populated Path (default is populated Path)\n\t-w\tWrite cleaned Path to registry (default is display only)\n\t-s\tPrint System Path (default is User Path)");
            exit(0);
        }
        if !ALLOWED_FLAGS.contains(&(*arg).as_str()) {
            println!("Flag {arg} not supported");
            exit(1);
        }
    }
    args
}
