# M015 — Wire ContextualBanditPolicy into CascadeRouter

## Objective
The `ContextualBanditPolicy` in `crates/roko-learn/src/contextual_bandit.rs` (1,372 LOC) is fully implemented but never called from the model routing path. Wire it into `CascadeRouter` so that bandit feedback updates occur after each agent dispatch, closing the feedback loop between routing decisions and task outcomes.

## Scope
- Crates: `roko-learn`, `roko-cli`
- Files:
  - `crates/roko-learn/src/contextual_bandit.rs` (existing policy)
  - `crates/roko-learn/src/cascade_router.rs` (integrate here)
  - `crates/roko-cli/src/orchestrate.rs` (call site for feedback)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.1
- Audit ref: `tmp/roko-trustworthy/AUDIT.md` §A4

## Steps
1. Understand existing code:
   ```bash
   grep -rn 'ContextualBandit\|BanditPolicy\|BanditDecisionKind' crates/roko-learn/src/ --include='*.rs' | grep -v target/
   grep -rn 'contextual_bandit' crates/roko-learn/src/lib.rs --include='*.rs'
   ```

2. Check how CascadeRouter currently selects models:
   ```bash
   grep -n 'fn select\|fn route\|fn explain_routing' crates/roko-learn/src/cascade_router.rs
   ```

3. In `cascade_router.rs`, add a field to hold a `ContextualBanditPolicy` (or a reference to its store):
   ```rust
   use crate::contextual_bandit::{BanditDecisionKind, BanditContextFeatures, ContextualBanditPolicy};
   ```

4. In the CascadeRouter's UCB stage (stage 3, >200 observations), integrate the bandit policy's `select()` method alongside or replacing the raw LinUCB selection. The bandit should provide an alternative selection when it has sufficient data for `BanditDecisionKind::ProviderModelRouting`.

5. Add a `record_bandit_feedback()` method to CascadeRouter that:
   - Takes a routing decision ID, selected model, task outcome (pass/fail), latency, cost
   - Delegates to `ContextualBanditPolicy::observe()` or equivalent
   - Persists the update to `.roko/learn/bandit-decisions.jsonl`

6. In `orchestrate.rs`, after gate results come back, call `record_bandit_feedback()` with the task outcome:
   ```bash
   grep -n 'gate_results\|gate_verdict\|gate.*pass\|gate.*fail' crates/roko-cli/src/orchestrate.rs | head -20
   ```

7. Add a unit test in `cascade_router.rs` that:
   - Creates a CascadeRouter with bandit integration enabled
   - Records 10 routing decisions with outcomes
   - Verifies the bandit state file updates

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- contextual_bandit
cargo test -p roko-learn -- cascade_router
# Confirm feedback path compiles:
cargo check -p roko-cli
```

## What NOT to do
- Do NOT replace LinUCB — add bandit as an additional signal alongside LinUCB
- Do NOT change the CascadeStage thresholds (50/200 observations)
- Do NOT modify the contextual_bandit.rs policy logic — just wire it into the routing path
- Do NOT add new dependencies — both modules are in the same crate
