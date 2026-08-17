# XCUT_08: Emit gen_ai.* OpenTelemetry Semantic Conventions

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-08`](../ISSUE-TRACKER.md#xcut-08)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.8
- Priority: **P3**
- Effort: 8 hours
- Depends on: `XCUT_06` (source 19.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

No OpenTelemetry dependency exists anywhere in the workspace (zero `opentelemetry` references in any `Cargo.toml`). Runtime events go to JSONL only via `crates/roko-runtime/src/jsonl_logger.rs`. Native gen_ai.* OTel emission would give six vendor integrations (Datadog, Honeycomb, Langfuse, Phoenix, Langtrace, Grafana) for ~200 LOC.

## Exact Changes

1. Add `opentelemetry = "0.28"` and `opentelemetry-otlp = "0.28"` to `crates/roko-runtime/Cargo.toml` behind an `otel` feature flag.
2. Create `otel.rs` module with `init_otel(endpoint: &str, protocol: &str) -> TracerProvider`.
3. Define attribute mapping to gen_ai.* v1.37+ conventions:
   - `gen_ai.provider.name` from `ProviderKind::label()`
   - `gen_ai.operation.name` = "chat" | "execute_tool" | "retrieval"
   - `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`
   - `gen_ai.usage.cache_read.input_tokens`
   - `gen_ai.conversation.id` from session ID
4. In `ModelCallService::call()`, create a span per model call with gen_ai.* attributes.
5. In `WorkflowEngine`, create parent spans for workflow runs.
6. Add `[observability]` section to `roko.toml` config schema: `provider`, `endpoint`, `protocol`.
7. Ensure OTel export is off by default (opt-in via config or feature flag).

## Design Guidance

Use a Cargo feature flag `otel` so the dependency is optional. This keeps the default binary lean. The `init_otel` function should be called from `roko_runtime::logging::init()` when the OTel config section is present. The `TracerProvider` should be stored in a global `OnceLock` for access from `ModelCallService`.

## Write Scope

- `crates/roko-runtime/src/otel.rs`
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-runtime/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] With `[observability] endpoint = "http://localhost:4317"` and `--features otel`, spans are exported via OTLP
- [ ] Each model call produces a span with `gen_ai.provider.name` and `gen_ai.usage.*` attributes
- [ ] Without the feature or config, zero overhead (no OTel initialization)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `[observability] endpoint = "http://localhost:4317"` and `--features otel`, spans are exported via OTLP
- Each model call produces a span with `gen_ai.provider.name` and `gen_ai.usage.*` attributes
- Without the feature or config, zero overhead (no OTel initialization)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
