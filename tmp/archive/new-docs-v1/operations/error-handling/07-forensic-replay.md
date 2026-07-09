# Forensic Replay

> How to reconstruct exactly what Roko did during a failed run — replaying the event log
> to reproduce the failure in isolation, without re-running the full task.

**Status**: Built (event log, hash chain, verify command) / Specified (interactive replay UI)
**Crate**: `roko-orchestrator`, `roko-fs`
**Depends on**: [Event Log Replay](03-event-log-replay.md), [Error Taxonomy](01-error-taxonomy.md)
**Used by**: [Observability](08-observability.md), [Failure Drill Examples](09-failure-drill-examples.md)
**Last reviewed**: 2026-04-19

---

## Purpose

Forensic replay answers the operator's most important post-failure question:

> "What exactly happened, in what order, and where did it go wrong?"

It is distinct from **crash recovery** (restoring execution state) and **event log replay**
(replaying to a checkpoint for resume). Forensic replay is a **read-only diagnostic** — it
reads the event log, verifies integrity, and reconstructs the failure sequence without
modifying any state.

---

## Prerequisite: the Event Log

Every Roko task writes a BLAKE3 hash-chained JSONL event log to:

```
.roko/events/<task-id>.jsonl
```

Each record has the form:

```json
{
  "seq":       42,
  "timestamp": "2026-04-19T14:03:17.441Z",
  "event":     "GateFailed",
  "data":      { "rung": "test", "exit_code": 1, "stderr_tail": "..." },
  "prev_hash": "blake3:a3f9...",
  "hash":      "blake3:7c12..."
}
```

The chain is intact if `hash(prev_hash || event || data) == hash`. A broken chain means
either log corruption or tampering — the forensic replay will stop at the break point.

---

## Step 1 — Verify Chain Integrity

Always verify before reading the log forensically. A corrupt chain means the log cannot
be trusted as a complete record.

```bash
roko events verify --task <task-id>
```

Expected output (clean):

```
Verifying task-7f3a ...
  Records: 187
  Chain:   OK (all 187 hashes valid)
  Span:    2026-04-19T14:00:00Z → 2026-04-19T14:03:22Z  (3m 22s)
```

Output (corrupt):

```
Verifying task-7f3a ...
  Records: 187
  Chain:   BROKEN at seq=103 (hash mismatch)
  Safe records: 0–102  (records 103–187 untrusted)
```

If the chain is broken, only trust events 0–102. The failure almost certainly occurred at
or after seq=103. Check disk health and look for partial writes.

---

## Step 2 — Show the Full Event Sequence

```bash
# Human-readable timeline
roko events show <task-id>

# Filter to a specific time window
roko events show <task-id> --from 14:02:00 --to 14:03:30

# Filter to a specific event type
roko events show <task-id> --type GateFailed,AgentTurn,CircuitOpen

# Raw JSONL (for piping to jq)
roko events dump <task-id>
```

### Common event types and what they mean

| Event type | What it records |
|---|---|
| `TaskStarted` | Task ID, config snapshot, start time |
| `AgentTurn` | Turn number, prompt tokens, completion tokens, model used |
| `AgentProducedDiff` | Diff hash, file paths touched, lines changed |
| `GateStarted` | Rung name, input hash |
| `GatePassed` | Rung name, duration |
| `GateFailed` | Rung name, exit code, stderr tail (last 4 KB) |
| `GateRetry` | Rung name, attempt number, back-off duration |
| `GateEscalated` | Rung name, escalation reason |
| `CircuitOpen` | Service name, failure count, reset ETA |
| `SubtaskStarted` | Subtask ID, PID |
| `SubtaskSucceeded` | Subtask ID, duration |
| `SubtaskFailed` | Subtask ID, error code, error message |
| `LLMRequest` | Model, prompt hash (not full prompt), token estimate |
| `LLMResponse` | Model, latency_ms, tokens used, finish_reason |
| `LLMError` | Model, error_code, http_status |
| `SubstrateWrite` | Record type, byte size |
| `SubstrateError` | Operation, error message |
| `EpisodeQueued` | Episode ID, routing tier |
| `TaskCompleted` | Final verdict, total duration |
| `TaskFailed` | Root error code, cascade depth |

---

## Step 3 — Isolate the Root Cause

Use the event timeline to walk backward from the terminal failure to the root cause.

### Example: tracking a test failure back to a bad agent turn

```bash
roko events dump <task-id> | jq 'select(.event == "GateFailed" or .event == "AgentTurn")'
```

Output:

```json
{ "seq": 12, "event": "AgentTurn",    "data": { "turn": 3, "model": "claude-opus-4-5", "tokens_out": 412 } }
{ "seq": 13, "event": "AgentProducedDiff", "data": { "hash": "d4a1...", "files": ["src/scoring/mod.rs"] } }
{ "seq": 18, "event": "GateFailed",   "data": { "rung": "test", "exit_code": 1,
    "stderr_tail": "thread 'test_score_clamp' panicked at 'assertion failed: score <= 1.0'" } }
```

Root cause: agent turn 3 produced a diff to `src/scoring/mod.rs` that broke the
`test_score_clamp` assertion. The diff hash `d4a1...` can be retrieved for inspection.

### Example: tracking an LLM error back to rate limits

```bash
roko events dump <task-id> | jq 'select(.event | startswith("LLM"))'
```

```json
{ "seq": 5,  "event": "LLMRequest",  "data": { "model": "gpt-4o", "latency_ms": 284 } }
{ "seq": 8,  "event": "LLMRequest",  "data": { "model": "gpt-4o", "latency_ms": 301 } }
{ "seq": 11, "event": "LLMError",    "data": { "model": "gpt-4o", "error_code": "ROKO-L-001",
    "http_status": 429, "retry_after_s": 60 } }
{ "seq": 14, "event": "CircuitOpen", "data": { "service": "llm/openai", "failure_count": 5 } }
```

Root cause: rate limit hit at seq=11, circuit opened at seq=14. All subsequent agent turns
were blocked.

---

## Step 4 — Extract Artefacts for Offline Analysis

```bash
# Extract all diffs produced during the run
roko events dump <task-id> | jq 'select(.event == "AgentProducedDiff") | .data.hash' \
  | xargs -I{} roko diff show {}

# Extract full stderr from a gate failure
roko events dump <task-id> | jq 'select(.event == "GateFailed") | .data.stderr_full'

# Extract all LLM prompts (hashed; full prompts stored separately if prompt_logging = true)
roko events dump <task-id> | jq 'select(.event == "LLMRequest") | .data.prompt_hash'

# Export full event log as JSON array for external analysis
roko events export <task-id> --format json > task-7f3a-events.json
```

### Prompt logging

Full LLM prompts are **not** stored in the event log by default (they can be large and
contain sensitive content). To enable full prompt storage for debugging:

```toml
[agent]
prompt_logging = true    # default: false; logs full prompts to .roko/prompts/<task-id>/
```

With `prompt_logging = true`, retrieve a prompt by hash:

```bash
roko prompt show <prompt-hash>
```

**Security note**: `.roko/prompts/` may contain secrets that appear in system prompts or
tool outputs. Do not commit this directory. Add to `.gitignore`:

```
.roko/prompts/
```

---

## Step 5 — Compare Against a Passing Run

If the failure is a regression (it worked before), compare event logs:

```bash
# Diff event sequences between a passing run and the failing run
roko events diff <passing-task-id> <failing-task-id>

# Output shows which events are present in one but not the other:
# + AgentProducedDiff (in failing run only — new diff introduced)
# - GatePassed rung=test (in passing run; not present in failing)
```

This quickly isolates whether the regression is in:
- A new agent diff (code change broke tests)
- A model change (different model produced different output)
- An infrastructure change (different tool availability)

---

## Replay in Restricted Environments

In production environments where re-execution is expensive, forensic replay is the primary
debugging tool. Key constraints:

1. **Read-only** — forensic replay never modifies state.
2. **Offline-capable** — `roko events` commands work without network access; the log is
   fully self-contained in `.roko/events/`.
3. **Tamper-evident** — chain verification detects any modification to the log after the
   fact. If the chain breaks, investigate log integrity before drawing conclusions from
   the log content.

### Shipping logs to an external collector

```bash
# Stream events to stdout for collection by your log aggregator
roko events dump <task-id> | tee >(curl -s -X POST https://logs.example.com/ingest \
  -H 'Content-Type: application/x-ndjson' --data-binary @-)
```

Or configure a log exporter in `roko.toml` (Specified):

```toml
[observability]
log_exporter = "otlp"
otlp_endpoint = "http://otel-collector:4317"
```

---

## Quick Reference — Forensic Replay Commands

| Command | Purpose |
|---|---|
| `roko events verify --task <id>` | Verify hash chain integrity |
| `roko events show <id>` | Human-readable timeline |
| `roko events show <id> --type <E1,E2>` | Filter by event type |
| `roko events dump <id>` | Raw JSONL to stdout |
| `roko events export <id> --format json` | Export as JSON array |
| `roko events diff <id1> <id2>` | Compare two event sequences |
| `roko prompt show <hash>` | Retrieve stored prompt (requires prompt_logging) |
| `roko diff show <hash>` | Retrieve a stored agent diff |

---

## See also

- [03-event-log-replay.md](03-event-log-replay.md) — event log format and hash chain spec
- [04-crash-recovery.md](04-crash-recovery.md) — using replay for crash recovery
- [08-observability.md](08-observability.md) — integrating event data with metrics/alerts
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — forensic replay walkthroughs
