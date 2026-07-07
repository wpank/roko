# Integration Tests

> Cross-crate tests that verify multi-component contracts under realistic (but hermetic) conditions.

**Status**: Shipping
**Crate**: `roko-test` harness + per-subsystem `tests/` directories
**Depends on**: [01-unit-tests.md](01-unit-tests.md), [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Integration tests assemble two or more crates in the same process, use real filesystem I/O inside per-test temp directories, and replay LLM calls from fixture tapes. They verify that subsystem contracts compose correctly — a passing unit suite plus a passing integration suite is the minimum bar for merging.

---

## Scope

An integration test may:
- Construct and wire together multiple crates (e.g., orchestrator + agent + gate + fs).
- Write to and read from a real filesystem in a per-test temp directory.
- Replay pre-recorded LLM responses from fixture tapes.
- Advance a synthetic clock across multiple turns.
- Assert on cross-crate state transitions (e.g., gate verdict → substrate write → learning update).

An integration test must not:
- Make real LLM calls (use tape replay).
- Make real network calls.
- Share state between tests.
- Exceed 60 seconds per individual test.

---

## TestContext for Integration Tests

Integration tests use an extended `IntegrationContext`:

```rust
#[tokio::test]
async fn orchestrator_dispatches_task_through_gate_to_substrate() {
    let ctx = IntegrationContext::builder()
        .with_tape("fixtures/agent_success.tape")   // LLM replay
        .with_gate_config(GateConfig::permissive())  // pass-all config for non-gate tests
        .build()
        .await;

    let plan = ctx.load_plan("fixtures/simple_plan.json");
    ctx.orchestrator().run_plan(&plan).await.unwrap();

    // Assert cross-crate outcomes
    let engrams = ctx.substrate().list_all().await.unwrap();
    assert!(!engrams.is_empty(), "orchestrator should have persisted output");

    let episode = ctx.learn().latest_episode().await.unwrap();
    assert!(episode.outcome.is_success());
}
```

---

## Integration Test Locations

Integration tests live in `tests/` directories at the crate that "owns" the interaction:

| Test file | Interaction tested |
|---|---|
| `roko-orchestrator/tests/orchestration_integration.rs` | orchestrator ↔ agent ↔ gate ↔ fs |
| `roko-gate/tests/gate_pipeline_integration.rs` | all 11 gates in sequence |
| `roko-agent/tests/backend_integration.rs` | LLM backend switching via CascadeRouter |
| `roko-learn/tests/feedback_loop_integration.rs` | orchestrator turn → learning update |
| `roko-cli/tests/cli_integration.rs` | CLI commands → subsystem state |

---

## Fixture Tapes

LLM responses are recorded as `*.tape` files under `tests/fixtures/`. A tape is a JSON-Lines file where each line is a recorded request/response pair:

```jsonl
{"request": {"model": "claude-3-5-sonnet", "messages": [...]}, "response": {"content": "...", "usage": {...}}}
```

To add a new integration test that needs LLM responses:

1. Run with `ROKO_RECORD_TAPE=tests/fixtures/my_test.tape` to record live responses.
2. Commit the tape file.
3. Use `ctx.with_tape("tests/fixtures/my_test.tape")` in the test.

See [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md) for the full tape API.

---

## Failure Semantics

Integration test failures report:
- Which crate boundary was crossed when the failure occurred.
- The last recorded event in the synthetic clock sequence.
- The substrate state at time of failure (serialized to the test output).

This is the "failure is a verdict" principle applied to integration tests: a failure must be diagnosable without a debugger.

---

## Running Integration Tests

```bash
# All integration tests
cargo test --test '*'

# Specific crate integration tests
cargo test -p roko-orchestrator --test orchestration_integration

# With tape recording (for adding new fixtures)
ROKO_RECORD_TAPE=tests/fixtures/new.tape cargo test -p roko-agent --test backend_integration
```

---

## Invariants

- Integration tests are hermetic: they never touch external networks.
- Each test gets a fresh temp directory; no cleanup needed (dropped on test exit).
- Tape files are deterministic: replaying a tape always produces the same sequence.
- A new integration test that requires a new tape file must commit the tape.

---

## Open Questions

- Should integration test fixture tapes be stored in Git LFS given their potential size?
- Should `IntegrationContext` provide a built-in timeout assertion to catch deadlocks?

## See also

- [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md) — tape format and recording
- [../tools-and-harness/03-fixture-library.md](../tools-and-harness/03-fixture-library.md) — shared fixtures
- [../by-subsystem/](../by-subsystem/README.md) — per-subsystem integration test coverage
