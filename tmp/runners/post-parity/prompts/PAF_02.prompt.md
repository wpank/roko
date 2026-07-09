# PAF_02: Wire knowledge reinforcement signals from gate outcomes

## Task
Emit `ReinforcementSignal::Gated` to the knowledge lifecycle manager after gate outcomes, strengthening entries used in prompts that led to passing gates.

## Runner Context
Runner PAF (Knowledge Lifecycle), batch 2 of 3. Depends on PAF_01.

## Problem
KL-2 anti-pattern: "Knowledge without feedback." The neuro store has `KnowledgeEntry::reinforce()` (lib.rs:610) accepting `ReinforcementSignal` types (`Retrieved`, `Gated`, `Cited`, `Quoted`, `Surprised`). `KnowledgeLifecycleManager::reinforce_batch()` (lifecycle.rs:296-324) is wired for all signal types. But gate outcomes in the v2 runner never emit `Gated` signals back to the lifecycle manager.

## Current Code (VERIFIED)

**Reinforcement signals** — `crates/roko-neuro/src/lib.rs:605-658`:
```rust
pub fn reinforce(&mut self, signal: ReinforcementSignal) { ... }
```
Bumps demurrage balance on the entry.

**Lifecycle reinforce_batch** — `crates/roko-neuro/src/lifecycle.rs:296-324`:
```rust
pub fn reinforce_batch(&mut self, entries: &[EntryId], signal: ReinforcementSignal) { ... }
```
Already wired for: Retrieved (L296), Gated (L302), Cited (L313), Quoted (L321).

**Missing**: No caller emits `Gated` signal from the v2 runner's gate outcome path.

## Exact Changes

### Step 1: Track which knowledge entries were used in each task's prompt

When building the system prompt (via SystemPromptBuilder), record the entry IDs injected:

```rust
// In the dispatch path, after prompt assembly:
let injected_entry_ids: Vec<EntryId> = system_prompt_builder
    .injected_knowledge_ids()  // if this method exists
    .unwrap_or_default();
```

If `injected_knowledge_ids()` doesn't exist, track them during the knowledge injection step (PK_03):

```rust
// During knowledge injection into prompt:
let mut injected_ids = Vec::new();
for entry in knowledge_entries {
    injected_ids.push(entry.id.clone());
    // ... inject into prompt ...
}
```

### Step 2: Emit Gated reinforcement after gate passes

In `event_loop.rs`, after a task's gates pass:

```rust
if gate_passed && !injected_entry_ids.is_empty() {
    if let Some(lifecycle) = &mut lifecycle_manager {
        lifecycle.reinforce_batch(
            &injected_entry_ids,
            ReinforcementSignal::Gated,
        );
        debug!(
            entries = injected_entry_ids.len(),
            "reinforced knowledge entries after gate pass"
        );
    }
}
```

### Step 3: Emit Retrieved reinforcement at query time

When knowledge entries are queried for prompt injection:

```rust
// After querying knowledge store for task context:
if let Some(lifecycle) = &mut lifecycle_manager {
    let retrieved_ids: Vec<_> = entries.iter().map(|e| e.id.clone()).collect();
    lifecycle.reinforce_batch(
        &retrieved_ids,
        ReinforcementSignal::Retrieved,
    );
}
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (emit Gated after gate pass)
- `crates/roko-cli/src/knowledge_helpers.rs` (emit Retrieved at query time)

## Read-Only Context
- `crates/roko-neuro/src/lib.rs` (KnowledgeEntry::reinforce, ReinforcementSignal)
- `crates/roko-neuro/src/lifecycle.rs` (reinforce_batch)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Gate pass → `Gated` reinforcement for all knowledge entries used in that task's prompt
- Knowledge query → `Retrieved` reinforcement for all returned entries
- Reinforcement calls are best-effort (no crash on failure)
- Entry IDs tracked from prompt assembly to gate outcome
- No reinforcement emitted on gate failure (only on pass)

## Do NOT
- Change the ReinforcementSignal enum
- Add new signal types
- Emit reinforcement for entries NOT used in the task's prompt
