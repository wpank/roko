# GATE_09: Route LLM judge through CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-09`](../ISSUE-TRACKER.md#gate-09)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.9
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The LLM judge oracle in orchestrate.rs (`AgentJudgeOracle` construction) falls back to `"claude-sonnet-4-20250514"` when no model is configured. This is the AP-5 anti-pattern: hardcoded model strings bypass the routing system. The CascadeRouter already exists and is wired in orchestrate.rs for agent dispatch. It should also be used for judge model selection.

## Exact Changes

1. Locate the `AgentJudgeOracle` construction in orchestrate.rs (search for `AgentJudgeOracle`).
2. Replace the hardcoded model fallback:
   ```rust
   // Before:
   let model = self.config.agent.model.as_deref().unwrap_or("claude-sonnet-4-20250514");

   // After:
   let model = if let Some(m) = self.config.agent.model.as_deref() {
       m.to_string()
   } else if let Ok(router) = self.cascade_router.lock() {
       router.select_model_for_role("gate-judge")
           .unwrap_or_else(|| "claude-sonnet-4-20250514".into())
   } else {
       "claude-sonnet-4-20250514".into()
   };
   ```
3. After the judge call completes, record a CascadeRouter observation:
   ```rust
   if let Ok(mut router) = self.cascade_router.lock() {
       router.observe("gate-judge", &model, judge_score, cost, duration_ms);
   }
   ```
4. Record an episode for each judge invocation:
   ```rust
   let episode = Episode {
       agent_id: "gate-judge".into(),
       task_id: plan_id.to_string(),
       kind: "gate-judge".into(),
       model: model.clone(),
       // ... fill usage from response
   };
   self.episode_logger.record(&episode);
   ```
5. Search for all occurrences of `"claude-sonnet-4-20250514"` in crates/ (excluding tests) and ensure none remain as runtime fallbacks.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test | grep -v '\/\/'` returns no runtime usages
- [ ] Judge model selection goes through CascadeRouter when available
- [ ] Episode recorded per judge invocation with model slug and cost

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test | grep -v '\/\/'` returns no runtime usages
- Judge model selection goes through CascadeRouter when available
- Episode recorded per judge invocation with model slug and cost
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
