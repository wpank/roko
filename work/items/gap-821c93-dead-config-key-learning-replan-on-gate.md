+++
id = "gap-821c93"
kind = "gap"
title = "Dead config key learning.replan_on_gate_failure still written by the demo config, its widget, and the ViabilityBench pinned template"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["demo/demo-app", "benchmarks/viabilitybench", "roko-core/config"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK32 gap-b5caf3)"
discovered_from = "gap-b5caf3 (backlog task 4110)"
anchors = ["demo/demo-resources/roko.toml", "benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml", "demo/demo-app/src/components/ConfigWidget.tsx", "benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/run_roko_plan.py"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'replan_on_gate_failure' demo/demo-resources/roko.toml benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml demo/demo-app/src/components/ConfigWidget.tsx benchmarks/viabilitybench/driver/planemit.py benchmarks/viabilitybench/driver/run_roko_plan.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -q"
+++

## Problem

Backlog task 4110 (part of `gap-b5caf3`, done) removed the `[learning] replan_on_gate_failure` config key from
roko's own schema and made loading it warn (`crates/roko-core/src/config/loader.rs:1530-1531`:
`"learning.replan_on_gate_failure was removed because no plan run ..."`) and `config set` refuse it
(`crates/roko-core/src/config/loader.rs:4668-4698`, the `dead_config_keys_are_removed_and_old_files_still_load`
test's own fixture). But three other places in the tree still write or offer the key as if it were live:

- `demo/demo-resources/roko.toml:27`: `replan_on_gate_failure = false`
- `benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml:58`: `replan_on_gate_failure = false`
- `demo/demo-app/src/components/ConfigWidget.tsx:83`: `{ key: 'replan_on_gate_failure', label: 'Replan on Failure', type: 'boolean' }` — offered as a live, editable field in the demo's config UI.

A fourth location, `demo/demo-app/src/lib/scenario-runners/archive/gate-retry.ts:25,84`, also references
`roko config set learning.replan_on_gate_failure true` twice, but lives under an `archive/` directory — confirmed
it is not imported by any live scenario runner (`grep -rl "gate-retry" demo/demo-app/src/lib/scenario-runners/*.ts`
finds nothing outside `archive/`), so it is dead code already and not counted as a live instance here, only noted.

## Why it matters

Any workspace that copies `demo/demo-resources/roko.toml` as a starting config, or the ViabilityBench pinned
template used to build fixed, byte-identical test configs, now loads with roko's own dead-key warning on every
run. The demo's `ConfigWidget` additionally lets an operator toggle a setting that does nothing and, if the demo
ever round-trips through `config set`, would now be flatly refused instead of silently accepted — a worse UX than
either removing the field or explaining it.

## Where

- `demo/demo-resources/roko.toml:27`
- `benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml:58`
- `demo/demo-app/src/components/ConfigWidget.tsx:83`
- (dead, not counted) `demo/demo-app/src/lib/scenario-runners/archive/gate-retry.ts:25,84`

## Current state

`crates/roko-core/src/config/loader.rs` already does the right thing for roko's own config loading and
`config set` (4110, done). These four other files were not part of that task's own `[[verify]]` and were missed.

## Plan

1. Remove the `replan_on_gate_failure = false` line from `demo/demo-resources/roko.toml` and
   `benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml` (check whether the ViabilityBench file's
   SHA-pinning means its hash must be re-pinned after the edit — read `benchmarks/viabilitybench/driver/planemit.py`
   for how `pinned.roko.toml` is verified against a hash before changing it).
2. Remove the field from `ConfigWidget.tsx`'s field list.
3. Delete the dead `archive/gate-retry.ts` scenario runner, or leave it (it is already excluded from the live
   scenario list) — your call, out of scope for the verify below either way.

## Done when

- None of the three live locations reference `replan_on_gate_failure`.
- The `[[verify]]` command passes.

## Notes

- Do not touch `crates/roko-core/src/config/loader.rs` itself — its warn/refuse behavior for this key is correct
  and already shipped (4110).
- If the ViabilityBench pinned template's hash-check fails after editing it, that is expected and part of the fix:
  re-pin it per `planemit.py`'s own instructions, not a reason to leave the dead key in.
- 2026-10-03 (coordinator, gate 7a): raised to p1. PK36's shakedown (3314, `driver/test_shakedown.py`), run against
  a binary built from main, fails all eight scenarios at setup: the driver's emitted config still writes the key
  (`planemit.py`'s `CONFIG_TEMPLATE`, line 172, and `run_roko_plan.py`, line 225), and `roko plan validate --strict`
  now refuses the unknown field, so no ViabilityBench Roko arm can run against main. The template is pinned:
  removing the line means bumping `TEMPLATE_VERSION` (`planemit-3`), updating the pinned `TEMPLATE_SHA256` in
  `test_planemit.py` and the golden `testdata/planemit/pinned.roko.toml`, and checking the ladder and Claude CLI
  templates and any pre-registration lock that names these hashes. The key was off (`false`), so removing it
  doesn't change behaviour.

## Progress

- Removed `replan_on_gate_failure = false` from `planemit.py`'s `CONFIG_TAIL` (shared by the pinned, ladder and
  Claude CLI templates — one edit fixes all three) and from `run_roko_plan.py`'s hand-written `[learning]` block.
  Bumped `TEMPLATE_VERSION` `planemit-3` -> `planemit-4`. Regenerated both golden files
  (`testdata/planemit/pinned.{roko,tasks}.toml`) by calling `planemit.emit(pinned_spec(), ...)` and diffing the
  result against the old goldens first: the only other change in either file is the version string in its header
  comment, confirming the edit is surgical. Updated `test_planemit.py`'s pinned `TEMPLATE_VERSION`/`TEMPLATE_SHA256`
  (new hash `24d5db33665a3537449708ff920e53ec0e182d9fd77c411c1feefd1eae4b8007`) and dropped its now-nonexistent
  `config["learning"]["replan_on_gate_failure"]` assertion. `LADDER_TEMPLATE_SHA256`/`CLAUDE_CLI_TEMPLATE_SHA256`
  have no hardcoded pins anywhere (only a `!=` check against `TEMPLATE_SHA256`), so nothing else needed re-pinning.
  Removed the key (and the now-empty `[learning]` section) from `demo/demo-resources/roko.toml`, and the
  `replan_on_gate_failure` field row from `ConfigWidget.tsx`'s `learning` section (its other three fields stay).
  Left `demo-app/src/lib/scenario-runners/archive/gate-retry.ts` untouched: already dead/archived and excluded from
  the live scenario list, out of scope either way per the item's own Plan step 3.
- Grep for a pre-registration lock or doc naming the old hash or version: the old hash
  (`7da8ed4b6f1a7dc39d9c957ef48250c2171185d4b703381e10cc510140e7d910`) appeared nowhere outside `test_planemit.py`
  (now updated); no pre-registration lock file references it. The old version string (`"planemit-3"`) appears in
  one closed item's historical note (`work/done/gap-4ec59f-...md:279`, a descriptive aside, not a pin — left as is,
  out of scope) and in `docs/v1`/`docs/v2`'s roko-config documentation of this key's pre-removal behaviour, which
  documents roko's own schema (unrelated to ViabilityBench) in doc trees already known-historical; `docs/v3`
  already documents the key as removed. Per instructions, `docs/whitepaper/*` and `tmp/cybernetic-harness/paper/*`
  were not touched or checked further.
- Verify command passes: no live location references the key, and `test_planemit.py` is 10 passed/1 skipped
  without a binary, 11 passed with `VB_TEST_ROKO_BIN` set (the `real_roko`-marked ladder-validation test). Ran the
  other three test files that import `planemit`/`run_roko_plan` (`test_run_roko.py`, `test_run_roko_routed.py`,
  `test_run_roko_plan.py`) as a regression check: all pass, zero collateral change.
- Ran the shakedown suite (3314) for real verdicts with `VB_TEST_ROKO_BIN` set to the given copy of main's binary
  (`823f2cfca`-era) and `VB_REQUIRE_REAL_ROKO=1`: setup no longer fails (confirming this fix unblocks the suite).
  5 of 8 pass (D2, D4, D5, D6, D10). 3 fail:
  - **D1** (blank answer isolation, G01): isolation itself is fixed (>= 2 attempts run after the blank reply), but
    the task's final verdict is `gate_failed`, not `completed` (turns=4, cost=$0.0024 — real work was metered).
    Points at the gate-rerun path for a recovered/retried attempt, not the isolation logic:
    `crates/roko-cli/src/runner/gate_dispatch.rs` (`run_gate_once`) or the retry loop in
    `crates/roko-cli/src/graph_task_dispatch.rs`.
  - **D3** (ladder escalation on a provider 500, G12) and **D7** (ladder escalation on a 401/auth failure, G04)
    fail with the same signature: `model_dispatched` on every recorded attempt stays the cheap rung
    (`gpt-oss-120b`), but the metering proxy's own `model_requested`/`model_reported` fields show the wire traffic
    did escalate (`glm-4.7`, then `gpt-5.4-mini`) — i.e. the ladder climbs correctly on the wire, but the verdict's
    `model_dispatched` bookkeeping does not follow it, so the run ends `infra_error` (`model_mismatch`) instead of
    recording a clean escalation. Both likely share one root cause. Code path: `model_dispatched` is written once,
    from `dispatch.target.model_slug`, at `crates/roko-cli/src/graph_task_dispatch/attempt.rs:918`; rung
    substitution is decided in `crates/roko-cli/src/graph_task_dispatch/ladder.rs`. Not fixed, per instructions.
