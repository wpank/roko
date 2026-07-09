# OBS__33: Add end-to-end observability integration test

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#obs--33`](../ISSUE-TRACKER.md#obs--33)
- Source: `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` — Task 18.33
- Priority: **??**
- Effort: ?
- Depends on: `OBS__21` (source 18.21), `OBS__22` (source 18.22), `OBS__25` (source 18.25), `OBS__29` (source 18.29)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: OBS__33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

_(no implementation section in source — read source task)_

## Write Scope

- `crates/roko-serve/tests/obs_integration.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/18-OBSERVABILITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `rg 'obs_events_flow\|obs_integration' crates/roko-serve/tests/` matches >= 1.
- [ ] Every `ModelCallService::call()` emits a `gen_ai.*` span with model, tokens, cost, latency.
- [ ] TUI dashboard shows RouterTrace, CostPanel, and GateRow widgets with live data.
- [ ] `JsonlLogger` batches writes (measured: < 5ms persistence overhead per run).
- [ ] Prometheus endpoint exports `roko_model_calls_total`, `roko_cost_usd_total`, `roko_gate_results_total`.
- [ ] Agent heartbeat and stall detection runs for all supervised agents.
- [ ] `/api/obs/cost` returns model/role cost breakdown with savings calculation.
- [ ] `/api/obs/latency` returns per-phase percentile breakdown.
- [ ] Anomaly detection fires on cost spikes, pass-rate drops, and latency surges.
- [ ] Anomaly alerts render in TUI error digest and CLI stderr.
- [ ] WebSocket endpoint streams filtered events.
- [ ] `cargo run -p roko-cli --features otel -- run "hello"` sends spans to configured OTel endpoint.
- [ ] `roko config otel set` configures export without env vars.
- [ ] End-to-end integration test verifies full pipeline.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: OBS__33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `rg 'obs_events_flow\|obs_integration' crates/roko-serve/tests/` matches >= 1.
- Every `ModelCallService::call()` emits a `gen_ai.*` span with model, tokens, cost, latency.
- TUI dashboard shows RouterTrace, CostPanel, and GateRow widgets with live data.
- `JsonlLogger` batches writes (measured: < 5ms persistence overhead per run).
- Prometheus endpoint exports `roko_model_calls_total`, `roko_cost_usd_total`, `roko_gate_results_total`.
- Agent heartbeat and stall detection runs for all supervised agents.
- `/api/obs/cost` returns model/role cost breakdown with savings calculation.
- `/api/obs/latency` returns per-phase percentile breakdown.
- Anomaly detection fires on cost spikes, pass-rate drops, and latency surges.
- Anomaly alerts render in TUI error digest and CLI stderr.
- WebSocket endpoint streams filtered events.
- `roko config otel set` configures export without env vars.
- End-to-end integration test verifies full pipeline.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: OBS__33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
