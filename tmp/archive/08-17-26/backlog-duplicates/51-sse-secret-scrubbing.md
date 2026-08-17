# SSE Secret Scrubbing

**Priority**: P2 — secrets can leak via real-time event streams
**Size**: S (½ day)
**Crate**: `crates/roko-serve/`

---

## Problem

The secret-scrubbing middleware in `crates/roko-serve/src/routes/middleware.rs`
explicitly skips `text/event-stream` responses. SSE streams bypass the `LogScrubber`
entirely — any secret values (API keys, tokens, credentials) present in streamed
events are sent to clients unredacted.

SSE consumers include the TUI dashboard, the demo frontend, and any external
monitoring tool. Secrets in events (e.g., a provider error message that includes the
API key in the URL, or a config snapshot that includes credentials) would be exposed
in real time.

The middleware skip is intentional — response-body scrubbing requires buffering the
full response, which is incompatible with streaming. The fix must scrub at the
producer site instead.

---

## Where to look

- `crates/roko-serve/src/routes/middleware.rs` — `is_scrubbable_content_type()` at
  line ~1484, which returns `false` for `text/event-stream`
- `crates/roko-serve/src/events.rs` — SSE event producers
- `crates/roko-serve/src/routes/status/` — status SSE routes
- `crates/roko-serve/src/routes/feeds.rs` — feed SSE routes
- `crates/roko-core/src/secrets/` — existing `LogScrubber` API

---

## What to do

**Step 1.** Identify all SSE event producers in `roko-serve`. These are functions that
construct `Event` or `Sse` responses.

**Step 2.** At each producer site, scrub the event payload before sending:

```rust
use roko_core::secrets::LogScrubber;

let scrubber = LogScrubber::from_config(&config);
let scrubbed_data = scrubber.scrub(&event_data);
let event = Event::default().data(scrubbed_data);
```

**Step 3.** Add a test that verifies a known secret pattern in an event payload is
redacted before reaching the SSE client.

---

## Acceptance criteria

- [ ] All SSE event producers scrub payloads using `LogScrubber` before sending
- [ ] A test proves that a secret in an event payload is redacted
- [ ] The middleware `text/event-stream` skip remains (it's correct for response-body
  scrubbing; the fix is at the producer, not the middleware)
- [ ] All existing tests pass (`cargo test -p roko-serve`)

---

**Origin**: productionizing audit (2026-08-13)
