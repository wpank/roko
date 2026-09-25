# hello-color

## About

`hello-color` is a minimal Rust binary that demonstrates terminal color output using the `colored` crate. When run, it prints a bright-magenta double-line border above and below a bold green `Hello, World!` greeting, producing a visually distinct framed message in any ANSI-capable terminal.

## Usage

```bash
cargo run --manifest-path demo/hello-color/Cargo.toml
```

## Dependencies

This crate uses [colored](https://crates.io/crates/colored) for terminal color output.

## License

MIT OR Apache-2.0
