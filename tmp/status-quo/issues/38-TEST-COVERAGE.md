# Test Coverage Gaps

## Critical

### `run()` main event loop — zero direct integration tests
- `runner/event_loop.rs:436`: Core `tokio::select!` dispatch (lines 436-2300) never directly called from any test.
- `handle_agent_failure` (line 2391): Retry/abort branching untested in isolation.
- `handle_merge_completion` (line 2798): Executor bridge untested.

### Self-hosting E2E permanently `#[ignore]`d
- `tests/e2e_self_host.rs:15`: The ONLY test that exercises `prd idea → draft → promote → plan run` end-to-end. Disabled with "needs ROKO_DISPATCHER fixture" — but fixture mechanism already exists.

## High

### `agent_events.rs` has zero tests
- No `#[cfg(test)]` block. Buffer-trim branch (`ceil_char_boundary` at line 57-68) untested. Can silently corrupt multibyte characters.

### `spawn_gate` semaphore exhaustion path untested
- `gate_dispatch.rs:28`: Production path wrapping `run_gate_once`. Semaphore poisoning → gate silently dropped → task hangs. Never exercised.

### Graph Engine tests validate stub behavior
- `tests/plan_conversion.rs`: `assert!(output.success)` — always succeeds because nothing runs. Tests prove stubs work, not that execution works.

## Medium

### Ollama E2E tests silently pass when Ollama absent
- `tests/ollama_e2e.rs`: `if !ollama_gate() { return; }` — reports as PASSED. CI sees 100% pass when tests never ran. Should use `#[ignore]`.

### ExecAgent parity — 3/3 tests permanently ignored
- `roko-agent/tests/exec_parity.rs:17,23,29`: All `#[ignore]`. Aspirational or dead.

### `common/run_sample_plan` fragile JSON scanning
- `tests/common/mod.rs:296`: Scans stdout for first `{` character. Log JSON objects would be misdetected.

### Real dispatch tests feature-gated
- `phase0_wiring.rs`, `cost_dedup.rs`, `smoke.rs`: Gated on `legacy-runner-v2` (which is default). Tests verify CascadeRouter via side-effect, not the runner loop directly.
