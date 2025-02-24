# PathHunter

PathHunter is a quick Windows utility I wrote to check which targets in my Path environment variable no longer exist, and to quickly remove them, keeping my Path clean. Also removes duplicate Path entries, should they exist.

## Buid instructions:

1. `cargo build --release`

## Usage

```bash
Usage: ph [FLAGS]

FLAGS:
        -h      Print usage
        -b      Print pre-populated Path (default is populated Path)
        -w      Write cleaned Path to registry (default is display only)
        -s      Print System Path (default is user Path; requires admin)
```