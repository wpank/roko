# Roko Code Conventions

Follow these patterns when adding code to any crate.

## Doc comments

```rust
/// Short one-line summary.
///
/// Longer description if needed. Reference other types with [`TypeName`].
///
/// # Examples
///
/// ```no_run
/// let x = TypeName::new();
/// ```
pub struct TypeName { ... }
```

## Module wiring

Every new file must be declared in its parent `mod.rs`:

```rust
// In mod.rs
pub mod new_module;
```

Every public type in a submodule should be re-exported from the crate root
or the parent module's `mod.rs`:

```rust
pub use self::new_module::NewType;
```

## Error handling

- Use `thiserror::Error` for error enums
- Propagate with `?`, never `unwrap()` on hot paths
- `unwrap()` is acceptable only in tests and infallible paths (e.g., regex compilation)

## Test patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_does_the_thing() {
        // arrange
        // act
        // assert
    }
}
```

## Naming

- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE`
- Crate names: `roko-*` (kebab-case)
- Module names: `snake_case`

## Imports

Prefer absolute crate paths for cross-crate imports:

```rust
use roko_core::signal::Signal;
use roko_agent::provider::ProviderRegistry;
```
