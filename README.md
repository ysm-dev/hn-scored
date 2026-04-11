# hn-scored

Static Hacker News RSS, Atom, and JSON feeds filtered by score threshold.

## Usage

```bash
cargo run -- --state ./state.json --output ./dist
```

Options:

- `--state <PATH>`: path to `state.json`
- `--output <PATH>`: output directory for generated feeds
- `--base-url <URL>`: absolute base URL for self-referencing feed links

## Development

```bash
cargo fmt --all
cargo clippy -- -D warnings
cargo test
```
