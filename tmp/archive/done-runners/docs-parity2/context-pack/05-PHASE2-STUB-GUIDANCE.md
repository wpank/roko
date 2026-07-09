# Phase 2+ Stub Guidance

Sections 08-chain, 09-daimon, 10-dreams, and 14-identity-economy describe
functionality planned for Phase 2+. The batches for these sections create
**stubs only** — type shells with doc comments, not full implementations.

## What a stub looks like

```rust
/// Represents a witness event observed on-chain.
///
/// Phase 2+: Will track block number, transaction hash, and decoded event
/// data for on-chain verification workflows.
#[derive(Debug, Clone)]
pub struct ChainWitness {
    /// The chain ID this witness observed.
    pub chain_id: u64,
    /// Block number of the witnessed event.
    pub block_number: u64,
    /// Human-readable description of what was witnessed.
    pub description: String,
}

impl ChainWitness {
    /// Create a new chain witness record.
    pub fn new(chain_id: u64, block_number: u64, description: impl Into<String>) -> Self {
        Self {
            chain_id,
            block_number,
            description: description.into(),
        }
    }
}
```

## Rules for stubs

1. **Struct fields should be real.** Use concrete types from the doc descriptions.
   Don't use `()` or `PhantomData` unless the doc explicitly says the type is generic.

2. **Constructor methods can be real.** Simple `new()` and accessor methods are fine.

3. **Complex logic gets `todo!()`.** Methods that require external state, async I/O,
   or multi-step algorithms should have `todo!("Phase 2+: <what this does>")`.

4. **Trait impls use defaults or todo.** If a trait method has a sensible default,
   use it. Otherwise `todo!()`.

5. **No external deps.** Don't add new crate dependencies. Use types already available.

6. **`#[allow(dead_code)]` is fine.** Since stubs won't be called yet, suppress
   dead-code warnings at the module level if needed.

7. **Wire into mod.rs.** Even stubs should be reachable from the crate root.

## What NOT to stub

- Don't create empty files with just `// TODO`
- Don't stub private helper functions
- Don't stub test utilities
- Don't create integration tests for stub functionality
