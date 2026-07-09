# End-to-End Tests

> Full self-hosting loop tests: `roko prd → plan run → gate → persist → resume` in a hermetic environment.

**Status**: Shipping
**Crate**: `roko-e2e` (dedicated test crate)
**Depends on**: [02-integration-tests.md](02-integration-tests.md), [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

End-to-end tests exercise the entire `roko prd → plan run → gate → persist → resume` pipeline using a seed PRD, pre-recorded LLM tapes, and a hermetic filesystem. They run in ~5 minutes and are gated to CI on merges to `main`. They are the closest thing to running the self-hosting loop without actually touching a live LLM.

---

## What E2E Tests Verify

An E2E test asserts all of the following in a single run:

1. **PRD ingestion**: `roko prd` parses and persists a seed PRD to the substrate.
2. **Plan generation**: `roko plan run` produces a valid plan DAG from the PRD.
3. **Task dispatch**: the orchestrator dispatches tasks to agents and receives LLM responses (from tape).
4. **Gate pipeline**: agent outputs pass through the 11-gate, 7-rung pipeline.
5. **Substrate persistence**: gate-approved engrams are written to the JSONL substrate.
6. **Learning update**: the episode logger, cost tracker, and bandit algorithms update correctly.
7. **Resume**: after a simulated crash (mid-plan), `roko plan resume` continues from the last checkpoint.
8. **State integrity**: the final substrate state is content-addressed correctly; no orphaned engrams.

---

## Hermetic Environment

E2E tests run in a fully hermetic environment:

| Resource | How controlled |
|---|---|
| Filesystem | Per-test `TempDir`; destroyed after test |
| LLM calls | Replayed from `*.tape` files; no network |
| Clock | Synthetic, advancing only on explicit calls |
| RNG | Seeded deterministically per test |
| Subprocesses | `roko-runtime` uses in-process mode for E2E |
| Git worktrees | Created in the temp dir; no real repo touches |

---

## E2E Test Structure

```rust
#[tokio::test]
async fn full_self_hosting_loop_completes() {
    let env = E2EEnvironment::builder()
        .with_tape_dir("tests/e2e_fixtures/tapes/")
        .with_prd("tests/e2e_fixtures/seed_prd.md")
        .build()
        .await;

    // Phase 1: PRD ingestion
    env.cli().prd_create("Seed PRD").await.unwrap();

    // Phase 2: Plan and run
    let plan_id = env.cli().plan_run().await.unwrap();

    // Phase 3: Wait for completion (synthetic clock advances)
    env.run_to_completion(plan_id).await.unwrap();

    // Phase 4: Assertions
    let substrate = env.substrate();
    let engrams = substrate.list_all().await.unwrap();
    assert!(!engrams.is_empty(), "loop must have produced persisted engrams");

    for engram in &engrams {
        let expected_hash = ContentHash::from_bytes(&engram.serialize());
        assert_eq!(engram.id().content_hash(), expected_hash,
            "content-addressing must be correct for persisted engram");
    }

    let episodes = env.learn().episodes().await.unwrap();
    assert!(!episodes.is_empty(), "learning must have recorded episodes");
}

#[tokio::test]
async fn crash_mid_plan_resumes_correctly() {
    let env = E2EEnvironment::builder()
        .with_tape_dir("tests/e2e_fixtures/tapes/crash_resume/")
        .with_crash_at_task(3) // inject crash after task 3
        .build()
        .await;

    let plan_id = env.cli().plan_run().await.unwrap();
    env.run_to_crash(plan_id).await;

    // Resume from checkpoint
    let resumed_id = env.cli().plan_resume(plan_id).await.unwrap();
    env.run_to_completion(resumed_id).await.unwrap();

    // Verify no duplicate work
    let substrate = env.substrate();
    let engrams = substrate.list_all().await.unwrap();
    let task_ids: Vec<_> = engrams.iter().map(|e| e.metadata().task_id()).collect();
    assert_eq!(task_ids.len(), task_ids.iter().cloned().collect::<std::collections::HashSet<_>>().len(),
        "no task should be executed twice after resume");
}
```

---

## E2E Fixture Directory

```
tests/e2e_fixtures/
  seed_prd.md                    seed product requirements document
  tapes/
    full_loop/                   tape files for the full loop test
      turn_001.tape
      turn_002.tape
      …
    crash_resume/                tape files for the crash+resume test
      pre_crash/
      post_resume/
  expected_final_state.json      golden file for final substrate state
```

---

## Flakiness Policy

E2E tests that flake more than once in 30 days are moved to a `quarantine` list and run separately. Quarantined tests are not blocking for PR merge but are reviewed weekly. A test is un-quarantined only after:
- Root cause identified and fixed.
- 20 consecutive clean CI runs.

---

## Running E2E Tests Locally

```bash
# Run all E2E tests (slow, ~5 min)
cargo test -p roko-e2e

# Run a specific E2E test
cargo test -p roko-e2e -- full_self_hosting_loop_completes

# With verbose output
cargo test -p roko-e2e -- --nocapture
```

E2E tests are excluded from `cargo test` (workspace-level) via:
```toml
# roko-e2e/Cargo.toml
[package.metadata.cargo-test]
exclude-from-workspace-test = true
```

Run explicitly via `-p roko-e2e` or in CI.

---

## Invariants

- E2E tests never make real LLM calls; all responses come from tape files.
- Every E2E test is fully hermetic and independently reproducible.
- A crash+resume E2E test must verify that no task is executed more than once.

---

## Open Questions

- Should the E2E suite include a multi-agent parallel execution test (two agents competing for the same worktree)?
- Should the seed PRD for E2E tests be the actual Roko self-improvement PRD, or a synthetic minimal one?

## See also

- [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md) — tape format
- [../by-subsystem/subsystem-orchestrator.md](../by-subsystem/subsystem-orchestrator.md) — orchestrator E2E coverage
- [../quality-gates/03-pre-release.md](../quality-gates/03-pre-release.md) — E2E as a release gate
