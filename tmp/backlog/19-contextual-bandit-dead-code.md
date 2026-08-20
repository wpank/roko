# 19 — Remove contextual_bandit.rs Dead Code from roko-learn

**Priority**: P3 — cleanup; no functional impact
**Size**: XS (~30 minutes)
**Crates**: `crates/roko-learn/`, `crates/roko-cli/`
**Depends on**: None

---

## Background

Roko is a Rust workspace of ~35 crates for building self-developing agents. The `roko-learn`
crate handles all learning and feedback for the plan execution engine: model routing, episode
logging, playbooks, and the cascade router that selects which LLM provider to call for each
task in a plan.

At the core of model routing is a contextual bandit algorithm — a class of reinforcement
learning algorithms that pick the best option from a set of choices given context about the
current situation (task role, plan type, cost constraints). Roko has a working bandit
implementation called `LinUCBRouter` inside `crates/roko-learn/src/model_router.rs`. The
`CascadeRouter` struct in `cascade_router.rs` owns a `LinUCBRouter` field and calls it
directly at every dispatch and after every task completes.

There is also a second, completely separate bandit implementation in
`crates/roko-learn/src/contextual_bandit.rs`. This second implementation (`ContextualBanditPolicy`)
was deleted during a cleanup pass in April 2026 because its logic had been superseded by
`LinUCBRouter`. An automated batch agent run subsequently re-added it. It has sat in the
codebase since: publicly exported, never called by any runtime path, but keeping a 1,372-line
file in the build and creating noise for anyone reading the learning subsystem.

This item removes the dead file and its two references.

## Current State

1. **`/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/contextual_bandit.rs`** — 1,372 lines. Defines 19+ public types including `ContextualBanditPolicy`, `BanditDecisionKind`, `BanditContextFeatures`, `BanditActionCandidate`, `BanditRewardObservation`, `RewardMetrics`, `PolicyUpdateCandidate`, `BanditPolicyConfig`, `ActionSafetyBounds`, and `CONTEXTUAL_BANDIT_SCHEMA_VERSION`. Contains 6 self-contained unit tests (all passing). Not imported by any module in `roko-learn` or `roko-agent` or `roko-cli` except through `lib.rs` and the one test below. The doc comment says it is "intentionally generic" — but there is no wire-up to `CascadeRouter`, the agent dispatcher, or any CLI command.

2. **`/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs` line 99** — `pub mod contextual_bandit;` with doc comment "Contextual bandit policy for model-selection feedback and reward recording." This is the only declaration that keeps the file in the build.

3. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/phase0_wiring.rs` lines 257–291** — A test function `phase0_bandit_policy_records_rewards` that imports 7 types from `roko_learn::contextual_bandit` and calls `policy.record_reward(...)`. The test comment says "we just verify it doesn't panic." It was written to satisfy a wiring checklist, not to validate any production integration. This is the sole external caller.

4. **`/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`** — The real runtime bandit. Owns a `LinUCBRouter` field and calls `linucb.select(...)` and `linucb.update(...)`. No reference to `ContextualBanditPolicy` anywhere in this file.

5. **`/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs`** — Contains `LinUCBRouter` with three-stage selection (static / confidence-interval / full UCB). This is what the runner actually uses.

## Implementation Plan

Three file changes, in this order:

### Step 1: Delete the dead file

```bash
rm /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/contextual_bandit.rs
```

### Step 2: Remove the `pub mod` declaration from `lib.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`, remove lines 98–99:

```rust
/// Contextual bandit policy for model-selection feedback and reward recording.
pub mod contextual_bandit;
```

The surrounding context (lines 95–104 for reference):
```rust
/// Learned intervention policy for conductor retries and aborts.
pub mod conductor;
pub mod context_pack_cache;
/// Contextual bandit policy for model-selection feedback and reward recording.  <- DELETE THIS
pub mod contextual_bandit;                                                         <- DELETE THIS
/// Pre-dispatch cost projection for budget estimation.
pub mod cost_projection;
```

### Step 3: Remove the dead test from `phase0_wiring.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/phase0_wiring.rs`, delete lines 256–291 (the `phase0_bandit_policy_records_rewards` function). It starts at line 256 with the doc comment `/// Verify bandit policy can record rewards and produce update candidates.` and ends at line 291 with the closing `}`.

The function immediately after it (`phase0_registries_basic_ops` at line 293) should remain untouched.

## Acceptance Criteria

1. `crates/roko-learn/src/contextual_bandit.rs` does not exist in the repository.

2. `cargo check --workspace` passes with zero errors and no new warnings.

3. `cargo clippy -p roko-learn -- -D warnings` passes clean.

4. `cargo test -p roko-learn && cargo test -p roko-cli` both pass; neither references `ContextualBanditPolicy` or any type from the deleted module.

## Verification Checklist

- [ ] `ls crates/roko-learn/src/contextual_bandit.rs` — "No such file or directory"
- [ ] `grep -rn 'contextual_bandit' crates/` — zero hits
- [ ] `cargo test -p roko-learn` — all tests pass
- [ ] `cargo clippy -p roko-learn -- -D warnings` — clean
- [ ] `cargo test -p roko-cli` — all tests pass
- [ ] `cargo check --workspace` — zero errors

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/contextual_bandit.rs` | Delete entirely |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs` | Remove lines 98–99 (`pub mod contextual_bandit;` and its doc comment) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/phase0_wiring.rs` | Remove lines 256–291 (`phase0_bandit_policy_records_rewards` test function) |
