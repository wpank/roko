# Migration Rules

## What to rename

Follow the naming table from `01-unified-vocabulary.md`. Key renames:

| Old | New | Phase |
|---|---|---|
| Engram | Signal | 1 |
| Envelope/events | Pulse | 1 |
| EventBus | Bus (trait) / BroadcastBus (impl) | 1 |
| Substrate | Store | 1 |
| Scorer | Score | 1 |
| Gate | Verify | 1 |
| Router | Route | 1 |
| Composer | Compose | 1 |
| Policy | React | 1 |

## Rename protocol

1. **Type alias first**: Add `pub type Signal = Engram;` in a central location
2. **Migrate callers**: Update all callers to use the new name
3. **Remove alias**: Once all callers migrated, remove the type alias
4. **Never break compilation**: Each step must pass `cargo check --workspace`

## What to wire (not build)

The codebase has extensive "built but not connected" code. Before building anything new:

1. `grep -rn 'StructName' crates/ --include='*.rs' | grep -v target/` to find existing code
2. Check if it just needs to be called from the runtime path
3. Wire existing code before writing new code

## What to avoid

- **Don't add features**: Only rename, rewire, or fix. No new functionality.
- **Don't refactor**: Structure stays the same unless the migration demands it.
- **Don't add tests for unchanged code**: Only test what you changed.
- **Don't modify tmp/ or docs/**: Those are reference material, read-only.
- **Don't touch roko.toml config keys**: They're stable API surface.

## Dependency rules

- Phase 0 (prep) must complete before Phase 1
- Phase 1 (kernel renames) can proceed in parallel within phase
- Phase 2 (engine) depends on Phase 1 completing
- Phase 3 (economy) depends on Phase 2 completing
- Within a phase, respect the `batch_deps()` graph

## Crate modification rules

- Prefer modifying one crate per batch where possible
- If a batch touches multiple crates, list them explicitly in the verify commands
- Never add new crate dependencies without justification
- Run `cargo check -p <crate>` after modifying any crate
