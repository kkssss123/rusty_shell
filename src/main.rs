use rusty_shell::interactive;
use std::process;

fn main() {
    // let args = Args::parse();

    // if let Err(e) = rusty_shell::run(args) {
    //     eprintln!("Error while running process: {e}");
    //     process::exit(1);
    // }
    if let Err(e) = interactive::run() {
        eprintln!("Error while running process: {e}");
        process::exit(1);
    }
}
