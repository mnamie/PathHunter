mod args;
mod audit;
mod clean;
mod display;
mod expandvars;
mod source;
mod winenv;

use std::io::{self, BufWriter, IsTerminal, Write};
use std::process::ExitCode;

use args::Command;

fn run() -> io::Result<u8> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = args::parse(&argv);

    if cfg.command == Command::Help {
        let mut out = io::stdout().lock();
        args::print_help(&mut out)?;
        out.flush()?;
        return Ok(0);
    }

    // Color/TTY detection
    if !cfg.no_color {
        let color_ok = if cfg!(windows) {
            winenv::setup_console()
        } else {
            io::stdout().is_terminal()
        };
        if !color_ok {
            cfg.no_color = true;
        }
    }

    if cfg.command == Command::Clean {
        return clean::run();
    }

    let raw = std::env::var_os("PATH")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if raw.is_empty() {
        eprintln!("ph: PATH is not set or empty");
        return Ok(1);
    }

    let mut sm = source::SourceMap::default();
    sm.build();
    let entries = audit::scan(&sm, &raw);

    let mut out = BufWriter::new(io::stdout().lock());
    display::render(&mut out, &entries, &cfg)?;
    out.flush()?;
    Ok(0)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        // A closed pipe (e.g. `ph | head`) is not an error worth reporting.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ph: error: {e}");
            ExitCode::FAILURE
        }
    }
}
