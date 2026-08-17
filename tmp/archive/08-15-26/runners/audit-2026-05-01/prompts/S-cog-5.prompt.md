# S-cog-5: Delete pheromones

## Task
Delete the pheromones crate (or pheromone modules within `roko-cognitive`). Per S-cog-1's inventory, pheromones have no production callers and ~68K LOC of net deletion.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-cog-1. Wave 4.

## Source plan
`tmp/subsystem-audits/implementation-plans/31-cognitive-layer-cleanup.md` § CL-3.

## Pre-deletion safety check (mandatory)

```bash
rg 'pheromone|Pheromone' crates/ -g '*.rs' \
  | rg -v 'crates/roko-cognitive/pheromone'
```

Each remaining hit:

- **Test code** in another crate that incidentally references pheromones: delete the test or update it.
- **Production caller**: stop and report. The inventory must be wrong.

If S-cog-1's inventory listed any pheromone caller as "needs replacement," that replacement should have been done before this batch. Check the inventory.

## Exact changes

### 1. Delete pheromone code

If pheromones live in `roko-cognitive`:

```bash
git rm -r crates/roko-cognitive/src/pheromone/
git rm -r crates/roko-cognitive/src/pheromones/   # alternate naming
git rm -r crates/roko-cognitive/src/pheromone.rs
```

If pheromones are their own crate (`roko-pheromones`):

```bash
git rm -r crates/roko-pheromones
```

### 2. Update `lib.rs`

If `roko-cognitive` retains other modules:

```rust
// crates/roko-cognitive/src/lib.rs
// REMOVE
pub mod pheromone;
pub mod pheromones;
```

If `roko-cognitive` becomes empty after deletion, delete the entire crate (same as S-cog-4 pattern).

### 3. Workspace `Cargo.toml`

```bash
rg 'roko-cognitive|roko-pheromones' Cargo.toml
```

Remove member declarations and deps.

### 4. Re-grep

```bash
rg 'pheromone|Pheromone' crates/ Cargo.toml -g '*.rs' -g '*.toml'
# Expect: 0 hits

rg 'roko-cognitive|roko_cognitive' crates/ Cargo.toml -g '*.rs' -g '*.toml'
# Expect: 0 hits, OR only non-pheromone modules within roko-cognitive
```

## Write Scope
- `crates/roko-cognitive/` (or `crates/roko-pheromones/`) — delete tree
- `Cargo.toml`
- Other crate `Cargo.toml`s if they had pheromone deps

## Verify

```bash
rg 'pheromone' crates/ -g '*.rs' Cargo.toml
# Expect: 0 hits
```

## Acceptance Criteria

- All pheromone code deleted (~68K LOC, per audit).
- Workspace `Cargo.toml` updated.
- `cargo check --workspace` clean.

## Do NOT

- Do NOT migrate features from pheromones into another crate just to keep the ideas alive. If pheromones had a real use case, S-cog-1 would have surfaced it.
- Do NOT bundle with S-cog-1/2/3/4.
- Do NOT skip the workspace `Cargo.toml` update.
- Do NOT preserve any pheromone constants or helpers "for reference."
