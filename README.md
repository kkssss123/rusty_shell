# rusty-shell

A small interactive command-line shell written in Rust. It provides built-in
commands and delegates other input to a selected system shell.

## Features

- Interactive prompt with command history.
- Built-in `cd`, `exit`, and `rn` commands.
- Delegates external commands to `cmd`, `sh`, PowerShell, or another selected shell.
- Supports shell-local working directories for built-in and external commands.
- Recursively renames supported media files using sequential numbers.
- Preserves the original directory structure.
- Processes subdirectories in parallel.
- Supports sorting by filename or modification time.
- Supports custom filename prefixes and suffixes.
- Supports in-place renaming and renaming to a destination directory.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (install using `rustup`)

Verify the installation:

```bash
rustc --version
cargo --version
```

## Installation

Clone the repository and build the project:

```bash
git clone https://github.com/kkssss123/rusty_shell.git
cd rusty_shell
cargo build --release
```

## Usage

Start the shell:

```bash
cargo run --release
```

Use `-s` or `--shell` to select the system shell used for external commands:

```bash
cargo run --release -- --shell powershell
cargo run --release -- -s pwsh
```

Without this option, rusty-shell uses `cmd` on Windows and `sh` on other platforms.

Once the shell starts, built-in and external commands can be mixed:

```text
> cd ./photos
> rn --prefix IMG_
> git status
> exit
```

### Built-in commands

| Command | Description |
|---|---|
| `cd [PATH]` | Changes rusty-shell's working directory. If `PATH` is omitted or `~`, moves to the home directory. Supports `~/...` expansion. |
| `rn [FROM] [OPTIONS]` | Recursively renames supported media files. |
| `exit` | Exits rusty-shell. |

`cd` changes the working directory maintained by rusty-shell. Running `cd` without arguments or with `~` moves to the home directory, and `~/...` (or `~\...`) resolves relative to the home directory. Subsequent external commands and relative `rn` paths use that directory as their working directory.

### External commands

Input that is not a built-in command is passed unchanged to the selected system shell.
This allows the selected shell to handle pipes, redirection, and conditional execution.

```text
> dir | findstr ".rs"
> cargo test > test.log
> git status && cargo test
```

The command invocation option depends on the selected shell:

| Shell | Invocation |
|---|---|
| `cmd`, `cmd.exe` | `/C <command>` |
| `powershell`, `pwsh` | `-Command <command>` |
| Other shells, such as `sh` and `bash` | `-c <command>` |

External-command syntax is interpreted by the selected shell. Built-in command
arguments are parsed by rusty-shell.

### `rn` command

Use the `rn` command to recursively rename supported media files:

```text
rn [FROM] [OPTIONS]
```

`FROM` defaults to the current directory (`.`).

### Supported file extensions

- `jpg`
- `jpeg`
- `webp`
- `png`
- `mp4`

### Options

| Option | Description |
|---|---|
| `--to <TO>` | Destination directory |
| `-f, --first <FIRST>` | Starting number (default: 1) |
| `-v, --verbose` | Display each source and destination path |
| `-p, --prefix <PREFIX>` | Add a prefix to filenames |
| `-s, --suffix <SUFFIX>` | Add a suffix to filenames |
| `--reverse` | Sort filenames in descending order |
| `--sort-by-mtime` | Sort by modification time in ascending order |
| `--sort-by-mtime-desc` | Sort by modification time in descending order |

By default, files are sorted by filename in ascending order. Files with the same
modification time are sorted by filename in ascending order. Numbering restarts in
each directory.

Verbose output from different subdirectories may appear in
an unpredictable order because subdirectories are processed in parallel.

### Examples

```text
> rn
> rn ./photos
> rn ./photos --to ./renamed
> rn ./photos --first 100
> rn ./photos --prefix vacation_
> rn ./photos --suffix _edited
> rn ./photos --reverse
> rn ./photos --sort-by-mtime
> rn ./photos --sort-by-mtime-desc
```

When `--to` specifies a different destination directory,
existing files with matching names may be overwritten.

### Error handling and recovery

If an error occurs during renaming, the backup directory under `FROM`
is retained to help recover files.

Depending on when the error occurs, some files may already have been
moved to `TO`, while others remain in the backup directory.
Check both locations when recovering from a failed operation.

The backup directory is removed after successful completion.

## History

Command history is saved across sessions to `~/.rusty_shell_history` in the user's home directory.

## Development

Run tests:

```bash
cargo test
```

Build the project:

```bash
cargo build
```

Run benchmarks:

```bash
cargo bench
```
