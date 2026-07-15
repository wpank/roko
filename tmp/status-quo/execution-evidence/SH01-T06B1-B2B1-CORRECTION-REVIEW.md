# SH01-T06B1-B2B1-CORRECTION independent review

- Verdict: **ACCEPTED**
- Reviewed at: `2026-07-14T08:33:51Z`
- Reviewer branch: `review/SH01-PROCESS-fa828276a535`
- Required base: `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Candidate: `fa828276a53597abc0fa82f249b7f14ac96f5a0d`
- Candidate subject: `fix(runner): SH01-T06B1-B2B1-CORRECTION retain process ownership`

## Scope and context

I read the complete canonical master checklist, the complete SH01 manifest, issues 28 and 47, the worker evidence, the candidate diff, and the exact relevant diff from historical commit `3041d095d`. I inspected all 162 added/changed lines relative to the required base: 116 insertions and 4 deletions in `crates/roko-cli/src/runner/agent_stream.rs`, plus the 46-line worker evidence record. No other candidate paths changed.

I also traced the unchanged production call path from `dispatch::spawn_streaming_cli_agent` through runner dispatch and the settlement/cancellation paths in `event_loop.rs`, and inspected the process registry implementation in `roko-agent`.

## Findings

1. PID ownership now begins immediately after `Child::id()` succeeds. Registration precedes the awaited `Started` delivery, stdin handling, captured-stdout validation, and reader-task creation. Thus every reachable post-spawn suspension or failure has durable PID ownership; confirmed wait/kill remains responsible for unregistering it.
2. The stdout closed-channel branch now returns from the reader task. It no longer merely breaks the inner parsed-event loop and then blocks on the live child's next stdout line. Returning drops the `BufReader` and captured stdout resource immediately.
3. The candidate does not contain `AgentHandle::is_finished`, another `is_finished` API, or any C4 polling/scope expansion. The historical precursor's `is_finished` addition is absent.
4. Existing lifecycle ownership remains coherent. `wait()` returns the complete handle on an unconfirmed process wait and unregisters only after a successful wait. `kill()` returns retryable handle ownership when process absence is unconfirmed; after confirmed absence it unregisters and deliberately stops/joins both readers. Runner settlement and cancellation retain or transition that ownership accordingly.
5. The two new tests directly cover the two corrected regressions and clean up the spawned process/registry state on both success and failure paths. Existing wait, kill, reader-panic, reader-cancellation, and retained-registration tests remain green.

## Independent parent/candidate reproduction

I temporarily reverted only the two production corrections in the isolated review checkout while retaining the candidate tests, built the library test target, and reproduced both parent failures:

- `spawned_pid_is_registered_before_started_delivery_completes`: **FAILED** after the expected five-second observation timeout with `left: None`, `right: Some(<pid>)`.
- `closed_event_channel_terminates_stdout_reader_before_child_exit`: **FAILED** after the expected five-second timeout with `stdout reader remained attached to the live child after its event channel closed`.

I then restored the candidate and proved the source was byte-for-byte identical to `HEAD` before continuing:

```text
8e6f3443081de82f63415443016b7213f5e6efb48f5f744b05bb3ab81c5b1437  working tree
8e6f3443081de82f63415443016b7213f5e6efb48f5f744b05bb3ab81c5b1437  HEAD blob
```

After rebuilding the candidate:

- exact early-registration regression: **PASS** (`1 passed`)
- exact closed-channel regression: **PASS** (`1 passed`)
- complete `runner::agent_stream::tests::` module: **PASS** (`17 passed, 0 failed`)

The intentional panic diagnostics printed by the reader-panic tests are expected assertions of structured error handling; those tests passed.

## Verification gates

```text
cargo test -p roko-cli --lib runner::agent_stream::tests::spawned_pid_is_registered_before_started_delivery_completes -- --exact --nocapture
PASS: 1 passed, 0 failed

compiled roko_cli lib-test binary runner::agent_stream::tests::closed_event_channel_terminates_stdout_reader_before_child_exit --exact --nocapture
PASS: 1 passed, 0 failed

compiled roko_cli lib-test binary runner::agent_stream::tests:: --nocapture
PASS: 17 passed, 0 failed

CARGO_INCREMENTAL=0 cargo check -p roko-cli --lib
PASS

CARGO_INCREMENTAL=0 cargo clippy -p roko-cli --lib -- -D warnings
PASS

rustfmt --edition 2024 --check crates/roko-cli/src/runner/agent_stream.rs
PASS

git diff --check
PASS

git diff --exit-code -- crates/roko-cli/src/runner/agent_stream.rs
PASS after parent reproduction/restoration
```

An initial unscoped `cargo test -p roko-cli` attempt compiled the required library test binary but failed later while linking the unrelated `roko` bin with `ld: write() failed, errno=28` because the shared target exhausted disk space. This was an environmental bin-link failure, not a source/test failure. Space was subsequently recovered, and the scoped library build, both regressions, full module, check, and clippy all completed successfully.

## Verdict

**ACCEPTED.** Candidate `fa828276a53597abc0fa82f249b7f14ac96f5a0d` corrects both parent regressions within the assigned scope, preserves existing wait/cancel supervision semantics, and excludes the rejected `AgentHandle::is_finished` expansion.
