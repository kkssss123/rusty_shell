use clap::Parser;
use rusty_shell::Args;
use std::process;

fn main() {
    let args = Args::parse();

    if let Err(e) = rusty_shell::run(args) {
        eprintln!("Error while running process: {e}");
        process::exit(1);
    }
}
