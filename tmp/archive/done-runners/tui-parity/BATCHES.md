# TUI Parity Batches

19 batches to bring Roko TUI to Mori parity. T1–T8 are the original TUI parity sweep; T9–T19 are the pre-merge polish additions for PR #13.

## Dependency graph

```
T7 (cleanup)          — independent
T8 (effects)          — independent
T1 (StateHub)         — independent (foundation)
T2 (segments)         — independent
  ├── T5 (pool+wave)  — depends on T1
  ├── T3 (approval)   — depends on T1
  │   └── T4 (procs)  — depends on T1, T3
  └── T6 (metrics)    — depends on T1, T2

T9  (messaging)  — independent
T10 (snapshot)   — independent (foundation)
  ├── T11 (plans+failures)     — depends on T10
  ├── T13 (modals)              — depends on T10
  │   └── T14 (modal cleanup)  — depends on T13
  └── T16 (fields cleanup)     — depends on T10, T11
T12 (input line)  — independent
T15 (dead code)   — independent
T17 (scroll/nav)  — independent
T18 (route tests) — independent
T19 (msg tests)   — depends on T9
```

## Serial execution order

Respects dependency DAG, interleaves independent batches:

```
T1 → T7 → T2 → T5 → T3 → T6 → T4 → T8
T1..T8 (done) → T9 → T15 → T18 → T10 → T11 → T16 → T12 → T13 → T14 → T17 → T19
```

Rationale for T9–T19: messaging (T9) and tests (T18) independent, can go first. Dead-code (T15) is pure deletion. Then snapshot bridging chain (T10 → T11 → T16). Then input+modal stack (T12 → T13 → T14). Then nav (T17). Then messaging tests (T19 depends on T9).

## Batch manifest

| Batch | Title | Group | Deps | ~LOC | Write scope |
|-------|-------|-------|------|------|-------------|
| T1 | StateHub subscription (replace polling with streaming) | streaming | — | ~350 | app.rs, state.rs, main.rs |
| T2 | Agent output segment parsing | display | — | ~500 | segment.rs (new), agents_view.rs, state.rs, mod.rs |
| T3 | Approval flow IPC | streaming | T1 | ~350 | approval_ipc.rs (new), app.rs, input.rs, orchestrate.rs, main.rs |
| T4 | Process supervision display | streaming | T1, T3 | ~300 | state.rs, app.rs, dashboard_view.rs, roko-runtime/process.rs |
| T5 | Parallel pool + wave ribbon | pool | T1 | ~350 | parallel_pool.rs, state.rs, dashboard_view.rs, input.rs |
| T6 | Context metrics + route display | display | T1, T2 | ~300 | state.rs, agents_view.rs, dashboard_view.rs |
| T7 | Dead field cleanup | cleanup | — | ~-200 | state.rs, app.rs, views/*, widgets/* |
| T8 | Visual effects (NervViz + particles) | effects | — | ~450 | postfx.rs, postfx_pipeline.rs, effects_config.rs, input.rs |
| T9  | Agent-server messaging: real LLM dispatch     | messaging | —      | ~250 | agent-server/features/messaging.rs, agent-server/state.rs |
| T10 | TUI snapshot bridging (gates/tokens/orch/partial progress) | snapshot | — | ~300 | state.rs, dashboard.rs |
| T11 | TUI plan nested tasks + failures population   | snapshot  | T10    | ~250 | state.rs, dashboard.rs |
| T12 | TUI inject/filter input line visibility       | input     | —      | ~200 | app.rs, input.rs, state.rs |
| T13 | TUI modal data + PlanDetail + key intercepts  | modals    | T10    | ~350 | input.rs, modals/*, state.rs, app.rs |
| T14 | TUI modal system consolidation                | modals    | T13    | ~250 | state.rs, input.rs, app.rs, modals/* |
| T15 | TUI dead widgets + dual theme/atmosphere merge| cleanup   | —      | ~-1800 | widgets/*, dashboard.rs |
| T16 | TUI duplicate fields + types consolidation    | cleanup   | T10,T11| ~-400 | state.rs, widgets/*, views/* |
| T17 | TUI scroll + PageUp/Down + ScrollAccel + tab-aware nav | nav | — | ~300 | input.rs, state.rs, scroll.rs, app.rs |
| T18 | Route tests: deployments/templates/mcp-code   | tests     | —      | ~400 | roko-serve/src/routes/{deployments,templates}.rs tests; roko-mcp-code tests; learning.rs test refactor |
| T19 | Agent-server messaging integration tests      | tests     | T9     | ~150 | agent-server/src/features/messaging.rs tests |

## Conflict groups

- **streaming**: T1, T3, T4 (touch app.rs, state.rs, orchestrate.rs)
- **display**: T2, T6 (touch agents_view.rs, state.rs)
- **pool**: T5 (touches parallel_pool.rs, dashboard_view.rs)
- **cleanup**: T7, T15, T16 (touch state.rs / widgets broadly)
- **effects**: T8 (touches postfx files)
- **messaging**: T9, T19 (touch agent-server messaging)
- **snapshot**: T10, T11 (touch state.rs + dashboard.rs data flow)
- **modals**: T13, T14 (touch modals + ModalState)
- **input**: T12, T17 (touch input.rs)
- **tests**: T18 (touches roko-serve + roko-mcp-code tests)

## Verification gates

| Batch | Commands |
|-------|----------|
| T1 | `cargo check -p roko-cli && cargo test -p roko-cli --lib` |
| T2 | `cargo check -p roko-cli && cargo test -p roko-cli --lib -- tui::segment` |
| T3 | `cargo check -p roko-cli && cargo test -p roko-cli --lib -- tui::approval_ipc` |
| T4 | `cargo check -p roko-cli -p roko-runtime` |
| T5 | `cargo check -p roko-cli` |
| T6 | `cargo check -p roko-cli` |
| T7 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T8 | `cargo check -p roko-cli` |
| T9 | `cargo check -p roko-agent-server && cargo clippy -p roko-agent-server --no-deps -- -D warnings` |
| T10 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T11 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T12 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T13 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T14 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T15 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T16 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T17 | `cargo check -p roko-cli && cargo clippy -p roko-cli --no-deps -- -D warnings` |
| T18 | `cargo check -p roko-serve -p roko-mcp-code && cargo test -p roko-serve --lib --no-run && cargo test -p roko-mcp-code --lib --no-run && cargo clippy -p roko-serve -p roko-mcp-code --no-deps -- -D warnings` |
| T19 | `cargo check -p roko-agent-server && cargo test -p roko-agent-server --lib --no-run && cargo clippy -p roko-agent-server --no-deps -- -D warnings` |
