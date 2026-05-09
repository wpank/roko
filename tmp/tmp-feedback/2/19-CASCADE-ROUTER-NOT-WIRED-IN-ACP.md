# Cascade Router Not Wired in ACP Path

## Problem

The CascadeRouter (LinUCB bandit for model selection) is fully wired in the CLI `plan run`
path but completely absent from ACP slash commands. Every ACP command uses whatever model
the user configured globally — no adaptive routing, no learning from outcomes.

Additionally, cascade router observations are silently dropped when the config key doesn't
match the wire slug.

## Root Cause

### A. ACP path skips CascadeRouter

**File:** `crates/roko-acp/src/bridge_events.rs`

Slash commands hardcode the model from `session.model` or the command's `--model` arg:
```rust
let options = AgentExecOptions {
    model: Some(session.model.clone()),  // ← always the global model
    // no cascade_router consultation
    ..
};
```

In contrast, `orchestrate.rs:dispatch_agent_with()` calls:
```rust
let model = cascade_router.select_model(&task_context);
// ... after execution:
cascade_router.observe(model, outcome);
```

### B. Observation key mismatch

**File:** `crates/roko-learn/src/cascade_router.rs`

The router persists to `.roko/learn/cascade-router.json`. Model keys in the config
(`roko.toml`) use display names like `"gpt-4o-mini"`, but observations use the wire
slug from the provider (e.g., `"gpt54-mini"`). When keys don't match, the observation
is recorded against a model that was never in the selection set, so it never influences
future selections.

```
Config:    models = ["gpt-4o-mini", "claude-sonnet-4-20250514"]
Observed:  model = "gpt54-mini"    ← doesn't match "gpt-4o-mini"
Result:    observation silently recorded but never read back
```

### C. DaimonState always `default()` in ACP

**File:** `crates/roko-acp/src/bridge_events.rs`

The DaimonState (affect engine) influences model selection, turn limits, and tool policy
in the CLI path. In ACP, `DaimonState::default()` is used, meaning affect-based modulation
never happens.

## Fix

### Fix 1: Wire CascadeRouter into ACP dispatch (~20 min)

**File:** `crates/roko-acp/src/session.rs`

Add `cascade_router: CascadeRouter` to `AcpSession`. Initialize from
`.roko/learn/cascade-router.json` on session start. In `bridge_events.rs`, use it for
model selection on slash commands that benefit from routing (`/do`, `/research`, `/analyze`).

### Fix 2: Normalize model keys (~10 min)

**File:** `crates/roko-learn/src/cascade_router.rs`

Add a `normalize_model_key()` function that maps both config names and wire slugs to a
canonical form. Call it in both `select_model()` and `observe()`.

### Fix 3: Load DaimonState in ACP (~10 min)

**File:** `crates/roko-acp/src/session.rs`

Load `DaimonState` from `.roko/state/daimon.json` (or compute from recent episodes) and
pass it through to agent dispatch options.

## Priority

**P1** — The cascade router is the primary mechanism for learning which models work best
for which tasks. Without it in ACP, every interaction uses a fixed model and no learning
occurs. This is 50%+ of roko's usage path (ACP in Zed) getting zero benefit from the
adaptive routing system.
