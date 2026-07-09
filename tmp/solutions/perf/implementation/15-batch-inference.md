# 15 — Batch Inference for Plan Execution (novel)

> Bottleneck: when a plan contains many independent tasks (10+), each
> task issues its own HTTP request. Dispatching them one-at-a-time
> wastes wall-clock time. For supported providers, batched / parallel
> dispatch can reduce latency 30-50 % on plan execution.
>
> Target savings: 30-50 % wall-clock on multi-task plans.
> Effort: ≈8 h. Risk: high (provider behaviours diverge wildly).

---

## Goal & success criteria

After this change:

1. The plan executor groups dispatchable tasks by provider/model.
2. For providers that support real-time concurrent dispatch (all of
   them), the executor issues parallel requests up to a per-provider
   concurrency cap.
3. For providers that support **async batch APIs** (OpenAI Batch,
   Anthropic Message Batches, Gemini BatchPredict), an opt-in
   `--batch-async` mode uses them for cost reduction (50 %) at the
   expense of latency (24 h SLAs).
4. Failures in one task do not abort the rest of the batch.

Done when:

- `roko plan run` with 10 independent tasks completes in roughly
  `tasks_per_concurrency * single_task_time + setup_overhead`, not
  `10 * single_task_time`.
- A unit test with a mock provider verifies the concurrency cap is
  respected.
- `--batch-async` produces a JSONL output of `{task_id,
  batch_request_id, status}` and a follow-up `roko plan collect
  <plan_id>` command that polls for results.

---

## Background

- Source: `OPTIMIZATION-PLAYBOOK.md` §13.
- Provider matrix:

  | Provider | Real-time concurrent | Async batch | Cost discount |
  |---|---|---|---|
  | OpenAI Chat | ✓ | OpenAI Batch (24 h) | 50 % |
  | Anthropic | ✓ | Message Batches (24 h) | 50 % |
  | Gemini | ✓ | BatchPredict | ~30 % |
  | Cerebras | ✓ | ✗ | — |
  | Moonshot | ✓ | ✗ | — |
  | Ollama | ✓ (local) | ✗ | — |

- The `SHARED_HTTP_CLIENT` already pools connections; firing N
  concurrent requests uses pooled sockets transparently.
- DAG-aware concurrency already exists in
  `crates/roko-cli/src/orchestrate.rs` (search for `parallel_tasks` and
  `concurrency_limit`); this plan extends that with provider-aware
  caps and the async-batch escape hatch.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` (plan execution path) | The current per-task dispatch loop. |
| `crates/roko-agent/src/model_call_service.rs` | What we batch. |
| `crates/roko-agent/src/provider/mod.rs` | Per-provider config (URLs, headers, batch endpoints). |
| `crates/roko-cli/src/plan.rs` | Plan parsing; understand the task DAG shape. |
| `crates/roko-cli/src/bench_demo.rs` | Existing comparison harness — useful for measuring the win. |

---

## Code-level plan

The plan splits into two separate features that share the same
plumbing. Implement Feature A first; Feature B is opt-in.

### Feature A — Real-time parallel dispatch

#### Step A1 — Define a concurrency policy

```rust
// crates/roko-cli/src/dispatch/parallel.rs (NEW)

pub struct ConcurrencyPolicy {
    pub global_max: usize,                      // default 8
    pub per_provider: HashMap<String, usize>,   // overrides
}

impl ConcurrencyPolicy {
    pub fn limit_for(&self, provider: &str) -> usize {
        self.per_provider.get(provider).copied().unwrap_or(self.global_max)
    }
}
```

Provider-specific defaults:

```rust
fn default_provider_limits() -> HashMap<String, usize> {
    let mut m = HashMap::new();
    m.insert("openai".into(), 10);       // OpenAI's per-key RPM is generous
    m.insert("anthropic".into(), 5);     // tighter; respect TPM
    m.insert("gemini".into(), 5);
    m.insert("cerebras".into(), 4);
    m.insert("moonshot".into(), 3);
    m.insert("ollama".into(), 2);        // local CPU/GPU bound
    m
}
```

#### Step A2 — Group tasks by provider/model

```rust
fn group_dispatchable(tasks: &[ReadyTask]) -> HashMap<(String, String), Vec<&ReadyTask>> {
    let mut groups: HashMap<(String, String), Vec<&ReadyTask>> = HashMap::new();
    for t in tasks {
        groups.entry((t.provider.clone(), t.model.clone())).or_default().push(t);
    }
    groups
}
```

A `ReadyTask` is a task whose dependencies have all completed
(ready-to-run set from the existing DAG scheduler).

#### Step A3 — Dispatch each group with a `Semaphore` cap

```rust
async fn dispatch_group(
    tasks: Vec<&ReadyTask>,
    caller: Arc<dyn ModelCaller>,
    cap: usize,
) -> Vec<(TaskId, Result<ModelCallResponse>)> {
    let sem = Arc::new(tokio::sync::Semaphore::new(cap));
    let mut handles = Vec::with_capacity(tasks.len());
    for task in tasks {
        let permit = Arc::clone(&sem).acquire_owned().await.unwrap();
        let caller = Arc::clone(&caller);
        let req = build_model_call_request(task);
        let id = task.id.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit;     // released on drop
            let res = caller.call(req).await;
            (id, res)
        }));
    }
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        out.push(h.await.expect("dispatch task"));
    }
    out
}
```

#### Step A4 — Wire into the plan executor

The existing scheduler already produces ready-task batches per tick.
Replace the inner `for task in ready { dispatch(task).await; }` with:

```rust
let groups = group_dispatchable(&ready);
let mut futures = Vec::new();
for ((provider, model), tasks) in groups {
    let cap = policy.limit_for(&provider);
    let caller = self.warm_pool.acquire(&provider, &model)
        .await
        .map(|g| g.caller.clone())
        .unwrap_or_else(|| Arc::clone(&self.default_caller));
    futures.push(dispatch_group(tasks, caller, cap));
}
let all_results: Vec<Vec<_>> = futures::future::join_all(futures).await;
```

### Feature B — Async batch APIs (opt-in)

#### Step B1 — Add a trait

```rust
// crates/roko-agent/src/provider/batch.rs (NEW)

#[async_trait::async_trait]
pub trait BatchProvider: Send + Sync {
    async fn submit_batch(&self, requests: Vec<ModelCallRequest>) -> Result<BatchHandle>;
    async fn poll_batch(&self, handle: &BatchHandle) -> Result<BatchStatus>;
    async fn fetch_results(&self, handle: &BatchHandle) -> Result<Vec<ModelCallResponse>>;
}

pub struct BatchHandle {
    pub provider: String,
    pub batch_id: String,
    pub created_at: DateTime<Utc>,
}

pub enum BatchStatus {
    Pending,
    InProgress { progress: f64 },
    Completed,
    Failed { reason: String },
}
```

Implement for OpenAI:

```rust
// crates/roko-agent/src/provider/openai_batch.rs (NEW)

pub struct OpenAiBatchProvider {
    api_key: String,
    base_url: String,    // default https://api.openai.com/v1
}

#[async_trait::async_trait]
impl BatchProvider for OpenAiBatchProvider {
    async fn submit_batch(&self, requests: Vec<ModelCallRequest>) -> Result<BatchHandle> {
        // 1. Build a JSONL of the batch requests.
        let body = build_jsonl(&requests);
        // 2. Upload the file via /v1/files (purpose=batch).
        let file_id = upload_file(&self.api_key, body).await?;
        // 3. Create the batch via /v1/batches.
        let resp = http().post(format!("{}/batches", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "input_file_id": file_id,
                "endpoint": "/v1/chat/completions",
                "completion_window": "24h",
            }))
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(BatchHandle {
            provider: "openai".into(),
            batch_id: resp["id"].as_str().unwrap_or_default().to_string(),
            created_at: Utc::now(),
        })
    }
    // ... poll_batch / fetch_results similarly ...
}
```

Implement for Anthropic and Gemini following the same shape; their
endpoints differ but the trait is uniform.

#### Step B2 — `roko plan run --batch-async`

```rust
roko plan run plans/big/ --batch-async
```

Behaviour:

1. Run the DAG until you hit a layer where all ready tasks share a
   provider that implements `BatchProvider`.
2. Collect requests for the layer; call `submit_batch`.
3. Persist the `BatchHandle` to `.roko/plans/<plan_id>/batches.jsonl`.
4. Exit (do not block). Print:

   ```
   Submitted batch openai/<id> with 23 tasks. Estimated cost $0.42 (50% off real-time).
   Run `roko plan collect <plan_id>` to fetch results.
   ```

5. The new `roko plan collect` polls each pending batch via
   `poll_batch`, and on completion, runs `fetch_results`, replays the
   responses through the regular post-dispatch pipeline (verdict
   recording, learning feedback), and continues the DAG.

---

## Step-by-step execution

1. `git checkout -b perf/15-batch-inference-feature-A`.
2. Implement Feature A (Steps A1–A4).
3. Tests + macro-benchmark (10-task plan).
4. PR `perf(plan): provider-aware concurrent dispatch (novel-A)`.
5. After A merges, `git checkout -b perf/15-batch-inference-feature-B`.
6. Implement Feature B (Steps B1–B2).
7. Tests against a sandboxed OpenAI Batch endpoint OR a mock.
8. PR `feat(plan): --batch-async via provider Batch APIs (novel-B)`.

---

## Anti-patterns / things NOT to do

- **Do NOT remove serialisation between layers of the DAG.** The DAG
  encodes dependencies; layer N tasks may depend on layer N-1 outputs.
  Parallelism is *within* a layer.
- **Do NOT exceed provider rate limits to "go faster".** A 429 storm
  triggers the provider's exponential backoff and slows you down. The
  per-provider caps in `default_provider_limits` are conservative for
  a reason.
- **Do NOT use `tokio::spawn` without bounding via the `Semaphore`.**
  Unbounded spawn fires N concurrent requests instantly, smashing rate
  limits.
- **Do NOT mix Feature A and Feature B in the same PR.** A is
  always-on, low-risk. B is opt-in, high-risk (file uploads, polling,
  state machine across CLI invocations). Two PRs, two reviews.
- **Do NOT buffer batch responses in memory** for large plans. Stream
  results into the substrate as they complete.
- **Do NOT swallow per-task failures.** Each task gets its own
  `Result`. The plan should record per-task verdicts as it normally
  would; plan exit code reflects "all critical tasks passed" semantics.
- **Do NOT poll a batch faster than 30 s.** Provider batch endpoints
  rate-limit the poll interface separately. Use exponential backoff
  starting at 30 s.
- **Do NOT trust the provider's "completion window" to be tight.**
  OpenAI advertises 24 h but in practice usually completes in 1-4 h.
  Do not promise users a tight SLA.
- **Do NOT batch user-facing interactive runs** (`roko run`, chat).
  Batch is only for `roko plan run`.

---

## Test plan

Feature A:

```rust
#[tokio::test]
async fn dispatch_group_respects_concurrency_cap() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));
    let caller = Arc::new(MockCaller::new(in_flight.clone(), max_seen.clone(), 50));
    let tasks: Vec<_> = (0..20).map(|i| ready_task(i)).collect();

    let _ = dispatch_group(tasks.iter().collect(), caller, 4).await;
    assert!(max_seen.load(Ordering::Relaxed) <= 4);
}
```

Macro-benchmark: a 10-task plan with all tasks targeting the same
fast model. Wall-clock should drop from ≈10 × per-task latency to
≈ceil(10/4) × per-task latency.

Feature B:

```rust
#[tokio::test]
async fn openai_batch_round_trip_against_mock() {
    let mock = mock_openai_batch_server().await;
    let provider = OpenAiBatchProvider::new("test-key", &mock.url);
    let requests = vec![dummy_request(); 3];
    let handle = provider.submit_batch(requests).await.unwrap();
    // Mock advances to Completed instantly.
    let status = provider.poll_batch(&handle).await.unwrap();
    assert!(matches!(status, BatchStatus::Completed));
    let results = provider.fetch_results(&handle).await.unwrap();
    assert_eq!(results.len(), 3);
}
```

---

## Rollback plan

- Feature A: a config flag `[conductor.plan.parallel] = false`
  reverts to single-task-at-a-time dispatch.
- Feature B: `--batch-async` is opt-in; never default.
- `git revert` for either feature is mechanical; the new modules can
  remain as dead code.

---

## Status check (acceptance)

Feature A:
- [ ] `ConcurrencyPolicy` + `dispatch_group` exist with tests.
- [ ] Plan executor groups by provider/model and dispatches with caps.
- [ ] Macro-benchmark improvement ≥30 % on a 10-task plan.

Feature B:
- [ ] `BatchProvider` trait + at least one implementor.
- [ ] `--batch-async` and `roko plan collect` subcommand wired.
- [ ] Mock-server round trip test passes.
