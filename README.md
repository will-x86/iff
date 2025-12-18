# iff

A terminal tool to quickly find and run commands from your shell history.

## What it does

Search through your bash/zsh history and re-run commands without retyping them.
```bash
$ iff docker
# Shows all docker commands you've run
# Navigate with arrows or j/k, press Enter to run
```

## Installation
```bash
cargo install ifff
```

## Usage
```bash
# Search with initial filter
iff docker

# Browse all history
iff

# Navigate with arrow keys or j/k
# Press Enter to run selected command
# Press q or Esc to quit
```

## Requirements

- Rust/Cargo (for installation)
- bash or zsh shell history

## License

MIT
