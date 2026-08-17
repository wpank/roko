# PAA_02: Wire SSE /api/workflows/latest/stream and WS /api/workflow/ws

## Task
Implement the workflow projection endpoints that the PrdPipelinePanel demo scenario requires.

## Runner Context
Runner PAA, batch 2 of 3. No dependencies.

## Problem
`demo/demo-app/src/lib/workflow-api.ts:154,174`:
```ts
const sse = new EventSource(`${SERVE_URL}/api/workflows/latest/stream?${workflowQuery(root)}`);
const ws = new WebSocket(`${WS_BASE}/api/workflow/ws`);
```

Without these, the `prd-pipeline` demo scenario renders nothing — it stays at `EmptyState`.

SSE event types: `state`, `delta`.
WS subscribe payload: `{ type: "subscribe", root, projections: ["workflow.artifacts", "workflow.execution", "workflow.gates", "workflow.agents"] }`

## Exact Changes

### Step 1: Check if workflows.rs exists

Search `crates/roko-serve/src/routes/` for `workflows.rs`.

### Step 2: Implement SSE stream

```rust
async fn workflow_stream(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WorkflowStreamParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.workflow_event_tx.subscribe();
    let root = params.root.clone();

    // Send initial state snapshot
    let initial = build_workflow_snapshot(&state, root.as_deref()).await;

    let stream = async_stream::stream! {
        yield Ok(Event::default().event("state").data(serde_json::to_string(&initial).unwrap()));
        while let Ok(event) = rx.recv().await {
            yield Ok(Event::default().event("delta").data(serde_json::to_string(&event).unwrap()));
        }
    };
    Sse::new(stream)
}
```

### Step 3: Implement WS endpoint

```rust
async fn workflow_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_workflow_ws(socket, state))
}
```

Handle subscribe messages and forward relevant events.

### Step 4: Register routes

```rust
.route("/workflows/latest/stream", get(workflow_stream))
.route("/workflow/ws", get(workflow_ws))
.route("/workflows/{id}", get(get_workflow))
```

## Write Scope
- `crates/roko-serve/src/routes/workflows.rs`


## Verify
```bash
cargo build -p roko-serve 2>&1 | head -30
cargo test -p roko-serve 2>&1 | tail -20
```
## Acceptance Criteria
- SSE endpoint sends initial `state` snapshot, then `delta` events
- WS endpoint accepts `subscribe` messages with projection filters
- PrdPipelinePanel can display plan/task state from these endpoints

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
