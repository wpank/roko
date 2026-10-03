+++
id = "gap-821c93"
kind = "gap"
title = "Dead config key learning.replan_on_gate_failure still written by the demo config, its widget, and the ViabilityBench pinned template"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["demo/demo-app", "benchmarks/viabilitybench", "roko-core/config"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK32 gap-b5caf3)"
discovered_from = "gap-b5caf3 (backlog task 4110)"
anchors = ["demo/demo-resources/roko.toml", "benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml", "demo/demo-app/src/components/ConfigWidget.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'replan_on_gate_failure' demo/demo-resources/roko.toml benchmarks/viabilitybench/driver/testdata/planemit/pinned.roko.toml demo/demo-app/src/components/ConfigWidget.tsx"
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
