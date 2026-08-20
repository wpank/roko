# Dogfood Debrief: 2026-08-17 Examples & CLI Surface Audit

> Comprehensive end-to-end exercise of the `examples/` directory, graph subsystem,
> CLI command surface, and supporting subsystems. 68 log files captured.

## Executive Summary

**The core workflow commands work well. The examples and peripheral CLI surface have
significant issues that would block or confuse a new user.**

- **8 graph examples**: 5 pass validation, 3 fail (schema drift)
- **Graph execution**: 4 graphs run successfully; 1 fails due to workspace test failures (expected)
- **~40 CLI commands tested**: 33 succeed, 7 produce errors or misleading output
- **6 assessment agents** audited: example configs, CLI UX, graph schema, architecture violations, doctor/onboarding UX, learning subsystems
- **Total findings**: 48 issues across 6 categories

## Test Results: Graph Examples

| Graph | Validate | Run | Issue |
|---|---|---|---|
| `single-gate.toml` | PASS | PASS (6.4s) | None |
| `linear-gates.toml` | PASS | FAIL (255s) | `cargo test` exit 101 — real failing tests in workspace (expected) |
| `parallel-gates.toml` | **FAIL** | — | `missing field 'graph'` — uses bare `name =` instead of `[graph]` table |
| `score-compose.toml` | PASS | PASS (68µs) | Stub cells, no real work |
| `task-execution.toml` | **FAIL** | — | `unknown variant 'on_success'` — should be `success` |
| `cognitive-loop.toml` | PASS | PASS (195µs) | All 7 cells are PassthroughCell stubs; no user-facing warning |
| `observed-cost.toml` | PASS | PASS (66µs) | Stub cell, telemetry projections display correctly |
| `conditional-branch.toml` | **FAIL** | — | `unknown variant 'on_success'` + uses `when` conditions not in loader |

### Root Cause: Two Different Condition Schemas

The broken examples use `type = "on_success"` / `type = "on_failure"` / `type = "when"` which
belong to `condition::Condition` (the engine's internal evaluator). The TOML graph loader uses
`RawEdgeCondition` which only supports: `Success`/`success`, `Failure`/`failure`, `Always`/`always`,
`OutputEquals`/`output_equals`.

The `parallel-gates.toml` also uses bare top-level `name =` instead of a `[graph]` table, and
references `cell_type = "gate"` (generic) which doesn't exist — only `gate.compile`, `gate.test`,
`gate.clippy` are registered.

## Test Results: CLI Commands

### Working Commands (33)

| Command | Result |
|---|---|
| `roko status` | Healthy, 86 signals, 78 episodes |
| `roko doctor` | 21 ok, 6 warn, 0 fail, 2 skip |
| `roko doctor disk` | 147GB free, .roko 52MB |
| `roko config show` | Works but models field dumps raw Debug output |
| `roko config validate` | 0 warnings, 0 errors |
| `roko config providers list` | 9 providers listed |
| `roko config models list` | 14 configured + 5 builtin |
| `roko plan list` | 42 plans listed |
| `roko plan show demo-hello` | Works (without `plans/` prefix) |
| `roko plan validate plans/demo-hello` | 0 diagnostics |
| `roko plan validate plans/demo-*` (6 demos) | All pass |
| `roko prd list` | 1 published, 7 drafts, 9 ideas |
| `roko knowledge stats` | 135 entries, avg confidence 0.493 |
| `roko knowledge query "routing"` | 2 matches returned |
| `roko learn all` | Router + experiments + efficiency + episodes |
| `roko learn episodes` | 78 entries |
| `roko learn efficiency` | 84 events |
| `roko explain gates` | Works with depth levels |
| `roko explain topics` | 10 topics listed |
| `roko note "..."` | Saved to .roko/notes/ |
| `roko show costs` | Cost breakdown by model/task/day |
| `roko show learning` | Routing + experiments + gates |
| `roko show history` | Recent turns listed |
| `roko show knowledge` | 135 entries, recent listed |
| `roko show agents` | 57 agent sessions listed |
| `roko layer-check` | 31/35 crates, 8 violations found |
| `roko config events` | Cron/watcher config listed |
| `roko job list` | No jobs (expected) |
| `roko recipe list` | No recipes (expected) |
| `roko agent list` | No agents (expected) |
| `roko trigger list` | No triggers (expected) |
| `roko feed list` | Correctly says serve not running |
| `roko graph show` (5 valid graphs) | All display correctly |

### Errors and Issues (7)

| Command | Error | Root Cause |
|---|---|---|
| `roko plan show plans/demo-hello` | `plan not found` | Path prefix `plans/` not stripped; must use bare `demo-hello` |
| `roko explain signals` | `unknown topic: signals` | Topic registered as `"engram"`, not `"signal"` post-rename |
| `roko learn router` | `unrecognized subcommand` | Subcommand is `route`, not `router` (but docs say `router`) |
| `roko show signals` | `no work item 'signals' found` | Falls through to work-item lookup; misleading error |
| `roko show episodes` | Same misleading error | Same — `show` treats unknown subjects as work IDs |
| `roko market list` | `unrecognized subcommand` | No `list`; only `browse` (but `browse` returns "not yet implemented") |
| `roko history list` | `session not found: list` | `history` takes optional ID, not subcommands; `list` parsed as session ID |

### Warnings and Data Quality Issues

| Observation | Details |
|---|---|
| `show plans` says "No plans found" | Despite `plan list` showing 42 plans — the `show plans` path has a different loader |
| `show costs` / `show plans` / `show *` all emit | `runner error: lifecycle contains more tasks than total_tasks` — stale snapshot validation |
| `config show` models dump | Raw `ModelProfile { provider: "...", slug: "...", ... }` Debug output |
| `e2e-smoke` plan validation | 2 warnings: tasks use `claude-haiku-4-5` which is not configured |
| `config providers list` | perplexity shows "base URL missing" despite having hardcoded default |
| `config models list` | Claude builtins show "missing (ANTHROPIC_API_KEY)" even when claude_cli works |
| Efficiency pass rate | 0% — structurally broken; `gate_passed` is always `None` at event emit time |

## Assessment Agent Findings

### A. Example Config Quality (16 findings)

| # | Sev | Issue |
|---|---|---|
| 1 | HIGH | `adding-custom-tools.md` contains hardcoded absolute paths to non-existent `roko-mr-stream-beta/` directory |
| 2 | MED | 6 example TOML files use `[prompt].token_budget` and `[prompt].role` — fields that don't exist in `PromptConfig`; silently ignored |
| 3 | MED | `adding-a-custom-protocol.md` ProviderKind enum shows 6 of 11 real variants (missing GeminiCli, CerebrasApi, CursorCli, Hermes, OpenClaw) |
| 4 | MED | No `roko-anthropic-api.toml` example for the most common CI alternative (`kind = "anthropic_api"`) |
| 5 | MED | No `roko-cerebras.toml` despite CerebrasApi being a registered provider |
| 6 | MED | `roko-gemini.toml` uses 3 Gemini 3.x preview model slugs with no availability caveat |
| 7 | MED | `[gemini].default_model` etc. use model map keys, not API slugs — runtime may resolve to wrong model |
| 8 | LOW | All example TOML files omit `config_version = 2`; loader emits spurious migration warnings |
| 9 | LOW | `roko-glm.toml` and `roko-ollama.toml` have `fallback_model` referencing models not defined in the same file |
| 10 | LOW | Curl examples use port 9090 but default serve port is 6677 |
| 11 | LOW | No example showing `[[gate]]` shell overrides |
| 12 | LOW | No example showing `[routing]` section configuration |
| 13 | LOW | Demo `provider-routing/roko.toml` uses stale slug format (`claude-sonnet-4-20250514`) |
| 14 | LOW | Demo uses `max_cost_per_task` / `max_cost_per_session` which don't exist in BudgetConfig |
| 15 | LOW | Demo uses `[routing].fast`/`standard`/`complex` shorthand; real fields are `fast_task_model` etc. |
| 16 | LOW | No `roko-gemini-cli.toml` for GeminiCli subprocess path |

### B. CLI UX Consistency (8 findings)

| # | Sev | Issue | Fix |
|---|---|---|---|
| 1 | HIGH | `roko learn router` fails; subcommand is `route`, but CLAUDE.md and help text say `router` | Add `#[command(alias = "router")]` |
| 2 | HIGH | `roko plan show plans/demo-hello` fails; path prefix not stripped | Strip leading `plans/` before lookup |
| 3 | MED | `roko show signals` gives "no work item" error instead of suggesting `roko status` | Improve error message with valid subjects |
| 4 | MED | `roko market list` doesn't exist; inconsistent with `plan list`, `job list`, etc. | Add `#[command(alias = "list")]` to `Browse` |
| 5 | MED | `roko config show` dumps raw `{:?}` Debug for models HashMap | Use `toml::to_string_pretty` or count summary |
| 6 | MED | `roko explain signals` fails post-rename; topic is `"engram"` | Add `"signal"` / `"signals"` aliases |
| 7 | MED | Help text at line 236 lists `router` under `learn` but CLI token is `route` | Sync help text or add alias |
| 8 | LOW | `roko history list` — `list` parsed as session ID | Document or reject gracefully |

### C. Graph Schema (5 findings)

| # | Sev | Issue |
|---|---|---|
| 1 | HIGH | 3 example graphs use wrong TOML schema (bare top-level, wrong condition types) |
| 2 | MED | `conditional-branch.toml` uses `when`/`Gte`/`Lt` conditions not wired into TOML loader |
| 3 | MED | `parallel-gates.toml` and `task-execution.toml` use generic `cell_type = "gate"` / `"agent"` not registered in default_registry |
| 4 | LOW | `cognitive-loop.toml` runs 7 stub cells with no user-facing warning they are no-ops |
| 5 | LOW | `linear-gates.toml` test gate failure is expected (dirty workspace) but no explanation in output |

### D. Architecture Violations (5 findings from `layer-check`)

| # | Sev | Issue | Location |
|---|---|---|---|
| 1 | LOW | 4x `Command::new("claude")` — all are binary presence probes, not model dispatch (false positives) | auth_detect.rs:146, bootstrap.rs:120, doctor.rs:807,978 |
| 2 | MED | 4x `model: String::new()` in test code — `DashboardEvent.model` should be `Option<String>` | state_hub.rs:2069,2249,2284,2700 |
| 3 | LOW | Same `claude --version` probe pattern copied 4 times across 3 files | Consolidate to one helper |
| 4 | LOW | layer-check rule doesn't distinguish test code from production code | Add `#[cfg(test)]` exclusion |
| 5 | LOW | layer-check rule doesn't exclude `--version` probe idiom from direct-subprocess check | Add pattern exclusion |

### E. Doctor / Onboarding UX (7 findings)

| # | Sev | Issue |
|---|---|---|
| 1 | HIGH | `config models list`: builtin Claude models show "missing (ANTHROPIC_API_KEY)" even when claude_cli is configured and working |
| 2 | MED | `config providers list`: perplexity_api shows "base URL missing" despite having hardcoded default in the dispatch backend |
| 3 | MED | `doctor` warns about 3 missing API keys even when a working CLI provider is the primary; no cross-check |
| 4 | MED | `roko setup` wizard doesn't write `[providers.*]` entries even when API keys are detected in env |
| 5 | LOW | `target_staleness` warns at 10GB threshold with no disk-space context; 179GB target in active dev is normal |
| 6 | LOW | `plans_dir_conflict` fix command (`mv .roko/plans/* plans/`) can silently clobber |
| 7 | LOW | `CursorAcp` exclusion from provider key check is undocumented |

### F. Learning / Knowledge Subsystems (7 findings)

| # | Sev | Issue |
|---|---|---|
| 1 | **HIGH** | **Efficiency pass rate is structurally 0%**: `gate_passed` is always `None` at `TurnCompleted` event emit time; gate result arrives as separate event but efficiency already written |
| 2 | MED | Cascade router: 59 observations but only 3 models have any confidence-stats data; stage transitioned to "confidence" based on global counter, not per-model readiness |
| 3 | MED | 11 stale `.tmp` files in `.roko/learn/` from crash-interrupted atomic writes; plus a `cascade-router.json.corrupted` file |
| 4 | MED | Knowledge store: 67/135 entries are content-free execution receipts that dilute signal |
| 5 | LOW | Gate thresholds: only 3 of 7 rungs tracked; format, clippy, test, LLM-judge rungs have never fired |
| 6 | LOW | 52/62 dream entries stuck at Raw/Replayed confidence (0.10-0.30); validation pipeline progressing slowly |
| 7 | LOW | State snapshot validation error on every `show *` command: "lifecycle contains more tasks than total_tasks" |

## Stale Snapshot Validation Error

Every `roko show *` command emits:
```
runner projection: invalid
runner error: validate run_state_json embedded in state-snapshot.json: lifecycle contains more tasks than total_tasks
```

This means the state snapshot from a previous run has inconsistent data. The `show` commands
still work (they fall back to file-based data), but the runner projection is unavailable. This
is a pre-existing data integrity issue from earlier plan runs.

## Priority Summary

### P0 — Blocks new users or produces wrong data

1. **Fix 3 broken graph examples** (wrong schema + condition types)
2. **Fix efficiency `gate_passed` propagation** (0% pass rate is structurally broken)
3. **Fix `adding-custom-tools.md`** hardcoded paths to non-existent directory
4. **Fix builtin model key status** (shows "missing" when claude_cli works)

### P1 — Significant UX friction

5. Add `router` alias to `roko learn route`
6. Strip `plans/` prefix in `roko plan show`
7. Add `signal`/`signals` alias to `roko explain` topics
8. Fix `config show` models Debug dump
9. Apply per-kind default base URL in provider list
10. Fix `[prompt]` section fields in 6 example TOML files

### P2 — Polish and completeness

11. Add `anthropic_api` and `cerebras_api` example configs
12. Add `list` alias to `roko market browse`
13. Improve `show <unknown>` error messages
14. Downgrade doctor API key warnings when CLI provider works
15. Clean up stale `.tmp` files in `.roko/learn/`
16. Add `config_version = 2` to all example configs
17. Fix state snapshot validation error
18. Add stub cell warnings in graph execution output

## Performance Observations

| Operation | Time |
|---|---|
| `roko doctor` | <1s |
| `roko config show` | <1s |
| `roko status` | <1s |
| `roko plan list` (42 plans) | <1s |
| `roko graph validate` | <1s |
| `roko graph run single-gate` | 6.4s (cargo check) |
| `roko graph run linear-gates` | 255s (compile + test + clippy) |
| `roko graph run score-compose` | 68µs (stub cells) |
| `roko graph run cognitive-loop` | 195µs (stub cells) |
| `roko graph run observed-cost` | 66µs (noop cell + telemetry) |

## Raw Logs

68 log files captured in `tmp/dogfood-2026-08-17-examples/`:
- `01-08`: Graph validation (8 graphs)
- `09-13`: Graph show (5 valid graphs)
- `14-18`: Graph run (5 graphs)
- `19-27`: Config, doctor, plan validation
- `28-37`: Status, plan list, providers, models, explain, knowledge, learn, prd, doctor disk
- `38-68`: Plan show, explain topics, config show, note, feeds, triggers, events, layer-check, experiments, learn route/episodes/efficiency, show costs/plans/learning/history/knowledge/agents, market, history
