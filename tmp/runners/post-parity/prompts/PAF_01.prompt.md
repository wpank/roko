# PAF_01: Wire knowledge admission gate at all ingestion points

## Task
Ensure all knowledge ingestion paths use the `LightAdmissionGate` pre-filter before storing entries, preventing low-quality or duplicate entries from entering the store.

## Runner Context
Runner PAF (Knowledge Lifecycle), batch 1 of 3. No dependencies.

## Problem
KL-1 anti-pattern: "Fire-and-forget ingestion." Knowledge entries are written to the store without pre-filtering in some paths. The `LightAdmissionGate` (admission.rs:30-41) and `KnowledgeAdmissionStore` (admission.rs:586-) exist and are used by `KnowledgeLifecycleManager::observe()` (lifecycle.rs:291), but NOT all ingestion paths go through the lifecycle manager.

## Current Code (VERIFIED)

**LightAdmissionGate** — `crates/roko-neuro/src/admission.rs:30-41`:
Fast pre-filter. Checks confidence threshold, novelty, duplicate detection.

**KnowledgeLifecycleManager** — `crates/roko-neuro/src/lifecycle.rs:196-218`:
Owns an `admission_store`. Its `observe()` method (L291-354) runs light gate first, then optionally submits to full admission.

**Direct ingestion paths that bypass lifecycle manager**:
- Episode distillation (`roko-neuro/src/episode_completion.rs`) — writes directly to knowledge store
- Dream consolidation (`roko-dreams/src/cycle.rs`) — writes insights directly
- Gate failure analysis → knowledge write
- CLI `knowledge` subcommand manual injection

## Exact Changes

### Step 1: Audit all knowledge_store.store() / .append() call sites

```bash
grep -rn 'knowledge_store.*store\|knowledge_store.*append\|knowledge_store.*insert\|knowledge_store.*write' crates/ --include='*.rs' | grep -v target/ | grep -v test
```

For each site, classify:
- **VIA_LIFECYCLE**: Already goes through KnowledgeLifecycleManager::observe() — OK
- **DIRECT**: Bypasses lifecycle manager — NEEDS FIX
- **EXEMPT**: Manual injection (CLI) — document but don't gate

### Step 2: Route episode distillation through lifecycle manager

In `episode_completion.rs`, after distilling an episode into a knowledge entry:

```rust
// BEFORE:
knowledge_store.append(entry)?;

// AFTER:
if let Some(lifecycle) = &lifecycle_manager {
    lifecycle.observe(entry).await?;  // runs admission gate
} else {
    // Fallback to direct write if lifecycle manager not available
    knowledge_store.append(entry)?;
}
```

### Step 3: Route dream consolidation through lifecycle manager

In dream cycle, after generating insights:

```rust
// BEFORE:
knowledge_store.append(insight_entry)?;

// AFTER:
lifecycle_manager.observe(insight_entry).await?;
```

### Step 4: Pass KnowledgeLifecycleManager to all subsystems that write knowledge

Ensure the lifecycle manager is available wherever knowledge is written:

```rust
// In orchestrate.rs or the runner startup:
let lifecycle_manager = KnowledgeLifecycleManager::new(
    knowledge_store.clone(),
    RuntimeKnowledgeConfig::from_config(&config),
);

// Pass to episode completion, dream runner, etc.
```

## Write Scope
- `crates/roko-neuro/src/episode_completion.rs` (route through lifecycle)
- `crates/roko-dreams/src/cycle.rs` (route through lifecycle)
- `crates/roko-cli/src/runner/event_loop.rs` (pass lifecycle manager to subsystems)

## Read-Only Context
- `crates/roko-neuro/src/admission.rs` (LightAdmissionGate, KnowledgeAdmissionStore)
- `crates/roko-neuro/src/lifecycle.rs` (KnowledgeLifecycleManager, observe, RuntimeKnowledgeConfig)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- All automated knowledge ingestion routed through KnowledgeLifecycleManager::observe()
- LightAdmissionGate filters out low-confidence and duplicate entries
- Direct store.append() only used when lifecycle manager unavailable (with warning log)
- Manual CLI injection exempt from gate (documented)
- Existing knowledge entries not affected (gate is for new ingestion only)

## Do NOT
- Change the LightAdmissionGate or KnowledgeAdmissionStore
- Block manual CLI knowledge injection
- Remove existing direct-write fallback paths (keep as backup)
