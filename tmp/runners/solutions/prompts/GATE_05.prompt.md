# GATE_05: Migrate ACP runner to GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-05`](../ISSUE-TRACKER.md#gate-05)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.5
- Priority: **P0**
- Effort: 4 hours
- Depends on: `GATE_03` (source 4.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run_gates()` at `crates/roko-acp/src/runner.rs:1674` is a ~100 line function that hardcodes three gates: CompileGate, TestGate, ClippyGate. It runs them in the wrong order (compile -> test -> clippy, should be compile -> clippy -> test). It implements its own adaptive threshold loading/saving and skip logic. Replacing it with GateService eliminates AP-6 (duplicate path) and AP-9 (wrong order) simultaneously.

The ACP runner already loads `AdaptiveThresholds` from disk for its skip logic. GateService accepts `with_adaptive_thresholds()` and handles skip/observe internally.

## Exact Changes

1. Add `roko-gate = { path = "../roko-gate" }` to `crates/roko-acp/Cargo.toml` if not already present.
2. Replace the entire `run_gates()` function body:
   ```rust
   async fn run_gates(
       _session_id: &str,
       workdir: &Path,
       clippy_enabled: bool,
       tests_enabled: bool,
       cancel_token: &CancelToken,
   ) -> Result<(bool, String)> {
       let mut enabled = vec!["compile".to_string()];
       if clippy_enabled { enabled.push("clippy".into()); }
       if tests_enabled { enabled.push("test".into()); }

       let gate_config = GateConfig {
           workdir: workdir.to_path_buf(),
           enabled_gates: enabled,
           shell_gates: vec![],
           max_rung: None,
           complexity: None,
           prior_failures: None,
       };

       let thresholds_path = workdir.join(".roko/learn/gate-thresholds.json");
       let thresholds = AdaptiveThresholds::load_or_new(&thresholds_path);
       let svc = GateService::new().with_adaptive_thresholds(thresholds.clone());
       let report = svc.run_gates(gate_config).await?;

       // Save updated thresholds
       if let Ok(t) = thresholds.lock() {
           let _ = t.save(&thresholds_path);
       }

       let passed = report.all_passed();
       let output = report.verdicts.iter()
           .map(|v| format!("{}: {}", v.gate_name, if v.passed { "pass" } else { &v.output }))
           .collect::<Vec<_>>()
           .join("\n");

       Ok((passed, output))
   }
   ```
3. Remove the old inline CompileGate/TestGate/ClippyGate construction and invocation code.
4. Update imports: remove direct gate imports, add `use roko_gate::{GateService, AdaptiveThresholds}; use roko_core::foundation::GateConfig;`.
5. Verify the caller at line 969 (`PipelineAction::RunGates`) still receives `(bool, String)` -- adjust return type if GateReport is more appropriate.

## Design Guidance

Keep the `(bool, String)` return type for minimal caller disruption. The ACP runner's caller only needs pass/fail and a summary string. Future work can expose the full `GateReport` if needed.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-acp/Cargo.toml`

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

- [ ] Gates now run in canonical order: compile -> clippy -> test (rung 0 -> 1 -> 2)
- [ ] Adaptive thresholds are loaded and saved via GateService
- [ ] `grep -rn 'CompileGate\|ClippyGate\|TestGate' crates/roko-acp/` returns no runtime usage (only import lines removed)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gates now run in canonical order: compile -> clippy -> test (rung 0 -> 1 -> 2)
- Adaptive thresholds are loaded and saved via GateService
- `grep -rn 'CompileGate\|ClippyGate\|TestGate' crates/roko-acp/` returns no runtime usage (only import lines removed)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
