# 07 — Subagent Task-Authoring Notes

> Supplemental implementation-detail ledger for the generated executable backlog.
> These notes were consumed while expanding the per-epic `plans/E*/tasks.toml`
> files to full checklist coverage. Keep them as provenance and as review notes
> for future task refinement, not as an open authoring queue.

## Status

- The generated plan layer in `plans/` now contains all 149 implementation tasks from
  `05-MASTER-CHECKLIST.md`.
- `plans/status-quo-authoring-gaps/tasks.toml` is superseded and all 96 authoring tasks are skipped.
- These notes are advisory refinements: they document stale paths, dependency shape, and unsafe
  task scopes that were applied during task-file expansion.

## Global Corrections

- Put cross-epic and pre-existing-plan prerequisites in `depends_on_plan`, not local `depends_on`.
  Examples: `E01`, `E02`, `P16-safety-contracts`, `P19-cascade-router-acp`, `P22-acp-tool-permission`.
- Use runtime tier names only: `mechanical`, `focused`, `integrative`, `architectural`.
- Use runtime roles only: `implementer`, `researcher`, `strategist`, `architect`, `reviewer`,
  `quick-reviewer`, `scribe`. Map prose-only roles like `refactorer` to `implementer`.
- Generated validation commands must include `--bin roko`: `cargo run -p roko-cli --bin roko -- ...`.
- For task authoring work, never satisfy a gap task with placeholder verify commands such as
  `true`, `echo ok`, or compile-only checks unrelated to the target behavior.

## High-Value Corrections By Epic

### E01

- `E01-T08` should target `crates/roko-cli/src/runner/gate_dispatch.rs` and
  `crates/roko-gate/src/rung_dispatch.rs`; `crates/roko-cli/src/runner/rung_dispatch.rs` does not
  exist.
- `E01-T05` should include `crates/roko-cli/src/commands/plan.rs`,
  `crates/roko-cli/src/runner/types.rs`, and `crates/roko-core/src/defaults.rs` in addition to
  the event loop.
- `E01-T07` should depend on both `E01-T04` and `E01-T05`; worktree isolation is unsafe before the
  real intra-plan DAG and concurrency cap are in place.
- `E01-T10` should either wait for `E01-T06` and `E01-T08` or split docs so it does not claim
  replan/gate-enrichment work is already complete.

### E02

- `E02-T09` is no longer a pure "writer absent" task; runtime-events writers are partially wired.
  Re-scope it to a route-contract/test decision around `routes/runs.rs`, `shared_runs.rs`,
  `state.rs`, and `roko-runtime/src/jsonl_logger.rs`.
- `E02-T10` is stale as written: `state/run-state.json` and `roko_runtime::RunLedger` are live at
  current HEAD. Re-scope toward `state/events.json` / `event_log_snapshot` unless a later snapshot
  migration supersedes both.
- `E02-T07` should wait for `E03-T06` if the shared `RetentionPolicy` consolidation is enforced first.
- `E02-T11` should depend on `E02-T01`, `E02-T03`, `E02-T04`, and `E02-T05`; `P24-T3` is a soft
  doctor-pattern reference, not a local task.

### E03

- `E03-T04` has four current `DashboardSnapshot` definitions, including
  `crates/roko-cli/src/commands/dashboard.rs`; include that file in the rename task.
- `E03-T05` should focus on remaining local file-scraper/fallback models after T04; connected-mode
  watch plumbing already exists.
- `E03-T06` should make `E02-T07` depend on it if retention policy consolidation is sequenced first.

### E04/E05

- `E04-T06` should target the default Claude-CLI runner path in `runner/event_loop.rs`,
  `runner/agent_stream.rs`, and `dispatch_v2.rs`; `P16-safety-contracts` is a cross-plan
  prerequisite, not a local dependency.
- `E04-T12` can be local-independent: add the `PermissionRequest` event/reply channel first, then
  have `E04-T13` answer it and `E04-T14` gate builtin execution.
- `E05-T05` should absorb the useful P14 test intent but not depend on P14's stale legacy-engine
  implementation.
- `E05-T08` should represent E02 as a cross-plan prerequisite. Add `E05-T06` if published verdicts
  include canonical rung mapping.

### E06/E07/E08

- `E06-T08` should use `crates/roko-core/src/config/schema.rs`, not `crates/roko-core/src/config.rs`.
- `E06-T07` is branch-gated by the ADR: either warm VCG from observations or downgrade `Auto`.
- `E07-T09` should depend on `E07-T02` locally and `P19-cascade-router-acp` as a plan prerequisite.
- `E08-T06` must include `crates/roko-cli/src/runner/persist.rs`; conductor circuit-breaker state has
  to round-trip through snapshot/resume.
- `E08-T07` should update `docs/v2/19-CONFIG.md` and `docs/v2/INTEGRATION-GUIDE.md`; there is no
  `crates/roko-cli/CLAUDE.md`.

### E09/E10/E11

- `E09-T04`, `E09-T05`, `E09-T06`, and `E09-T08` can be local roots; E01 is rollout context, not a
  local dependency.
- `E09-T09` must depend on `E09-T01..T08`; do not make it depend on E13, because E13 consumes this
  design.
- `E10-T05` should use `depends_on_plan = ["E03"]`, not local `depends_on`.
- `E10-T07` is independent server-side replay parsing and should not depend on `E10-T05`.
- `E11-T04` is mutually exclusive with `E03-T07`; if E03 lands first, close E11-T04 as done-by-E03.

### E12/E13

- `E12-T04` should be a net-reduction task for `#[allow(dead_code)]`, not a require-zero task.
- `E12-T06` is unsafe as a single delete task: split into duplicate-safety deletion after E04 and
  full crate deletion only after exported live primitives are moved/replaced.
- `E12-T09` cannot be a straight deletion yet; `roko-plugin` has live consumers. Split into
  audit/migrate/delete.
- `E13-T03` is orthogonal to Lens work; keep it local-root and use `depends_on_plan` only for the
  E01 naming/engine decision context.

### E14/E15

- `E14-T05` does not need to depend on `E14-T04`; Gemini CLI wiring is independent of native Gemini
  API streaming unless the implementation deliberately reuses that backend.
- `E14-T06` should include ACP/message-building and backend request files. `translate/gemini.rs` is
  tool-call translation, not the message/image translation surface.
- `E14-T07` can assert default advertised-handler parity after `E14-T01` if `E14-T02/T03` feature-gate
  non-executable tools instead of implementing all handlers.
- `E15-T4` needs a local/shared Claude `mcpServers` shape serializer because E15-T1's helper is private
  in `roko-cli`.
- `E15-T5` must account for the current consumer deserializing `readOnly`/`openWorld`, not necessarily
  `readOnlyHint`/`openWorldHint`.

### E17/E18

- `E17-T04` should depend on `P22-acp-tool-permission` as a plan prerequisite and should derive
  `ToolPermission` from session/role consent instead of all-true defaults in `bridge_events.rs`.
- `E17-T05` should depend locally on `E17-T04` and on `P28-image-support` as a plan prerequisite.
  Its target surface is `crates/roko-acp/src/handler.rs` plus `crates/roko-acp/src/types.rs`.
- `E17-T06` should depend on `E17-T05` in addition to `E17-T01..T04`; the conformance test checks
  advertised capabilities from T04/T05.
- `E18-T04..T09` should be independent implementation tasks. The generated authoring-gap plan
  serializes their task-block authoring to avoid editing the same plan file concurrently, but the
  implementation DAG should not inherit that serialization.
- `E18-T08` should use `.github/workflows/deploy-fly.yml`, root `fly.toml`, and the
  `FLY_TOML_TEMPLATE` in `crates/roko-cli/src/commands/server.rs`; prose references to
  `deploy-fly.yml` are incomplete.
- `E18-T10` should gate on `depends_on_plan = ["E01-execution-engine"]` and local `E18-T05..T08`.
- `E18-T12` rewrite scope and `E18-T13` grep scope must match. If docs-lint greps all `docs/`,
  then T12 must broaden beyond `docs/v2`; otherwise prefer the narrower lint scope:
  `README.md CLAUDE.md docs/v2 docker/README.md`.
- `E18-T13` should depend on `E18-T12` as well as `E18-T10` and `E18-T11`.
