# rusty-shell

A small command-line shell written in Rust.

## Features

- Launches a shell session.
- Recursively renames supported media files using sequential numbers.
- Preserves the original directory structure.
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

Once the shell starts, run the `rn` command:

```bash
rn [FROM] [OPTIONS]
```

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
| `-v, --verbose` | Display detailed output |
| `-p, --prefix <PREFIX>` | Add a prefix to filenames |
| `-s, --suffix <SUFFIX>` | Add a suffix to filenames |
| `--reverse` | Sort filenames in descending order |
| `--sort-by-mtime` | Sort by modification time in ascending order |
| `--sort-by-mtime-desc` | Sort by modification time in descending order |
| `-h, --help` | Print help information |

By default, files are sorted by filename in ascending order. Files with the same modification time are sorted by filename in ascending order.

### Examples

```bash
rn
rn ./photos
rn ./photos --to ./renamed
rn ./photos --first 100
rn ./photos --prefix vacation_
rn ./photos --suffix _edited
rn ./photos --reverse
rn ./photos --sort-by-mtime
rn ./photos --sort-by-mtime-desc
```

## Development

Run tests:

```bash
cargo test
```

Build the project:

```bash
cargo build
```
