# F2: Add ISFR WebSocket/SSE Stream to roko-serve

## Context

The demo-app needs real-time rate updates pushed to the frontend. roko-serve already has SSE (`/api/events`) and WebSocket (`/ws`) endpoints that broadcast `ServerEvent`. After F1 adds `IsfrRateComputed` to the event enum, SSE/WS clients automatically receive these events.

This task ensures the frontend can also connect directly to the relay's WebSocket for even lower latency (subscribing to `isfr:rates` topic).

## No New Files Needed (SSE/WS path)

The existing SSE and WebSocket infrastructure already broadcasts all `ServerEvent` variants. Once F1 adds `IsfrRateComputed`, clients subscribed to `/api/events` or `/ws` will automatically receive ISFR updates.

## Optional: Add Topic-Filtered SSE Endpoint

For clients that only want ISFR events (not all server events), add a filtered endpoint.

### File to Modify

- `crates/roko-serve/src/routes/isfr.rs` — add SSE endpoint

### Implementation

Add to the existing `routes()` in `isfr.rs`:

```rust
.route("/isfr/stream", get(isfr_stream))
```

Handler:

```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;

/// SSE stream of ISFR rate events only.
async fn isfr_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.event_bus.subscribe();

    // NOTE: event_bus.subscribe() returns a broadcast::Receiver<ServerEvent>.
    // broadcast::Receiver::recv() returns Result<T, RecvError>.
    // RecvError variants: Lagged(u64), Closed.
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Only forward ISFR-related events.
                    let is_isfr = matches!(&event,
                        ServerEvent::IsfrRateComputed { .. }
                        | ServerEvent::IsfrSourceHealthChanged { .. }
                        | ServerEvent::IsfrKeeperStateChanged { .. }
                    );
                    if is_isfr {
                        let data = serde_json::to_string(&event).unwrap_or_default();
                        return Some((Ok(Event::default().data(data)), rx));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keepalive"),
    )
}
```

## Frontend Usage

The demo-app will consume this in two ways:

1. **SSE (simple, roko-serve path)**: `EventSource` to `http://localhost:6677/api/isfr/stream`
2. **WebSocket (relay path)**: Direct WebSocket to `ws://localhost:9011/relay/agents/ws` with Subscribe frame for `isfr:rates`

The frontend task (F4) implements both and uses relay WS when available, falling back to SSE.

## Verification

```bash
cargo build -p roko-serve
# Start serve + keeper:
# curl -N http://localhost:6677/api/isfr/stream
# Should see SSE events as keeper publishes rates
```

## Dependencies

- F1 (ISFRState + IsfrRateComputed event variant + routes)
