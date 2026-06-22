# ACPM_23: Add ToolEffectiveness Bandit

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-23`](../ISSUE-TRACKER.md#acpm-23)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.23
- Priority: **P1**
- Effort: 5 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CascadeRouter` at `crates/roko-learn/src/cascade_router.rs` uses Thompson sampling bandits for model selection. The same pattern can be applied to MCP tool strategy selection (`keyword` vs `structural` vs `hybrid` vs `hdc` vs `embedding`).

## Exact Changes

1. Define `ToolCallRecord`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ToolCallRecord {
       pub tool: String,
       pub strategy: Option<String>,
       pub query: String,
       pub results_count: usize,
       pub was_useful: bool,
       pub latency_ms: u64,
       pub timestamp: DateTime<Utc>,
   }
   ```
2. Define `ToolEffectivenessBandit` with per-tool-per-strategy Thompson sampling:
   ```rust
   pub struct ToolEffectivenessBandit {
       arms: HashMap<String, HashMap<String, BanditArm>>,  // tool -> strategy -> arm
       path: PathBuf,
   }

   struct BanditArm {
       successes: f64,
       failures: f64,
   }
   ```
3. Implement `observe(record: &ToolCallRecord)` that updates the bandit arm (success/failure counts).
4. Implement `recommend_strategy(tool: &str) -> String` that samples from the posterior (Beta distribution) and returns the strategy with the highest sample.
5. Implement `stats(tool: &str) -> Vec<(String, f64, usize)>` returning `(strategy, success_rate, observations)`.
6. Persist to `.roko/learn/tool-effectiveness.json`.
7. Add `pub mod tool_effectiveness;` to `crates/roko-learn/src/lib.rs`.

## Design Guidance

Reuse the Thompson sampling math from `CascadeRouter` (Beta(alpha, beta) where alpha = successes + 1, beta = failures + 1). Use the `rand` crate's Beta distribution for sampling. When a tool has fewer than 10 observations for any strategy, return "hybrid" as the default.

## Write Scope

- `crates/roko-learn/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: after 10 positive observations for "hybrid" and 2 for "keyword", "hybrid" is recommended more often (sample 100 times, >60% should be "hybrid")
- [ ] Persistence round-trip preserves bandit state
- [ ] Empty state defaults to "hybrid"

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: after 10 positive observations for "hybrid" and 2 for "keyword", "hybrid" is recommended more often (sample 100 times, >60% should be "hybrid")
- Persistence round-trip preserves bandit state
- Empty state defaults to "hybrid"
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
