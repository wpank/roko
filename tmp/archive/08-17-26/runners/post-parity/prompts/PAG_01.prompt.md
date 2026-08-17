# PAG_01: Delete pheromone system

## Task
Remove the pheromone coordination system — it's wired but provides no measurable benefit (confirmation counter stuck at 0, no observed behavioral improvement).

## Runner Context
Runner PAG (Cognitive Cleanup), batch 1 of 3. No dependencies.

## Problem
CL-1 anti-pattern: "Overengineered signal system with zero validation." The pheromone system is a multi-crate mechanism (coordination.rs, system_prompt_builder.rs, context.rs) with deposit/decay/promotion/scope semantics. But:
- Confirmation counter (coordination.rs:311) is always 0 — no code increments it
- No deposit function found in production — `Pheromone::new()` is the only constructor
- Injected into prompts via `pheromone_section()` (system_prompt_builder.rs:1205) but no evidence of improved outcomes
- The UNIFIED-IMPLEMENTATION-PLAN explicitly flags this for deletion

## Current Code (VERIFIED)

**Pheromone struct** — `crates/roko-orchestrator/src/coordination.rs:297-332`:
7 fields including `confirmations: u32` (always 0).

**Promotion system** — `coordination.rs:777-862`:
`PromotionGate` with `min_confirmations` — never triggered since confirmations=0.

**Prompt injection** — `crates/roko-compose/src/system_prompt_builder.rs:184,523,1205-1233`:
`with_pheromones()` at L184, rendered into `pheromone_signals` section at L1205.

**Context source** — `crates/roko-neuro/src/context.rs:144-148`:
`ContextSource::Pheromone { kind, source }` variant.

## Exact Changes

### Step 1: Remove pheromone from SystemPromptBuilder

```rust
// In system_prompt_builder.rs:
// REMOVE: self.pheromones field (L184)
// REMOVE: pheromone_section() method (L1205-1233)
// REMOVE: pheromone injection in build() (L523)
```

### Step 2: Remove ContextSource::Pheromone variant

```rust
// In context.rs:
// REMOVE: ContextSource::Pheromone { kind, source } variant (L144-148)
// Update any match arms that handle this variant
```

### Step 3: Remove or feature-gate Pheromone struct in coordination.rs

```rust
// In coordination.rs:
// OPTION A (preferred): Delete Pheromone struct, PromotionGate, and related code (L297-862)
// OPTION B: Feature-gate behind #[cfg(feature = "pheromones")] if you're unsure about removing
```

### Step 4: Remove any pheromone deposit/read calls in orchestrate.rs

Search for pheromone-related calls in orchestrate.rs and remove:
```bash
grep -n 'pheromone\|Pheromone' crates/roko-cli/src/orchestrate.rs | grep -v '//'
```

### Step 5: Verify compilation

```bash
cargo build --workspace
cargo test --workspace
```

Fix any compilation errors from removed types.

## Write Scope
- `crates/roko-compose/src/system_prompt_builder.rs` (remove pheromone injection)
- `crates/roko-neuro/src/context.rs` (remove ContextSource::Pheromone)
- `crates/roko-orchestrator/src/coordination.rs` (remove Pheromone, PromotionGate)
- `crates/roko-cli/src/orchestrate.rs` (remove pheromone calls)
- Any other files with pheromone references (fix compilation)

## Read-Only Context
- None (this is a deletion task)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- No `Pheromone` struct in production code
- `pheromone_section()` removed from SystemPromptBuilder
- `ContextSource::Pheromone` variant removed
- `cargo build --workspace` passes
- `cargo test --workspace` passes
- No behavioral change (pheromones provided no observable benefit)

## Do NOT
- Remove the stigmergy/coordination concepts that ARE useful (just pheromones)
- Delete test code that was specifically testing pheromone mechanics (remove tests too)
- Touch the signal/substrate systems (those are different)
