# GATE_19: Define custom gate config schema

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-19`](../ISSUE-TRACKER.md#gate-19)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.19
- Priority: **P2**
- Effort: 2 hours
- Depends on: `GATE_01` (source 4.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Currently, custom gates are configured via `ShellGateCommand` in `GateConfig.shell_gates`. This supports arbitrary shell commands but requires callers to construct the vector programmatically. A config-driven approach (from `roko.toml`) enables user extensibility without code changes.

## Exact Changes

1. Add `CustomGateSpec` to `crates/roko-core/src/foundation.rs`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct CustomGateSpec {
       /// Gate name (used in enabled_gates list).
       pub name: String,
       /// Program to invoke.
       pub program: String,
       /// Arguments.
       #[serde(default)]
       pub args: Vec<String>,
       /// Timeout in milliseconds (default: 60000).
       #[serde(default = "default_custom_gate_timeout")]
       pub timeout_ms: u64,
       /// Which rung to assign (for ordering). Default: 5 (custom).
       #[serde(default = "default_custom_rung")]
       pub rung: u8,
       /// Whether non-empty stderr should fail the gate.
       #[serde(default)]
       pub fail_on_stderr: bool,
   }
   ```
2. Add `custom_gates: Vec<CustomGateSpec>` to `GateConfig`.
3. Update all construction sites with `custom_gates: vec![]`.

## Write Scope

- `crates/roko-core/src/foundation.rs`

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

- [ ] `CustomGateSpec` is serializable/deserializable
- [ ] Default timeout and rung values are sensible

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `CustomGateSpec` is serializable/deserializable
- Default timeout and rung values are sensible
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
