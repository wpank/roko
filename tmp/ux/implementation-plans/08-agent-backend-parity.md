# 08 — Agent Backend Parity (Codex / Cursor / streaming / cascade)

> **Source plan**: `tmp/ux/ux-followup/06-advanced-agent-backends.md` items
> 36, 37, 38, 39, 40, 40a. Cross-cuts hygiene item 60c (cascade router
> e2e).
>
> **Status as of 2026-05-01**: `roko-agent` supports 8+ backends with
> uneven test coverage. Claude (CLI + API) is well-tested. Codex
> conformance to its newest protocol is uncertain. Cursor's streaming
> path has 6 occurrences of "streaming" but no documented parity test.
> The cascade router has unit tests against mock dispatchers but no
> end-to-end test that drives a routing decision through a real
> backend's trait impl.
>
> **Effort**: 8-12 days.
>
> **Risk**: Medium. The cascade router is the single most user-impactful
> change vector — wrong routing decisions cost real money. Tests must
> not regress production behaviour.

---

## What this plan accomplishes

Bring Codex, Cursor, ExecAgent, Gemini/Perplexity/Ollama, and the
cascade router up to test-parity with the Claude reference
implementation. After this plan:

- Codex backend has a 10-turn conformance test against a captured
  session log.
- Cursor backend has documented streaming support and a streaming
  parity test.
- Each non-Claude backend has the five canonical scenarios:
  *happy path, streaming, tool-call, error, session-continuation.*
- ExecAgent and ClaudeCliAgent are reconciled (one stays; the other
  is documented as a thin wrapper if both are needed).
- Gemini / Perplexity / Ollama have a single canonical entry-point file
  per backend (no `<name>_agent.rs` + `<name>_backend.rs` overlap).
- The cascade router has an integration test that boots `Dispatcher`
  with two mock backends, runs cascade until one wins, and asserts
  state at `.roko/learn/cascade-router.json`.

## Why this matters

The dispatcher and cascade router decide which provider gets each
request. Bugs there don't surface as test failures — they surface as
wrong-model selection or runaway cost. The Claude tests give us
confidence for Claude; everything else is unverified.

---

## Required reading

```
crates/roko-agent/src/                            (entire directory)
crates/roko-agent/src/tool_loop/mod.rs            (1686 LOC)
crates/roko-agent/src/dispatcher/mod.rs           (1069 LOC)
crates/roko-agent/src/multi_pool.rs               (1007 LOC)
crates/roko-agent/src/codex_agent.rs              (966 LOC)
crates/roko-agent/src/claude_agent.rs             (966 LOC; reference)
crates/roko-agent/src/claude_cli_agent.rs         (878 LOC; reference)
crates/roko-agent/src/cursor_agent.rs             (763 LOC)
crates/roko-agent/src/exec.rs                     (~600 LOC)
crates/roko-agent/src/gemini/                     (multi-file)
crates/roko-agent/src/perplexity/                 (multi-file)
crates/roko-agent/src/ollama_agent.rs + ollama_backend.rs
crates/roko-agent/src/openai_compat_backend.rs    (973 LOC)
crates/roko-learn/src/cascade_router.rs           (4766 LOC)
crates/roko-learn/src/model_router.rs             (2170 LOC)
crates/roko-std/src/mock.rs                       (mock dispatcher pattern)
.roko/learn/cascade-router.json (sample after running cascade)
```

Existing tests:

```
crates/roko-agent/src/{claude_agent,claude_cli_agent}.rs   # mod tests
crates/roko-learn/src/cascade/tests.rs
crates/roko-agent/tests/                                   # integration
```

---

## Deliverables

### Per-backend canonical test suite (5 backends × 5 scenarios)

For each of `codex_agent`, `cursor_agent`, `gemini::native`, `perplexity::chat`,
`ollama_agent`, `openai_compat_backend`, `exec`, replicate the Claude
test layout:

```rust
// crates/roko-agent/tests/<backend>_parity.rs

#[tokio::test]
async fn happy_path() { /* assert send_turn returns expected response */ }

#[tokio::test]
async fn streaming() { /* assert send_turn_streaming yields chunks in order */ }

#[tokio::test]
async fn tool_call() { /* assert tool-call frames are routed back to the dispatcher */ }

#[tokio::test]
async fn error_path() { /* assert error frames produce DispatchError, not panic */ }

#[tokio::test]
async fn session_continuation() { /* assert second turn carries prior context */ }
```

Tests use `mockito` or `httpmock` (already a dev-dep) to fake the
backend HTTP. For Codex specifically, use a *captured* session log.

### Codex conformance harness (item 36)

1. Capture a real Codex session at the latest protocol revision (a
   10-turn fixture that exercises multi-reasoning, draft-frame
   semantics, tool calls, refusals).
2. Save under
   `crates/roko-agent/tests/fixtures/codex_session_v<N>.json`.
3. Build a replay harness in `crates/roko-agent/tests/codex_conformance.rs`
   that feeds each fixture frame and asserts our parsing matches the
   captured ground truth.

### Cursor streaming verification (item 37)

1. Confirm `cursor_agent::send_turn_streaming` exists and is plumbed
   through `LlmBackend`.
2. If absent, port the Claude streaming pattern. Use `tokio::sync::mpsc::Receiver<StreamChunk>`
   on the public API.
3. Add a parity test that compares the concatenated stream output to a
   non-streaming fetch of the same prompt.

### ExecAgent vs ClaudeCliAgent decision (item 39)

These overlap in purpose (both wrap a CLI). Two paths:

**Option A (consolidate)**: ExecAgent absorbs the generic CLI path;
ClaudeCliAgent becomes a thin layer that configures ExecAgent with
Claude-specific defaults. Net LOC drop ~400.

**Option B (specialise)**: ClaudeCliAgent stays Claude-only; ExecAgent
documents itself as "for any non-CLI provider that exposes a stdin/stdout
JSON protocol". Net LOC drop 0; clearer mental model.

Recommendation: **Option A**. ClaudeCliAgent is a 5-line config of the
ExecAgent pattern. Avoid two parallel implementations.

### Gemini / Perplexity / Ollama unification (item 40)

For each backend, choose:

- **Single file** (`<backend>.rs`): if total LOC < 2 000.
- **Directory** (`<backend>/{mod.rs, native.rs, ...}`): if multi-protocol
  (e.g. native API + OpenAI-compat shim coexisting).

Concretely:

| Backend | Today | Recommend |
|---------|-------|-----------|
| Gemini | `gemini/native.rs` 883 LOC | single file `gemini.rs` |
| Perplexity | `perplexity/{chat,deep_research,tool_loop,search}.rs` 2562 LOC total | keep directory; clean the `mod.rs` to be the public surface |
| Ollama | `ollama_agent.rs` + `ollama_backend.rs` | single file `ollama.rs` |

Don't do both file + directory. Pick one or the other per backend.

### Cascade router integration test (item 40a / 60c)

`crates/roko-learn/tests/cascade_router_integration.rs`:

```rust
#[tokio::test]
async fn cascade_router_persists_winner_and_route_through_real_trait() {
    let tempdir = tempfile::tempdir().unwrap();
    let mock_a = roko_std::mock::Backend::new("model-a")
        .with_response("a-result");
    let mock_b = roko_std::mock::Backend::new("model-b")
        .with_response("b-result");
    let dispatcher = Dispatcher::new(vec![mock_a, mock_b]);
    let router = CascadeRouter::new(tempdir.path().join("cascade-router.json"));

    // Drive 10 routing decisions.
    for i in 0..10 {
        router.route(&dispatcher, format!("prompt-{i}")).await.unwrap();
    }

    // Assertion: winner persisted to JSON.
    let raw = std::fs::read_to_string(tempdir.path().join("cascade-router.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(parsed["winner"].as_str().is_some(), "no winner persisted");
}
```

Run under `cargo test -p roko-learn --test cascade_router_integration`.
Add to CI's default test matrix.

---

## Step-by-step

### Step 1 — Capture Codex fixture (1 day)

Run an actual Codex session through `roko chat` against the newest
protocol. Save the raw JSON-RPC frames (use the existing dispatcher
trace logs). Trim PII; commit under `tests/fixtures/`. Document the
protocol version in `tests/fixtures/codex-version.txt` so the next
regen knows what to compare against.

Anti-pattern: don't fake the fixture. The whole point is to lock in
real-world behaviour.

### Step 2 — Build the Claude reference golden (half day)

Write a test scaffold in `crates/roko-agent/tests/_helpers/parity_kit.rs`
that exposes:

```rust
pub async fn assert_happy_path<B: LlmBackend>(backend: B, expected: &str);
pub async fn assert_streaming<B: LlmBackend>(backend: B, expected: &str);
pub async fn assert_tool_call<B: LlmBackend>(backend: B, tool_name: &str);
pub async fn assert_error_path<B: LlmBackend>(backend: B, status: u16);
pub async fn assert_session_continuation<B: LlmBackend>(backend: B);
```

Existing Claude tests rewrite around this kit so behaviour is provably
identical. New backend tests reuse the kit verbatim.

### Step 3 — Per-backend tests (1 day each, parallelisable)

Apply the kit to every non-Claude backend. Where a scenario doesn't
apply (e.g. a backend without tool-call support), add an
`#[ignore = "no tool calls"]` marker with a comment linking to the
backend's docs.

### Step 4 — ExecAgent / ClaudeCliAgent reconciliation (1 day)

Choose Option A (consolidation). Steps:

1. Move ExecAgent's generic logic into a `cli_agent.rs` module.
2. Reduce `claude_cli_agent.rs` to a `ClaudeCliAgent::new()` that
   configures `cli_agent::CliAgent` with Claude defaults.
3. All callers of `ClaudeCliAgent` keep working (struct moves, not API
   change).
4. `cargo test -p roko-agent` green.

### Step 5 — Gemini / Ollama unification (half day)

Move multi-file backends to single files where applicable. Update
`mod.rs` and any external imports.

### Step 6 — Cascade router integration test (1 day)

Per the deliverable above. Add to CI.

### Step 7 — Documentation (half day)

`docs/v2/AGENT-BACKENDS.md`:

- A table mapping backend → file → test coverage.
- A "How to add a new backend" walkthrough using the parity kit.
- A "How the cascade router learns" section pointing at
  `cascade-router.json`.

CLAUDE.md "Status table" rows for each backend reflect new test
coverage.

---

## Anti-patterns to avoid

- **Don't write parity tests with `tokio::time::sleep`-based
  synchronisation.** Use `mockito` deterministic responses or
  `tokio::time::pause`. We have prior incidents (item 58 in
  `09-hygiene-and-test-coverage.md`).
- **Don't unify `_agent.rs` and `_backend.rs` files across the board.**
  Some backends (Ollama notably) have a thin agent-side wrapper plus a
  thicker backend. Single file is fine for *some*; not all. Decide per
  backend.
- **Don't run real network calls in parity tests.** Test against
  `mockito` fake servers. Reserve a separate `--ignored` smoke layer
  for actual provider exercise (only run on a manual `cargo test --
  --ignored backends_smoke`).
- **Don't change the wire format of `cascade-router.json` casually.**
  The TUI dashboard reads this file. If you bump the schema, also
  ship a migration shim (see `crates/roko-cli/src/snapshot_migrate.rs`).
- **Don't mock so deeply that the test passes on bug-X but production
  hits bug-X-prime.** Mocks must reflect real backend behaviour: HTTP
  status codes, header expectations, JSON shape, partial-chunk
  framing. Capture real fixtures (Step 1).
- **Don't fold the cascade integration test into a `#[ignore]`-by-default
  layer.** It runs in <5 s and is a key safety net.

## Done when

1. `crates/roko-agent/tests/<backend>_parity.rs` exists and passes for
   each non-Claude backend.
2. `crates/roko-agent/tests/codex_conformance.rs` exists and replays
   the captured fixture green.
3. `crates/roko-agent/tests/_helpers/parity_kit.rs` exists; all parity
   tests use it.
4. ExecAgent / ClaudeCliAgent reconciliation merged; net LOC drop
   visible in `tokei`.
5. Backend file layout consistent (single file or directory, never both).
6. `cargo test -p roko-learn --test cascade_router_integration` passes.
7. `docs/v2/AGENT-BACKENDS.md` exists with the coverage table.
8. `tmp/ux/ux-followup/06-advanced-agent-backends.md` items 36-40, 40a
   marked DONE.
9. `tmp/ux/ux-followup/09-hygiene-and-test-coverage.md` item 60c marked
   DONE.
