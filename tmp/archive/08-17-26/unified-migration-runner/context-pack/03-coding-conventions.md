# Rust Coding Conventions

## Style

- Run `cargo +nightly fmt --all` before committing
- All code must pass `cargo clippy --workspace --no-deps -- -D warnings`
- Prefer `Result<T, E>` over `unwrap()` in library code
- Use `thiserror` for error types, `anyhow` only in CLI/binary code

## Error handling

- Library crates: define error enums with `#[derive(thiserror::Error)]`
- Binary crate (roko-cli): use `anyhow::Result` at the top level
- Never use `.unwrap()` on hot paths or in library code
- Use `.expect("reason")` only for truly impossible cases with a clear message

## Naming

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`
- Feature flags: `kebab-case`

## Testing

- Unit tests in the same file: `#[cfg(test)] mod tests { ... }`
- Integration tests in `crates/<name>/tests/`
- Use `#[tokio::test]` for async tests
- Name test functions: `test_<what_it_tests>`
- Test only what you changed — don't add tests for existing unchanged code

## Import conventions

- Group imports: std, external crates, internal crates, current crate
- Use `use crate::` for intra-crate imports
- Prefer specific imports over glob imports

## Documentation

- Only add doc comments to new public APIs you create
- Don't add doc comments to code you're just renaming
- Use `///` for public items, `//` for implementation notes
