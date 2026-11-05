use clap::Parser;
use std::error::Error;
use std::process::Command;

pub mod commands;
pub mod interactive;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    shell: Option<String>,
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let shell = resolve(args.shell);

    let status = Command::new(shell).status()?;

    if !status.success() {
        return Err("Shell execution failed".into());
    }

    Ok(())
}

fn resolve(shell: Option<String>) -> String {
    shell.unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            String::from("cmd")
        } else {
            String::from("sh")
        }
    })
}
