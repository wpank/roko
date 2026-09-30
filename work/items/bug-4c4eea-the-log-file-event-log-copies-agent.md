+++
id = "bug-4c4eea"
kind = "bug"
title = "The --log-file event log copies agent output verbatim, so evidence bundles hold raw agent text"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a7a70f5fe"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-evidence's report on gap-09e478)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::record", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent", "scripts/run_evidence.py"]
lane = "rust-cold"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["gap-09e478"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn log_file_never_copies_agent_output_text' crates/roko-cli/src/ && cargo test -p roko-cli --lib log_file_never_copies_agent_output_text"
+++

## Problem

`EventLogWriter::record` writes every StateHub event to the `--log-file` JSONL as it is. `DashboardEvent::AgentOutput` carries the agent's streamed text in `content`, so each piece of agent output becomes a `dashboard.agent_output` line. `./dev.sh fast` and `run-evidence` pass `--log-file {bundle}/events.jsonl`, so an evidence bundle's `events.jsonl` holds the agent's raw output: tool results, file contents, and any secret an agent or tool printed. Nothing redacts it; the collector's `redact_text` applies only to what the collector copies itself.

## Why it matters

The development record (epic spec-f2463d) keeps bundles to derived data. The collector keeps Activity rows as "derived fields only, no signal bodies", and truncates and redacts summaries (gap-09e478). The event log breaks that rule with the largest volume of text in a run.

## Where

- `crates/roko-cli/src/graph_execution/event_log.rs`: `EventLogWriter::record` (:260).
- `crates/roko-core/src/dashboard_snapshot.rs`: `DashboardEvent::AgentOutput { content, .. }` (:148). `GateResult.output_text` and `GateOutputLine` also carry raw text.
- `scripts/run_evidence.py`: reads the bundle's `events.jsonl` for its metrics.

## Current state

At BASE the writer does no per-event filtering. The collector's metrics need event kinds and counts, not the text.

## Plan

1. In `record`, replace `AgentOutput.content` with derived fields: byte length, line count and a sha256 of the text. Keep the agent, plan, task and attempt ids.
2. Decide the same for `GateResult.output_text` and `GateOutputLine`, since gate command output can echo secrets too: keep at most a redacted, truncated excerpt.
3. If a raw transcript is ever needed, write it to a separate private file outside the bundle, behind an explicit flag.
4. Add `log_file_never_copies_agent_output_text`: publish an `AgentOutput` event containing a canary string, and check the log holds the canary's hash but not the canary.

## Done when

- [ ] No `--log-file` line contains agent output text.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-4c4eea` at `a7a70f5fe`; cargo verification deferred to the batch check.
- Decisions: `agent_output`, `gate_output_line` and `task_output_appended` keep only `<field>_bytes`, `<field>_lines`
  and `<field>_sha256` (plus `stream_kind` for a TUI stream record). `gate_result` gives the same digest of
  `output_text` plus `output_text_excerpt`: the last 240 bytes after redaction by `LogScrubber`'s built-in patterns,
  `<secret name>=<value>` assignments and the values of secret-named environment variables. The excerpt keeps the
  closing `✗ timed out after N ms` line that `scripts/run_evidence.py` reads (it now reads `output_text_excerpt`,
  falling back to `output_text` for older logs).
- Plan step 3 (a private raw-transcript file behind a flag) was not built: nothing needs a raw transcript yet.
- Python: `python3 scripts/test_run_evidence_graph.py CollectorUnits` passes, including the new
  `test_gate_timeouts_are_read_from_the_logged_excerpt`. The Graph bundle scenarios, which need a built `roko`, now
  also check that `events.jsonl` holds no agent text.
