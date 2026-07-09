# UX Follow-up Batches

47 batches closing every open P0/P1 item in `tmp/ux-followup/` (the post-PR-13
catalogue). Grouped into 8 phases; each phase is a reasonable one-night slice.

## Dependency graph

```
selfhost (P0 — CLAUDE.md items 10–11):
  UX01 (gate-feedback loop)  ──► UX02 (PRD-publish event)
                                   │
                                   ├─► UX04 (plan validate CLI)
                                   │
                                   └─► UX03 (E2E self-hosting smoke)

tui-stream (P0/P1 — eliminate TUI polling):
  UX05 (in-process StateHub)  ┐
  UX06 (notify file-watcher)  ┼─► UX07 (incremental tailers)
                              │   UX08 (task-output watcher)
                              │
                              ├─► UX09 (agent WS consumer)
                              └─► UX10, UX11 (git watch, backpressure)

state:
  UX12 (snapshot_version + migrate) ──► UX13 (resume validation)
  UX14 (ProcessSupervisor SIGTERM + Drop)           — independent

observ:
  UX15 (verdicts widget)   — deps on UX24
  UX16 (diagnosis panel)    — deps on UX05
  UX17 (efficiency trend)   — independent
  UX18 (metrics alignment)  — independent
  UX19 (experiments widget) — deps on UX05
  UX20 (topology widget)    — deps on UX05
  UX21 (sidecar /logs)      — independent
  UX22 (c-factor trend)     — deps on UX17

wired:
  UX23 (gate rungs)         — independent
  UX24 (playbook query)     — independent
  UX25 (HDC fingerprint)    — independent
  UX26 (safety contracts)   — independent
  UX27 (role whitelist)     — independent
  UX28 (Enriching-phase wiring) — independent
  UX29 (phase2 reality + mcp audit) — independent

backends:
  UX30 (Codex harness)     ┐
                           ├─► UX32 (test parity) ──► UX34 (cascade tests)
  UX31 (Cursor streaming)  ┘
  UX33 (backend docs + dir cleanup)

hygiene:
  UX35 (adaptive threshold)    — independent
  UX36 (roko.toml keys)        — independent
  UX38 (unwrap cleanup)       ─┐
                                ├─► UX37 (prompt-builder snapshot tests)
                                └─► UX39 (HTTP validation)
  UX40 (episode backend field) — independent
  UX41 (coverage scaffold)      — independent
  UX42 (clippy + flakes)        — independent

docs:
  UX43 (MORI parity regen)     — independent
  UX44 (smoke tests)            — deps on UX03
  UX45 (terminology + sidecar)  — independent
  UX46 (plans + Mori sidecar)   — independent
  UX47 (tui-parity hardening)   — independent
```

## Serial execution order (ALL_BATCHES in lib/common.sh)

```
UX01 UX02 UX03 UX04                                  # selfhost
UX05 UX06 UX07 UX08 UX09 UX10 UX11                   # tui-stream
UX12 UX13 UX14                                       # state
UX15 UX16 UX17 UX18 UX19 UX20 UX21 UX22              # observ
UX23 UX24 UX25 UX26 UX27 UX28 UX29                   # wired
UX30 UX31 UX32 UX33 UX34                             # backends
UX35 UX36 UX37 UX38 UX39 UX40 UX41 UX42              # hygiene
UX43 UX44 UX45 UX46 UX47                             # docs
```

## Batch manifest

| Batch | Title | Group | Deps | Catalog refs | ~LOC |
|-------|-------|-------|------|--------------|------|
| UX01 | Gate-failure → plan-revision feedback loop (self-hosting P0) | selfhost | — | 06, 89 | ~500 |
| UX02 | PRD-publish event → orchestrator auto-trigger (self-hosting P0) | selfhost | UX01 | 05, 51, 90 | ~400 |
| UX03 | End-to-end self-hosting smoke test | selfhost | UX01 UX02 UX04 | 60, 26, 43 | ~300 |
| UX04 | roko plan validate CLI command | selfhost | — | 12 | ~300 |
| UX05 | Standalone TUI: in-process StateHub + delete polling fallback | tui-stream | — | 68 | ~250 |
| UX06 | Notify file-watcher replaces 500 ms .roko/ polling thread | tui-stream | — | 69, 76 | ~350 |
| UX07 | Incremental tailers for signals / episodes / events | tui-stream | UX06 | 71, 73, 74 | ~400 |
| UX08 | Task-output directory watcher + per-file incremental tail | tui-stream | UX06 | 72 | ~250 |
| UX09 | Agent sidecar /stream WebSocket consumer on Agents tab | tui-stream | — | 70 | ~400 |
| UX10 | Git view fs-watch replaces 3 s git CLI polling | tui-stream | — | 75 | ~200 |
| UX11 | TUI channel backpressure + durable dashboard-gen counter | tui-stream | — | 77, 78 | ~200 |
| UX12 | ExecutorSnapshot schema_version + migration framework | state | — | 79, 81, 60d | ~350 |
| UX13 | Resume: validate plan-discovery vs snapshot consistency | state | UX12 | 82 | ~150 |
| UX14 | ProcessSupervisor SIGTERM escalation + Drop + CancellationToken | state | — | 80, 60e, 18 | ~400 |
| UX15 | Verdicts substrate reader + per-gate trend widget | observ | UX24 | 35c, 83, 87 | ~450 |
| UX16 | Conductor diagnosis TUI panel + HTTP endpoint | observ | UX05 | 10, 31, 84 | ~400 |
| UX17 | Efficiency-events trend aggregator + Learning sparkline | observ | — | 85 | ~300 |
| UX18 | Metrics schema alignment: roko-core vs roko-agent-server | observ | — | 35, 86 | ~300 |
| UX19 | Prompt experiment winners on Learning tab | observ | UX05 | 20, 88 | ~250 |
| UX20 | Agent topology TUI widget | observ | UX05 | 14 | ~300 |
| UX21 | Sidecar /logs endpoint + aggregator proxy | observ | — | 13 | ~350 |
| UX22 | GET /api/c-factor/trend endpoint + trend widget | observ | UX17 | 09 | ~300 |
| UX23 | Gate pipeline: wire remaining 4 rungs in run_gate_rung | wired | — | 35a | ~450 |
| UX24 | Playbook store query seam (dispatch / prompt builder) | wired | — | 35b, 94 | ~350 |
| UX25 | HDC fingerprint per-episode | wired | — | 11, 30, 93 | ~200 |
| UX26 | Safety contract enforcement wiring | wired | — | 35d, 91 | ~400 |
| UX27 | Role-based tool whitelist enforcement | wired | — | 35e, 92 | ~300 |
| UX28 | Enrichment pipeline Enriching-phase wiring | wired | — | 29, 95 | ~250 |
| UX29 | Phase-2 build-surface reality check + MCP audit | wired | — | 32, 33, 34 | ~200 |
| UX30 | Codex backend conformance test harness | backends | — | 36 | ~500 |
| UX31 | Cursor backend streaming path wiring | backends | — | 37 | ~400 |
| UX32 | Backend test parity (happy/stream/tool/error/session) | backends | UX30 UX31 | 38 | ~600 |
| UX33 | ExecAgent vs ClaudeCliAgent docs + backend dir cleanup | backends | — | 39, 40 | ~400 |
| UX34 | Cascade router + model router integration tests | backends | UX32 | 40a, 60c | ~300 |
| UX35 | Adaptive gate thresholds load-path audit + wiring | hygiene | — | 08, 48a | ~200 |
| UX36 | roko.toml unused keys: consume or remove | hygiene | — | 48b | ~300 |
| UX37 | SystemPromptBuilder 6-layer snapshot tests | hygiene | UX38 | 19 | ~300 |
| UX38 | Top-10 unwrap() cleanup | hygiene | — | 55 | ~500 |
| UX39 | HTTP route validation + OpenAPI surface | hygiene | UX38 | 60a | ~600 |
| UX40 | Episode struct: explicit backend dispatcher field | hygiene | — | 60b | ~150 |
| UX41 | cargo llvm-cov coverage scaffold | hygiene | — | 59 | ~100 |
| UX42 | clippy missing_* doc sweep + timeout-flake audit | hygiene | — | 56, 58 | ~400 |
| UX43 | MORI-PARITY-CHECKLIST mechanical regeneration tool | docs | — | 46 | ~300 |
| UX44 | Smoke tests for CLAUDE.md 'What to work on' items 1-9 | docs | UX03 | 45 | ~400 |
| UX45 | Terminology sweep + stale-snapshot sidecar | docs | — | 47, 64, 65, 66 | ~150 |
| UX46 | tmp/implementation-plans refresh + Mori path-corrected sidecar | docs | — | 67, 67a | ~100 |
| UX47 | tui-parity runner hardening + CI dry-run + log retention | docs | — | 27, 28, 27a, 28a | ~250 |

## Verification gates (summary)

| Batch | Verify commands |
|-------|-----------------|
| UX01, UX02 | `cargo check -p roko-cli -p roko-orchestrator -p roko-runtime -p roko-learn` + `cargo clippy` |
| UX03 | `cargo test -p roko-cli --test e2e_self_host --no-run` |
| UX04 | `cargo test -p roko-cli --lib --no-run -- plan::validate` + clippy |
| UX05–UX11 | `cargo check -p roko-cli` + `cargo clippy -p roko-cli --no-deps -- -D warnings` |
| UX12, UX13 | `cargo test -p roko-cli --lib --no-run -- snapshot` + clippy |
| UX14 | `cargo test -p roko-runtime --lib --no-run` + clippy (roko-runtime + roko-cli) |
| UX15–UX22 | `cargo check -p roko-cli -p roko-serve` + clippy |
| UX23 | `cargo test -p roko-gate --lib --no-run` + clippy |
| UX24 | `cargo test -p roko-learn --lib --no-run -- playbook` + clippy |
| UX25 | `cargo test -p roko-learn --lib --no-run -- episode_logger` + clippy |
| UX26, UX27 | `cargo test -p roko-agent --lib --no-run -- safety` + clippy |
| UX28 | `cargo test -p roko-compose --lib --no-run -- enrichment` + clippy |
| UX29 | root `cargo check` + workspace `cargo check` + workspace-wide clippy + audit artifact |
| UX30–UX34 | `cargo test -p roko-agent --lib --no-run` + clippy |
| UX35 | `cargo test -p roko-gate --lib --no-run -- adaptive_threshold` + clippy |
| UX36 | `cargo test -p roko-cli --lib --no-run -- agent_config` + clippy |
| UX37 | `cargo test -p roko-compose --lib --no-run -- system_prompt_builder` + clippy |
| UX38 | `cargo clippy -p roko-compose -p roko-serve -p roko-gate -p roko-runtime --no-deps -- -D warnings` |
| UX39 | `cargo test -p roko-serve --lib --no-run` + clippy |
| UX40 | `cargo test -p roko-learn --lib --no-run -- episode_logger` + clippy |
| UX41 | coverage workflow + helper both contain `cargo llvm-cov` |
| UX42 | `cargo clippy --workspace --no-deps -- -D warnings -D clippy::missing_errors_doc -D clippy::missing_panics_doc` |
| UX43 | `tools/mori-parity-check.sh` executable |
| UX44 | `cargo test -p roko-cli --test smoke --no-run` |
| UX45 | grep sweep + bardo-backup sidecar |
| UX46 | current Mori sidecar + refreshed implementation-plan index |
| UX47 | `TUI_PARITY_MAX_BATCHES` env var honoured + `--dry-run` CI step |

## Conflict groups

Batches in the same write-scope group should not run in parallel against the
same worktree; the runner enforces this via the dep DAG. If you hand-edit
`--only`, respect these clusters:

- **tui-stream**: UX05–UX11 all touch `crates/roko-cli/src/tui/{app.rs, dashboard.rs, state.rs}`
- **state**: UX12–UX14 touch `crates/roko-cli/src/orchestrate.rs` + `crates/roko-runtime/`
- **observ**: UX15–UX22 touch `crates/roko-cli/src/tui/` + `crates/roko-serve/src/routes/`
- **wired**: UX23–UX29 fan out across `roko-gate`, `roko-learn`, `roko-agent`, `roko-compose`
- **backends**: UX30–UX34 all touch `crates/roko-agent/src/`
- **hygiene**: UX38–UX39 both heavy on `crates/roko-serve/`
