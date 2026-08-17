# S-acp-2: ACP end-to-end transcript proof test

## Task
Add `crates/roko-acp/tests/transcript_e2e.rs` that drives a real `AcpSession` against a stubbed `ModelCallService` stream and asserts the JSON-RPC frames sent on the wire match the ACP spec for happy path + typed `Failed` event.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-acp-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/21-acp-protocol-completion.md` § ACP-2.

## Why
Existing ACP tests cover individual events. Nothing asserts the complete transcript shape against a deterministic stream. A regression in `bridge_events.rs` could land without being caught.

## Read first

```bash
ls crates/roko-acp/tests/ crates/roko-acp/src/
rg 'pub fn test_with_stream|fn collect_outbound_frames|fn AcpSession' crates/roko-acp/src/ -n
```

If a `test_with_stream` helper doesn't exist, you may need to add one. If `AcpSession` is hard to construct in tests, make a `#[cfg(test)]` constructor.

## Exact changes

### 1. Test helper

In `crates/roko-acp/src/session.rs` (or `lib.rs`), gated by `#[cfg(test)]` or `#[cfg(any(test, feature = "test-helpers"))]`:

```rust
#[cfg(test)]
pub mod test_support {
    use super::*;
    use roko_agent::ModelStreamEvent;

    pub struct StubStreamModelCallService {
        pub events: std::sync::Mutex<Vec<ModelStreamEvent>>,
    }

    impl StubStreamModelCallService {
        pub fn new(events: Vec<ModelStreamEvent>) -> Arc<Self> {
            Arc::new(Self { events: std::sync::Mutex::new(events) })
        }
    }

    #[async_trait]
    impl ModelCallService for StubStreamModelCallService {
        async fn call(&self, _req: ModelCallRequest) -> Result<ModelCallResponse, ModelCallError> {
            unimplemented!("stub uses streaming only")
        }
        async fn stream(&self, _req: ModelCallRequest) -> Result<BoxedStream, ModelCallError> {
            let events = self.events.lock().unwrap().drain(..).collect::<Vec<_>>();
            Ok(Box::pin(futures::stream::iter(events.into_iter().map(Ok))))
        }
    }

    impl AcpSession {
        pub fn test_with_stream(events: Vec<ModelStreamEvent>) -> Self {
            let stub = StubStreamModelCallService::new(events);
            // Construct an AcpSession wired to the stub. Reuse whatever
            // session-building helpers already exist; only the dispatch
            // service is replaced.
            Self::test_default_with_dispatch(stub)
        }

        /// Drive the session with a prompt; return the JSON-RPC frames it sent.
        pub async fn collect_outbound_frames(&mut self, prompt: &str) -> Vec<JsonRpcFrame> {
            // Use an in-process pipe for the JSON-RPC transport. Write
            // a `session/prompt` request, drive the session loop, read
            // back all outbound frames until the response arrives.
        }
    }
}
```

If `AcpSession::test_default_with_dispatch` doesn't exist, build it minimally — enough fields to dispatch a prompt.

### 2. Test cases

`crates/roko-acp/tests/transcript_e2e.rs`:

```rust
use roko_acp::session::AcpSession;
use roko_agent::ModelStreamEvent;

#[tokio::test]
async fn happy_path_transcript_matches_spec() {
    let stream = vec![
        ModelStreamEvent::TextDelta("Hello ".into()),
        ModelStreamEvent::TextDelta("world.".into()),
        ModelStreamEvent::Completed { text: "Hello world.".into(), usage: None },
    ];
    let mut session = AcpSession::test_with_stream(stream);
    let frames = session.collect_outbound_frames("hi").await;

    let chunks: Vec<_> = frames.iter()
        .filter(|f| f.method.as_deref() == Some("session/update"))
        .filter(|f| f.params.get("update")
            .and_then(|u| u.get("kind"))
            .and_then(|k| k.as_str()) == Some("agent_message_chunk"))
        .collect();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].params["update"]["text"], "Hello ");
    assert_eq!(chunks[1].params["update"]["text"], "world.");

    let prompt_resp = frames.iter().rfind(|f| {
        f.id.is_some() && f.method.is_none() && f.result.is_some()
    }).unwrap();
    assert_eq!(prompt_resp.result.as_ref().unwrap()["text"], "Hello world.");
}

#[tokio::test]
async fn failed_stream_surfaces_typed_failure() {
    let stream = vec![
        ModelStreamEvent::TextDelta("Working...".into()),
        ModelStreamEvent::Failed { reason: "rate_limit".into(), code: Some(429) },
    ];
    let mut session = AcpSession::test_with_stream(stream);
    let frames = session.collect_outbound_frames("hi").await;

    let failed_update = frames.iter().find(|f| {
        f.method.as_deref() == Some("session/update")
            && f.params.get("update")
                .and_then(|u| u.get("status"))
                .and_then(|s| s.as_str()) == Some("failed")
    });
    assert!(failed_update.is_some(), "expected a session/update with status=failed; got {frames:#?}");
}

#[tokio::test]
async fn cancellation_emits_session_update_cancelled() {
    let stream = vec![
        ModelStreamEvent::TextDelta("Starting".into()),
        // Test cancels mid-stream
    ];
    let mut session = AcpSession::test_with_stream(stream);
    let frames = session.collect_outbound_frames_with_cancel("hi").await;
    // Assert a cancellation update was emitted
}
```

If `JsonRpcFrame` isn't a public type, expose enough of the test transport to inspect the wire format (or use raw JSON parsing of the test pipe).

## Write Scope
- `crates/roko-acp/tests/transcript_e2e.rs` (new)
- `crates/roko-acp/src/session.rs` (test helpers, `#[cfg(test)]` only)
- `crates/roko-acp/src/lib.rs` (re-export test_support if needed)

## Read-Only Context
- `crates/roko-acp/src/bridge_events.rs`
- `crates/roko-agent/src/model_call_service.rs`

## Verify

```bash
ls crates/roko-acp/tests/transcript_e2e.rs

rg 'happy_path_transcript_matches_spec|failed_stream_surfaces_typed_failure' crates/roko-acp/tests/transcript_e2e.rs
# Expect: 2+ hits
```

## Acceptance Criteria

- `transcript_e2e.rs` exists with 2-3 test cases.
- Stub stream service implemented.
- Tests assert specific JSON-RPC frame shapes (kinds, fields).
- No raw HTTP / network in tests.

## Do NOT

- Do NOT make tests dependent on environment vars (no `ANTHROPIC_API_KEY` in tests).
- Do NOT skip the `Failed` event test — that's the whole point.
- Do NOT bundle with S-acp-1/3/4.
- Do NOT add `#[ignore]` to tests; they should run in CI.
- Do NOT touch the production code path beyond adding `#[cfg(test)]` helpers.
