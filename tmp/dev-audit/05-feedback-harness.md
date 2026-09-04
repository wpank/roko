# Feedback harness

## Goal

Claude, Codex, Roko, and a human should be able to answer in seconds:

- What run is this?
- What exact code/config/model was used?
- What phase is slow or stuck?
- Which process and command owns the time?
- What did the agent change?
- Which endpoint/CLI/TUI behavior was actually observed?
- Did exactly one terminal result occur?
- Where are the logs and screenshots for this run only?

The harness is the verification owner in FAST mode.

## Command surface and status

Extend the existing dev.sh rather than establishing a second permanent toolkit:

Implemented across `a58bdbacb` and expanded evidence commit `bba2f8858`:

    ./dev.sh fast [wrapper-options] <plans-directory> [-- native-roko-options]
    ./dev.sh run-evidence [evidence-options] -- command [args...]
    ./dev.sh evidence-validate .roko/runs/<run-id>
    ./dev.sh feedback --run-id RUN_ID
    ./dev.sh score --bundle-root .roko/runs

Impact selection is integrated into focused runner gates rather than exposed as a separate
`dev.sh impact` command. `score` aggregates existing bundles; automatic cold/warm fixture creation
and real repetitions remain open.

## Run bundle

The schema-v2 harness supplies a private, bounded bundle containing a manifest, redacted command,
machine/cache/resource snapshots, before/after Git state and diffs, stdout/stderr, fresh
run-scoped status and logs, process inventory, timings, lifecycle validation, usage/gate summaries,
safe GET results, optional CLI/text/PNG evidence, metrics, a score, a deterministic debrief, and
strict validation. It caps every artifact and the total bundle, and terminates the process group
on deadline.

Every invocation creates:

    .roko/runs/<timestamp>-<slug>/
      manifest.json
      command.txt
      stdout.log
      stderr.log
      events.jsonl
      status.jsonl
      commands.jsonl
      usage.jsonl
      endpoints.json
      gates.json
      processes.json
      timings.json
      diff.patch
      diff-stat.json
      summary.json
      score.json
      DEBRIEF.md
      screenshots/
        manifest.json
        *.txt
        *.png

manifest.json is written before execution and includes:

- Schema version and run ID.
- UTC and monotonic start.
- Invocation argv.
- Workspace, base commit, branch, and dirty filenames only.
- Roko/Cargo/Rust/Node/model/provider versions.
- Model, effort, tier, prompt hash, selected tools, and redacted config.
- Machine CPU/memory/swap/disk snapshot.
- Cache strategy and target identity.

Never capture raw secrets or the full environment.

## Canonical event envelope

Every event needs:

- schema_version
- run_id
- plan_id
- task_id
- attempt_id
- span_id and parent_span_id
- monotonic_ms and UTC timestamp
- type
- lifecycle state
- structured body

Required spans:

- setup
- maintenance
- cache warm
- plan validation
- capacity wait
- context selection
- prompt assembly
- provider queue
- provider inference
- each tool/command
- gate selection
- each gate
- smoke probe
- commit
- cleanup

Current repeated “spawning” logs demonstrate why queued, preparing, launch_requested, launched,
settled, and terminal must be distinct events.

## Command evidence

commands.jsonl records:

- Stable command ID and owning task/attempt.
- Argv as an array, working directory, and redacted environment keys.
- Start/end/duration.
- PID/process-group ID.
- Exit code, signal, timeout, cancellation, or lost-process classification.
- stdout/stderr artifact paths and bounded previews.
- Cargo target/profile/cache mode, lock-wait time, and selected target.

This allows the harness to cancel a cold compile without killing or losing the patch.

## Endpoint query surface

The integration implements run-scoped APIs:

- GET /api/runs/:run_id
- GET /api/runs/:run_id/events?cursor=&types=
- GET /api/runs/:run_id/tasks/:task_id/attempts
- GET /api/runs/:run_id/gates
- GET /api/runs/:run_id/logs?source=&level=&since=
- GET /api/runs/:run_id/metrics
- GET /api/runs/:run_id/artifacts
- GET /api/runs/:run_id/screenshots
- GET /api/runs/:run_id/bundle
- GET /api/openapi.json
- SSE /api/runs/:run_id/events/stream

They use opaque byte cursors and bounded responses and do not scan the entire global JSONL on
every query. New records have hashed, grammar-checked per-run indexes. The explicit
`roko run-index repair` command now handles pre-index history offline under aggregate
byte/record/deadline budgets; it is dry-run by default, replaces nothing after a truncated scan,
and is never invoked by HTTP or startup.

The safe GET seed list is [endpoints/core-get.txt](endpoints/core-get.txt). Discover additional GET
routes from OpenAPI when available. Never automatically call POST, PUT, PATCH, or DELETE.

## Endpoint collection

For each safe endpoint:

- Record request path, substitutions, start/end, HTTP status, content type, byte count, and
  response artifact.
- Use a two-second default timeout.
- Redact credentials and sensitive fields.
- Bound responses.
- Treat unavailable optional services as explicit skipped evidence, not success.
- Capture a five-second bounded SSE window and validate event IDs/order.

## Screenshot collection

Collect only relevant views:

- TUI text snapshots for dashboard, plans, agents, logs, diff, and gates.
- PNGs for web/browser changes.
- Before/after/diff images when visual behavior changed.
- Manifest entries with run ID, route/tab, dimensions, timestamp, triggering event, and hash.

Screenshots remain change-selected evidence, not mandatory overhead. The harness can import one
unambiguous Roko text-screenshot directory and can run explicit text or PNG adapter hooks. It
records skipped optional surfaces rather than calling them successful. A real final-tree visual
fixture is still required before claiming runtime proof for a visual change.

For stable visual workflows on macOS, Codex record/replay can turn a recorded browser interaction
into reusable automation:
[Codex record and replay](https://learn.chatgpt.com/docs/extend/record-and-replay.md).

## Codex/Claude capture

Codex non-interactive mode can emit JSONL item events and validate a structured final answer:

    codex exec --json --output-schema <schema> <prompt>

Use the JSON stream as a provider artifact, but translate it into Roko's canonical run/attempt
events. Keep raw provider output separately for diagnosis.

Official reference:
[Codex non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode.md).

Use an equivalent structured-output wrapper for Claude. Provider differences should not leak into
the bundle schema.

## Summary validation

The schema-v2 validator implements the cross-artifact checks below. FAST requires a structured
event stream; other callers can promote status, CLI, endpoint, and screenshot evidence from
optional to required. A selected evidence-policy failure returns `125` even when the wrapped
command exits successfully.

A bundle is invalid if any of these is true:

- Missing run ID or manifest.
- Malformed JSON/JSONL.
- Missing run-start or missing/multiple terminal events.
- Task/attempt lifecycle is unbalanced.
- More than one real launch for one attempt without a retry transition.
- Exit code disagrees with terminal outcome.
- A timed-out/killed process is reported successful.
- Changed files exceed the authorized scope.
- A behavior change lacks CLI/API/TUI/browser evidence.
- Screenshots/logs claim another run ID.
- Common secret patterns are present.
- Artifact sizes exceed configured bounds without truncation metadata.

The proposed summary schema is in [schemas/session.schema.json](schemas/session.schema.json).

## Automatic debrief

`feedback` generates `DEBRIEF.md` deterministically from facts, with separate sections:

1. Outcome.
2. Phase timeline.
3. First failure.
4. Changed files/LOC.
5. Verification selected and why.
6. Endpoint/screenshot results.
7. Provider usage/cost/cache.
8. Resource/cache state.
9. Hypotheses, clearly labeled as hypotheses.
10. Recommended next action.

No LLM rewrites measured facts or terminal semantics.

Implementation details and limits are tracked in
[docs/v2/30-EVIDENCE-BUNDLES.md](../../docs/v2/30-EVIDENCE-BUNDLES.md). The remaining proof gap is
representative cold/warm runs and escaped-regression accounting, not another bundle design.
