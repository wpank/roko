# 20-gateway/05 -- Handles and Batches

> Agent-facing keyless inference handles and the bounded asynchronous batch queue.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md), sections 13-14

**Source:** `crates/roko-gateway/src/handle.rs`, `crates/roko-gateway/src/batch.rs`

---

## 1. InferenceHandle: Key Isolation Boundary

The `InferenceHandle` is the principal security boundary of the gateway. Agents interact
with the gateway exclusively through this handle. It contains:

1. A **bounded channel sender** (`mpsc::Sender<InferenceEnvelope>`)
2. An **agent identity** (`AgentId`)
3. A **shared budget counter** (`Arc<AtomicU64>`, in microdollars)

It does **not** contain: API keys, provider references, cost tables, cache state, or
any other gateway internal. An agent that has an `InferenceHandle` can make inference
requests but cannot access credentials.

```rust
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceEnvelope>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,
}
```

### Handle Creation

Handles are created by the host via `InferenceGateway::create_handle(agent_id, budget)`.
The channel sender is cloned from the gateway's single bounded channel. The budget is
independent per handle -- different agents can have different budgets.

```rust
pub fn create_handle(&self, agent_id: impl Into<String>, budget_microdollars: u64)
    -> InferenceHandle
{
    InferenceHandle::new(self.sender.clone(), agent_id.into(), budget_microdollars)
}
```

### Cloning

Handles are `Clone`. Cloned handles share the same budget counter but can be used from
different tasks/threads. This enables fan-out patterns where multiple agent subtasks
share a single budget.

### InferenceClient Trait

The handle implements the `InferenceClient` trait:

```rust
#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn complete(&self, request: InferenceRequest)
        -> GatewayResult<InferenceResponse>;
    async fn stream(&self, request: InferenceRequest)
        -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>>;
}
```

This trait is also implemented by `InferenceGateway` itself, enabling the batch
processor to use the gateway directly as a client.

---

## 2. InferenceEnvelope

The internal message type that crosses the channel boundary:

```rust
pub struct InferenceEnvelope {
    pub agent_id: AgentId,
    pub request: InferenceRequest,
    pub(crate) budget: Arc<AtomicU64>,
    pub reply: InferenceReply,
}

pub enum InferenceReply {
    Complete(oneshot::Sender<GatewayResult<InferenceResponse>>),
    Stream(mpsc::Sender<GatewayResult<InferenceChunk>>),
}
```

The `reply` variant determines how the gateway sends the response back:

- **Complete:** One-shot channel. The gateway sends exactly one `Result<InferenceResponse>`.
- **Stream:** Multi-producer channel (capacity 64). The gateway sends two items: a text
  delta chunk and a done+usage chunk.

### Stream Accounting Invariant

Stream callers receive the completed response chunked into two items **after** the full
pipeline runs. This means:
- Cost is computed and deducted before any chunks are emitted
- Cache store happens before streaming begins
- No stage can be bypassed by using the stream API

This differs from a true incremental streaming architecture (where chunks flow as the
provider produces them). The current approach prioritizes accounting correctness over
streaming latency. True incremental streaming is roadmap work.

---

## 3. Gateway Loop

The gateway processes envelopes in a dedicated task started by `spawn_gateway_loop`:

```rust
pub fn spawn_gateway_loop(self: &Arc<Self>) -> GatewayResult<JoinHandle<()>> {
    let mut receiver = self.receiver.lock()?.take()
        .ok_or(GatewayError::AlreadyStarted)?;
    let gateway = Arc::clone(self);
    Ok(tokio::spawn(async move {
        while let Some(envelope) = receiver.recv().await {
            let gateway = Arc::clone(&gateway);
            tokio::spawn(async move {
                gateway.process(envelope).await;
            });
        }
    }))
}
```

Key properties:
- **Single receiver:** The receiver is taken from a `Mutex<Option<Receiver>>` exactly
  once. A second call returns `AlreadyStarted`.
- **Per-request tasks:** Each envelope spawns its own tokio task, enabling concurrent
  processing.
- **Bounded channel:** The channel capacity (default 200) matches the global
  backpressure limit. When the channel is full, `handle.infer()` blocks.

---

## 4. Batch Queue

The `BatchQueue` enables asynchronous processing at reduced cost.

### Architecture

```rust
pub struct BatchQueue {
    entries: Mutex<VecDeque<BatchEntry>>,
    results: DashMap<String, BatchResult>,
    flush_interval: Duration,     // default: 30s
    flush_size: usize,            // default: 50
    max_queue: usize,             // default: 1,000
    processor: Arc<dyn BatchProcessor>,
}
```

### Submission

```rust
pub async fn submit(&self, mut request: InferenceRequest) -> GatewayResult<String> {
    request.metadata.is_batch = true;
    let custom_id = format!("roko-{}", Uuid::new_v4());
    // ... push to VecDeque, auto-flush if >= flush_size
}
```

The batch flag is set automatically so the cost tracker applies the 50% discount.
The custom ID format `roko-{uuid}` is stable and can be used as a correlation key.

### Auto-Flush Triggers

| Trigger | Threshold | Behavior |
|---------|-----------|----------|
| **Item count** | 50 items accumulated | Immediate flush on submit |
| **Time** | 30 seconds since oldest item | Periodic check via `spawn_auto_flush` |
| **Manual** | `POST /api/gateway/batch/flush` | Force-flush the current queue |

### BatchProcessor Trait

```rust
#[async_trait]
pub trait BatchProcessor: Send + Sync {
    async fn process_batch(&self, entries: Vec<BatchEntry>)
        -> Vec<(String, GatewayResult<InferenceResponse>)>;
}
```

Two implementations:

1. **`ClientBatchProcessor`:** Drains entries through `InferenceClient::complete` with
   bounded concurrency (`buffer_unordered(8)`). Every item gets the same pipeline
   processing as real-time requests. This is the shipped default.

2. **`BatchProcessor` trait:** Extension boundary for native provider batch APIs (e.g.,
   Anthropic's `/v1/messages/batches`). Roadmap work.

### Result Storage

```rust
pub struct BatchResult {
    pub response: Option<InferenceResponse>,
    pub status: BatchStatus,           // Pending, Complete, Failed
    pub error: Option<String>,
}
```

Results are stored in a `DashMap<String, BatchResult>` and retrieved via
`GET /api/gateway/batch/result/{id}`. The `Pending` -> `Complete`/`Failed` transition
happens atomically when the processor finishes an item.

### Queue Bounds

The queue has a hard maximum (default 1,000 items). Submissions beyond the limit are
rejected with `GatewayError::BatchQueueFull { capacity }` (HTTP 503).

### Polling Contract

The exported `BATCH_POLL_INTERVAL` constant is **60 seconds**. This is the contract for
future native provider batch processors to use as their polling interval.

---

## 5. GatewayHttpState

The HTTP adapter bundles the live gateway and batch queue:

```rust
#[derive(Clone)]
pub struct GatewayHttpState {
    pub gateway: Arc<InferenceGateway>,
    pub batch: Arc<BatchQueue>,
}
```

`GatewayHttpState::new(gateway)` creates the standard batch queue with a
`ClientBatchProcessor` backed by the same gateway. `spawn_batch_loop()` starts the
periodic auto-flush timer.

The host merges these routes into its Axum router via `gateway_routes(state)`.
Authentication is a host middleware concern -- the gateway routes do not validate
tokens themselves.
