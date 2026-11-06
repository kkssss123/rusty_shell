use crate::{commands, commands::BuiltinCommand, shell::ShellOutcome::*};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub(crate) enum ShellOutcome {
    Continue,
    Exit,
}

pub(crate) struct Shell {
    cwd: PathBuf,
    program: String,
}

impl Shell {
    pub(crate) fn new(program: Option<String>) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let program = program.unwrap_or_else(default_shell_program);
        Ok(Self { cwd, program })
    }

    pub(crate) fn execute(&mut self, line: &str) -> Result<ShellOutcome> {
        if line.trim().is_empty() {
            return Ok(Continue);
        }

        match commands::parse_builtin(line)? {
            Some(BuiltinCommand::Cd(path)) => {
                self.change_directory(&path)?;
                Ok(Continue)
            }
            Some(BuiltinCommand::Exit) => Ok(Exit),
            Some(BuiltinCommand::Rn(args)) => {
                commands::rn::execute(args, &self.cwd)?;
                Ok(Continue)
            }
            None => {
                self.run_in_system_shell(line)?;
                Ok(Continue)
            }
        }
    }

    fn run_in_system_shell(&self, line: &str) -> Result<()> {
        Command::new(&self.program)
            .args([shell_command_option(&self.program), line])
            .current_dir(&self.cwd)
            .status()?;
        Ok(())
    }

    fn change_directory(&mut self, path: &Path) -> Result<()> {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        };
        let path = dunce::canonicalize(path)?;

        if !path.is_dir() {
            return Err(Error::new(
                ErrorKind::NotADirectory,
                format!("not a directory: {}", path.display()),
            ));
        }

        self.cwd = path;
        Ok(())
    }
}

fn default_shell_program() -> String {
    if cfg!(target_os = "windows") {
        "cmd".to_string()
    } else {
        "sh".to_string()
    }
}

fn shell_command_option(program: &str) -> &'static str {
    let program = program.rsplit(['/', '\\']).next().unwrap_or(program);

    if program.eq_ignore_ascii_case("cmd") || program.eq_ignore_ascii_case("cmd.exe") {
        "/C"
    } else if program.eq_ignore_ascii_case("powershell")
        || program.eq_ignore_ascii_case("powershell.exe")
        || program.eq_ignore_ascii_case("pwsh")
        || program.eq_ignore_ascii_case("pwsh.exe")
    {
        "-Command"
    } else {
        "-c"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn exit_returns_exit_outcome() -> Result<()> {
        let mut shell = Shell::new(None)?;

        let outcome = shell.execute("exit")?;

        assert!(matches!(outcome, ShellOutcome::Exit));
        Ok(())
    }

    #[test]
    fn blank_input_continues() -> Result<()> {
        let mut shell = Shell::new(None)?;

        let outcome = shell.execute("   ")?;

        assert!(matches!(outcome, ShellOutcome::Continue));
        Ok(())
    }

    #[test]
    fn malformed_builtin_syntax_is_rejected_before_execution() -> Result<()> {
        let mut shell = Shell::new(None)?;

        let error = shell.execute("rn \"unterminated").unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn cd_changes_the_shell_working_directory_without_changing_the_process_directory() -> Result<()>
    {
        let root = tempdir()?;
        let child = root.path().join("child");
        fs::create_dir(&child)?;
        let process_cwd = std::env::current_dir()?;
        let mut shell = shell_in(root.path(), "nothing");
        let child = child.to_string_lossy().replace('\\', "/");

        shell.execute(&format!("cd {child}"))?;

        assert_eq!(shell.cwd, dunce::canonicalize(root.path().join("child"))?);
        assert_eq!(std::env::current_dir()?, process_cwd);
        Ok(())
    }

    #[test]
    fn cd_without_path_or_with_tilde_changes_to_home_directory() -> Result<()> {
        let root = tempdir()?;
        let mut shell = shell_in(root.path(), "nothing");
        let home_dir = dunce::canonicalize(std::env::home_dir().expect("home dir should exist"))?;

        shell.execute("cd")?;
        assert_eq!(shell.cwd, home_dir);

        shell.cwd = dunce::canonicalize(root.path())?;
        shell.execute("cd ~")?;
        assert_eq!(shell.cwd, home_dir);
        Ok(())
    }

    #[test]
    fn cd_rejects_too_many_arguments() -> Result<()> {
        let mut shell = Shell::new(None)?;
        let error = shell.execute("cd foo bar").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn rn_uses_the_shell_working_directory_for_relative_paths() -> Result<()> {
        let root = tempdir()?;
        let path = root.path();
        fs::write(path.join("photo.jpg"), b"photo")?;
        let mut shell = shell_in(path, "nothing");

        shell.execute("rn")?;

        assert_eq!(fs::read(path.join("001.jpg"))?, b"photo");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn unknown_command_is_executed_by_cmd() -> Result<()> {
        let mut shell = Shell::new(Some(default_shell_program()))?;

        let outcome = shell.execute("echo rusty-shell-external-command-test > NUL")?;

        assert!(matches!(outcome, ShellOutcome::Continue));
        Ok(())
    }

    #[test]
    fn external_command_runs_in_the_shell_working_directory() -> Result<()> {
        let root = tempdir()?;
        let path = root.path();
        let mut shell = shell_in(path, default_shell_program().as_ref());

        shell.execute("echo rusty-shell-working-directory-test > location.txt")?;

        assert_eq!(
            fs::read_to_string(path.join("location.txt"))
                .unwrap()
                .trim(),
            "rusty-shell-working-directory-test"
        );
        Ok(())
    }

    fn shell_in(cwd: &Path, program: &str) -> Shell {
        Shell {
            cwd: dunce::canonicalize(cwd).expect("directory should exist"),
            program: program.to_string(),
        }
    }
}
