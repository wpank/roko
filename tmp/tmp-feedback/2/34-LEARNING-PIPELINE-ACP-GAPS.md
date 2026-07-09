# Learning Pipeline ACP Gaps: Distillation Wired, Router/Daimon Not

## Problem

The learning pipeline has improved since the solution docs were written. Episode distillation,
auto-dream, and efficiency events are now wired in the ACP path. But cascade router selection
and DaimonState modulation are still CLI-only.

## Current Status

| Component | CLI Path | ACP Path |
|-----------|----------|----------|
| Episode logging | Wired | Wired |
| Episode distillation | Wired | **Wired** (new) |
| Efficiency events | Wired | **Wired** (new) |
| Auto-dream trigger | Wired | **Wired** (new) — triggers after plan completion |
| Cascade router **selection** | Wired | **NOT wired** — always uses session.model |
| Cascade router **observation** | Wired | Partial — observations recorded but key mismatch |
| DaimonState modulation | Wired | **NOT wired** — always default() |
| Playbook injection | Wired | Partial — only for plan run, not slash commands |
| Prompt experiments (A/B) | Wired | **NOT wired** — no experiment selection in ACP |

## What Changed Since Solution Docs

The `tmp/solutions/self-developing/` docs were written before these items were wired:

1. **Episode distillation** — now runs after each agent completion in ACP
2. **Auto-dream** — `maybe_auto_dream()` triggers at plan completion
3. **Efficiency events** — written to `.roko/learn/efficiency.jsonl` from ACP

These should be crossed off in any priority lists.

## Remaining Gaps

### A. Cascade Router Selection in ACP (~15 min fix)

See doc 19 for full details. The router's `select_model()` is never called from ACP.
Every ACP interaction uses the fixed session model.

### B. DaimonState in ACP (~10 min fix)

See doc 19. DaimonState influences:
- Model selection (high arousal → stronger model)
- Turn limits (low valence → fewer turns)
- Tool policy (high dominance → more tools allowed)

Without it, ACP agents don't benefit from affect-based adaptation.

### C. Prompt Experiments in ACP (~15 min fix)

**File:** `crates/roko-learn/src/experiments.rs`

The experiment store supports A/B testing of system prompts. In the CLI path, experiments
are selected per-task. In ACP, the experiment store is never consulted.

**Fix:** In `bridge_events.rs`, before dispatching agents, check for active experiments:
```rust
if let Some(experiment) = experiment_store.active_for_role(&role) {
    let variant = experiment.select_variant();
    options.system_prompt_override = Some(variant.prompt.clone());
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-acp/src/session.rs` | Load cascade router, daimon state, experiment store |
| `crates/roko-acp/src/bridge_events.rs` | Wire router selection + experiment selection |

## Priority

**P1** — ACP is the primary interaction path (Zed users). Not having learning/adaptation
means 50%+ of usage doesn't benefit from roko's learning systems.
