# 228 — Dogfood Session Evidence Bundle

> **Status: SOURCE-IMPLEMENTED AS THE DEVELOPMENT HARNESS** (2026-08-31, `bba2f8858`).
> `run-evidence`, `evidence-validate`, `feedback`, and `score` provide a private schema-v2 bundle
> with fresh run-scoped status/log slicing, safe GET/OpenAPI collection, opt-in CLI/text/PNG proof,
> process/resource/Git evidence, metrics, scoring, deterministic debrief, and strict validation.
> Final success/failure/timeout/cancellation/live-screenshot fixtures and representative benchmark
> repetitions remain evidence work, not missing harness code.

**Priority**: P1 — self-hosting failures are expensive to reproduce, and today's evidence is assembled manually across terminal logs, runner state, HTTP responses, screenshots, and Git worktrees
**Size**: M (2–3 days)
**Wave**: 3
**Crates**: `roko-cli`
**Depends on**: #112 (continuous screenshots, done), #115 (structured log wiring), #182 (lightweight status file, done), #215 (run-scoped HTTP events, soft)
**Source**: `tmp/dogfood-audit/`

## Background

The August 2026 dogfood sessions produced useful diagnoses, but every session used a different
hand-built collection method. Some captured stdout only, some captured a debrief, some inspected
worktrees manually, and none produced a single run-scoped bundle containing the command, commit,
configuration summary, structured events, status samples, screenshots, diffs, timings, and final
diagnosis. This made later auditing depend on prose summaries and made it hard to distinguish an
old defect from a regression in current code.

Roko now has most of the required primitives:

- `plan run --log-file <path>` writes flushed JSONL runner events.
- `plan run --screenshots` captures event-driven text snapshots plus a manifest.
- `.roko/state/status.json` is a cheap polling surface.
- `.roko/events.jsonl`, `.roko/state/run-ledger.jsonl`, snapshots, prompt logs, and the main tracing
  log contain deeper evidence.
- `roko diagnose`, `roko status --json`, and the HTTP control plane expose structured summaries.

The missing piece is a run-scoped collector that composes these surfaces and copies only the
relevant evidence into a portable, redacted directory.

## Implementation Plan

Stage 0 subset now present:

- [x] Create a unique bundle before command dispatch and expose a generic wrapper.
- [x] Record redacted invocation, Git, host, cache, and timing metadata.
- [x] Stream and cap stdout/stderr separately while preserving live output.
- [x] Record success/failure/timeout/cancellation semantics and terminate the full process group.
- [x] Capture bounded Git diff/untracked metadata and FAST timed-out-attempt pointers/patches.
- [x] Filter fresh append-only runner surfaces by observed `run_id` and collect status samples.
- [x] Query safe HTTP endpoints and capture explicitly selected text/PNG screenshots.
- [x] Generate runner metrics/score/debrief output and enforce the portable schema-v2 bundle.

- [x] Add the supported `dev.sh run-evidence`/`fast` surface that creates a uniquely
   named session directory before dispatch. Accept the normal `plan run` flags without changing
   their semantics.
- [x] Write `manifest.json` before execution with: schema version, UTC start time, invocation argv,
   absolute workspace, Git HEAD/branch/dirty file names, roko version, host OS/architecture,
   selected plan/backlog ID, and redacted config/provider/model metadata. Never persist secret
   values or the full process environment.
- [x] Capture command stdout and stderr separately while preserving live terminal output. Record the
   child exit code and whether it exited, timed out, was cancelled, or was killed by the collector.
- [x] Enable a bundle-local structured event log and selected screenshots. Poll `status.json` at a bounded
   interval and write timestamped samples without blocking the runner.
- [x] At terminal state, copy or slice run-relevant data from the runner event log, run ledger, state
   snapshot, diagnose output, prompt logs (only when explicitly enabled), agent PID history, and
   attempt worktree diff/commit metadata. Use `run_id` to filter append-only files.
- [x] Write `metrics.json` with available setup/agent/gate/report/cleanup latency, token and cost totals, retry
   counts, model/provider selection, cache usage, maximum silence interval, gate failures, changed
   LOC/files, and first/last event times.
- [x] Generate a deterministic `DEBRIEF.md` with facts filled from the bundle and separate
   headings for observations, root-cause hypotheses, code fixes, and follow-up backlog links.
- [x] Add a redaction pass and a bundle validator. Validation fails if expected files are absent,
   JSONL is malformed, a claimed successful run lacks a terminal event, or common secret formats
   are detected.

## Acceptance Criteria

- [x] One command produces a self-contained directory with `manifest.json`, stdout/stderr logs,
   structured events, status samples, screenshots/manifest, metrics, diagnosis, Git/worktree
   evidence, and `DEBRIEF.md`; unselected optional surfaces are explicit `skipped` records.
- [x] Append-only logs are sliced from pre-launch offsets and filtered by observed `run_id`; older
   run evidence is not mixed into the bundle.
- [x] The operator still sees live output while capture is active.
- [x] The bundle records exact exit semantics and cannot label a killed or timed-out process as a
   successful run.
- [x] A validation command checks completeness, parseability, terminal-event consistency, and secret
   redaction.
- [x] The bundle format is documented and schema-versioned in
  `docs/v2/30-EVIDENCE-BUNDLES.md`.

## Verification Checklist

- [ ] Run a one-task successful plan and validate the resulting bundle.
- [ ] Run a mock agent that exits before its first event; verify the bundle records `lost_effect` or
      equivalent terminal evidence and includes the diagnosis.
- [ ] Run a mock gate timeout; verify timing, timeout kind, and ledger event agree.
- [ ] Start with an already-populated `.roko/events.jsonl`; verify the bundle contains only the new
      `run_id`.
- [ ] Put fake API-key-shaped values in test config; verify the bundle validator rejects leaks.
- [ ] Confirm live output remains visible and the capture path does not materially delay execution.

## Files to Modify

| File | Change |
|---|---|
| `dev.sh` | Supported `run-evidence`, `evidence-validate`, `feedback`, `score`, and FAST wrappers |
| `scripts/run_evidence.py` | Bundle lifecycle, collection, bounds, redaction, metrics, scoring, debrief, and validation |
| `docs/v2/30-EVIDENCE-BUNDLES.md` | Schema-v2 operator contract and security boundaries |
| `docs/v2/29-FAST-DEVELOPMENT.md` | FAST evidence policy and required-event integration |
| `crates/roko-serve/src/routes/runs.rs` | Read-only run/bundle/artifact/screenshot metadata queries |
