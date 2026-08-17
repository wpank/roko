# Remove contextual_bandit.rs dead code from roko-learn

**Status:** Backlog
**Priority:** P3 (cleanup — no functional impact)
**Size:** XS (~30 minutes)
**Crate:** `roko-learn` (`crates/roko-learn/`)

---

## Background

Roko is a Rust workspace (~35 crates, ~800K LOC) for building self-developing agents.
`roko-learn` is the learning and feedback crate. It handles model routing decisions,
episode logging, playbooks, experiments, and the cascade router that picks which LLM
provider to call for each task.

During a cleanup pass in April 2026 the file `contextual_bandit.rs` was deleted because
its logic had already been superseded by the `LinUCBRouter` implementation inside
`model_router.rs`. An automated batch agent run subsequently re-added the file, and it
has sat in the codebase since, publicly exported but never called by any production code.

---

## What exists today

### The dead file

**Path:** `crates/roko-learn/src/contextual_bandit.rs`
**Size:** 1,372 lines
**Tests inside the file:** 6 (self-contained unit tests)
**Public items exported:** 19 top-level `pub struct / pub enum / pub fn / pub const`
declarations, including:

- `CONTEXTUAL_BANDIT_SCHEMA_VERSION` — schema version constant
- `BanditDecisionKind` — enum of routing surfaces (ProviderModelRouting, etc.)
- `BanditContextFeatures` — context passed to the bandit at selection time
- `BanditActionCandidate` — a candidate action the policy can select
- `ActionSafetyBounds` — cost/latency hard limits applied before selection
- `BanditRewardObservation` — reward signal fed back after an action executes
- `RewardMetrics` — latency/cost/token breakdown inside a reward observation
- `PolicyUpdateCandidate` — record emitted instead of mutating manifests directly
- `BanditPolicyConfig` — configuration struct (epsilon, exploration weight, etc.)
- `ContextualBanditPolicy` — the top-level policy struct with `select_action` /
  `record_reward` methods and async JSONL persistence

The module's doc comment says it is "intentionally generic" and that callers provide
action ids, context features, and bounded rewards. The file has no `#[allow(dead_code)]`
annotations of its own; it relies on the crate-level lint suppressions in `lib.rs`.

### The public export

**Path:** `crates/roko-learn/src/lib.rs`, line 99:

```rust
/// Contextual bandit policy for model-selection feedback and reward recording.
pub mod contextual_bandit;
```

### The one external caller

**Path:** `crates/roko-cli/tests/phase0_wiring.rs`, lines 257–291

```rust
/// Verify bandit policy can record rewards and produce update candidates.
#[test]
fn phase0_bandit_policy_records_rewards() {
    use roko_learn::contextual_bandit::{
        ActionSafetyBounds, BanditContextFeatures, BanditDecisionKind, BanditPolicyConfig,
        BanditRewardObservation, ContextualBanditPolicy, RewardMetrics,
    };

    let mut policy = ContextualBanditPolicy::new(BanditPolicyConfig::default());

    let context = BanditContextFeatures::new(
        BanditDecisionKind::ProviderModelRouting,
        "implementation",
        "test-plan",
        "implementer",
    );

    let observation = BanditRewardObservation {
        action_id: "model:claude-sonnet-4-6".to_string(),
        context_key: context.context_key(),
        success: true,
        quality: 1.0,
        metrics: RewardMetrics {
            latency_ms: Some(5000),
            cost_usd: Some(0.05),
            total_tokens: Some(1500),
            retry_count: 0,
        },
    };
    let bounds = ActionSafetyBounds::default();

    // Should not panic and returns Option<PolicyUpdateCandidate>.
    let _candidate = policy.record_reward(observation, bounds);
    // The policy is in default mode which may or may not produce a candidate
    // on the first observation — we just verify it doesn't panic.
}
```

This test does not validate any production behavior. It was added alongside the module
re-introduction to keep the `#[deny(missing_docs)]` and dead-code lints quiet.

---

## Why this is dead code

The actual model routing logic lives in `crates/roko-learn/src/model_router.rs`, which
implements `LinUCBRouter` — a three-stage (static / confidence-interval / full UCB)
contextual bandit. `CascadeRouter` in `cascade_router.rs` owns a `LinUCBRouter` field
and calls it directly at selection and observation time:

```rust
// cascade_router.rs (simplified)
pub struct CascadeRouter {
    linucb: LinUCBRouter,   // ← the real bandit
    ...
}
```

`ContextualBanditPolicy` in `contextual_bandit.rs` is a *different*, independently
implemented bandit that was never wired into `CascadeRouter`, the agent dispatcher, the
gate pipeline, or any CLI command. It carries its own JSONL persistence path, its own
reward schema, and its own exploration logic — none of which are exercised at runtime.

The only non-self-referential caller is the `phase0_bandit_policy_records_rewards` test
in `roko-cli`, which was written purely to satisfy the wiring checklist, not to validate
a real integration.

---

## What to do

### Step 1 — Delete the file

```
crates/roko-learn/src/contextual_bandit.rs
```

### Step 2 — Remove the `pub mod` declaration from `lib.rs`

In `crates/roko-learn/src/lib.rs`, remove lines 98–99:

```rust
/// Contextual bandit policy for model-selection feedback and reward recording.
pub mod contextual_bandit;
```

### Step 3 — Remove or update the test in `phase0_wiring.rs`

In `crates/roko-cli/tests/phase0_wiring.rs`, delete the test function
`phase0_bandit_policy_records_rewards` (lines 256–291). The test has no production
equivalent and will not compile once the module is gone. If the intent was to verify
the cascade router's bandit policy records rewards, a proper replacement test should
target `CascadeRouter::record_observation` in `roko-learn/src/cascade_router.rs` —
but that is out of scope for this cleanup ticket.

---

## Verification

After making the three changes above:

```bash
# 1. Module compiles cleanly with no references to the deleted module
cargo test -p roko-learn

# 2. No dangling imports or lint failures in the learning crate
cargo clippy -p roko-learn -- -D warnings

# 3. The roko-cli test suite passes without the deleted test
cargo test -p roko-cli

# 4. Full workspace check (catches any crate that re-exported or re-used the types)
cargo check --workspace
```

---

## Acceptance criteria

1. `crates/roko-learn/src/contextual_bandit.rs` does not exist in the repository.
2. `cargo check --workspace` passes with zero errors or new warnings.
3. `cargo clippy -p roko-learn -- -D warnings` passes clean.
4. `cargo test -p roko-learn && cargo test -p roko-cli` both pass; neither references
   `ContextualBanditPolicy` or any type from the deleted module.

---

## Out of scope

- Wiring a replacement bandit policy. If `ContextualBanditPolicy` was re-added because
  someone wanted a generic routing policy surface, the right next step is to open a
  separate spec describing the integration point into `CascadeRouter` or the agent
  dispatcher. Do not block this cleanup ticket on that work.
- Touching `model_router.rs`, `cascade_router.rs`, or `roko-conductor`. Those use
  their own bandit implementations that are already wired and tested.
