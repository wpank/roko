# Mock LLMs

> How LLM calls are intercepted and replayed from tape files for deterministic, hermetic testing.

**Status**: Shipping
**Crate**: `roko-test`
**Depends on**: [01-test-harness.md](01-test-harness.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

All LLM calls in tests are intercepted by a `TapeReplayer` that reads pre-recorded responses from `*.tape` files. A tape file is a JSON-Lines file of request/response pairs. No real LLM calls are made in any test.

---

## How Tape Replay Works

1. The test sets up an `IntegrationContext` or `E2EEnvironment` with a tape directory.
2. When test code triggers an LLM call (via `roko-agent`), the request is routed to the `TapeReplayer` instead of the real backend.
3. The `TapeReplayer` matches the incoming request to the next unplayed entry in the tape.
4. It returns the recorded response.
5. After the test, it verifies that all tape entries were consumed (no unused entries = test ran as expected).

---

## Tape File Format

A tape file is JSON-Lines. Each line is one request/response pair:

```jsonl
{"seq": 1, "request": {"model": "claude-3-5-sonnet-20241022", "messages": [...], "max_tokens": 4096}, "response": {"id": "msg_01...", "content": [{"type": "text", "text": "Here is my implementation..."}], "usage": {"input_tokens": 512, "output_tokens": 1024}}}
{"seq": 2, "request": {"model": "claude-3-5-sonnet-20241022", "messages": [...], "max_tokens": 4096}, "response": {"id": "msg_02...", "content": [...], "usage": {...}}}
```

Fields:
- `seq`: sequence number (1-indexed). Requests are matched in sequence order.
- `request`: the full request body as sent to the LLM provider.
- `response`: the full response body returned by the provider.

---

## Recording a Tape

To record a new tape (requires real LLM credentials):

```bash
ROKO_RECORD_TAPE=tests/fixtures/my_new_test.tape cargo test -p roko-agent -- my_new_test
```

The `TapeRecorder` middleware records every LLM request and response to the file. After the test:
1. Review the tape file.
2. Redact any sensitive content (API keys, private data).
3. Commit the file.

---

## Using a Tape in Tests

```rust
#[tokio::test]
async fn my_integration_test() {
    let ctx = IntegrationContext::builder()
        .with_tape("tests/fixtures/my_new_test.tape")
        .build()
        .await;

    // … test code that triggers LLM calls …
}
```

For tests that need multiple sequential tapes:
```rust
let ctx = IntegrationContext::builder()
    .with_tape_sequence(vec![
        "tests/fixtures/turn_1.tape",
        "tests/fixtures/turn_2.tape",
    ])
    .build()
    .await;
```

---

## Tape Matching

By default, the `TapeReplayer` matches requests by sequence order (tape entry 1 matches the first LLM call, entry 2 the second, etc.). Strict matching (request body comparison) is available:

```rust
let ctx = IntegrationContext::builder()
    .with_tape("tests/fixtures/test.tape")
    .with_strict_tape_matching(true) // fails if request body doesn't match
    .build()
    .await;
```

Strict matching is recommended for tests where the prompt content matters.

---

## Handling Missing Tape Entries

If a test makes more LLM calls than the tape has entries, the `TapeReplayer` panics with:
```
TapeExhausted: tape has 3 entries but test made 4 LLM calls.
```

If a test makes fewer LLM calls than the tape has entries, the test fails with:
```
UnusedTapeEntries: 1 of 3 tape entries were not consumed.
```

Both conditions indicate a test-tape mismatch that must be fixed.

---

## Invariants

- Tapes are immutable after commit. A test that changes its LLM call pattern must update its tape.
- Tapes do not contain real API keys or sensitive data.
- `ROKO_RECORD_TAPE` mode is never set in CI.

---

## Open Questions

- Should tape files be compressed (`.tape.gz`) for storage efficiency?

## See also

- [03-fixture-library.md](03-fixture-library.md) — companion fixtures for tape-backed tests
- [../tiers/02-integration-tests.md](../tiers/02-integration-tests.md) — integration test tape usage
