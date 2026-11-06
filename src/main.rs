use clap::Parser;
use rusty_shell::interactive;
use std::process;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    shell: Option<String>,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = interactive::run(args.shell) {
        eprintln!("Error while running process: {e}");
        process::exit(1);
    }
}
