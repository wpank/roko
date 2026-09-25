# 27.04 -- Mutation Consent and Experiments

> Depth file for [27-ACP.md](../../27-ACP.md).

---

## Mutation Consent Protocol

ACP sessions use a consent protocol before performing file mutations. The
server can request permission from the IDE before making changes to the
workspace.

### Request Flow

```
Server -> Client: session/request_permission
   { "sessionId": "abc", "action": "file_write", "path": "src/main.rs" }

Client -> Server: (response)
   { "approved": true }
```

The `send_request()` method on `StdioTransport` sends a JSON-RPC request and
waits for the matching response via a `oneshot` channel:

```rust
pub async fn send_request(
    &mut self,
    method: &str,
    params: serde_json::Value,
) -> TransportResult<JsonRpcResponse> {
    let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = oneshot::channel();
    self.pending_requests.lock()?.insert(request_id, sender);
    // Send request and await response
    self.write_message(&request).await?;
    receiver.await
        .map_err(|_| TransportError::ResponseChannelClosed { request_id })
}
```

### Cancellation Safety

A `PendingRequestGuard` ensures the pending registry is cleaned up if the
request future is cancelled (e.g., by a `session/cancel` notification):

```rust
struct PendingRequestGuard {
    request_id: u64,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
}

impl Drop for PendingRequestGuard {
    fn drop(&mut self) {
        self.pending_requests.lock().ok().map(|mut p| p.remove(&self.request_id));
    }
}
```

### File Change Notifications

After mutations are performed, the server notifies the IDE:

```rust
pub struct FileChangeNotification {
    pub path: String,
    pub change_type: FileChangeType,  // Created | Modified | Deleted
}
```

These are sent as `session/update` notifications:

```json
{
  "sessionUpdate": "file_change",
  "fileChange": {
    "path": "src/main.rs",
    "changeType": "modified"
  }
}
```

### Worktree Change Detection

The `WorktreeChangeSnapshot` struct in the ACP runner captures git state before
and after opaque provider phases:

```rust
pub(crate) struct WorktreeChangeSnapshot {
    head: Option<String>,           // HEAD commit hash
    dirty: BTreeMap<String, Option<String>>,  // path -> content hash
}
```

`capture()` records HEAD and content signatures for all dirty paths.
`changed_files()` diffs the before/after snapshots to identify net changes,
including files modified by commits the provider created internally.

## Prompt Experiments (A/B Testing)

The ACP server supports prompt experiments for A/B testing different system
prompt configurations.

### Experiment Assignment

The `ExperimentStore` from `roko-learn` manages experiment state:

```rust
pub struct ExperimentStore {
    experiments: Vec<Experiment>,
    assignments: HashMap<String, String>,  // session_id -> variant
}
```

When a new session is created, it is assigned to experiment variants:

1. Check active experiments in the store
2. Apply random assignment based on traffic allocation
3. Record the assignment for the session lifetime
4. Replace canonical prompt sections with experiment variants

### ACP Context Injection

In the current implementation, ACP sessions inject experiment context into
the provider prompt rather than using the runner's full receipt protocol.
This is noted as a parity gap:

> ACP/serve still inject context rather than using the runner receipt protocol.

The runner-side experiment integration uses `ExperimentReceipt` to track
which canonical prompt sections were replaced and bind exact prelaunch prompts.
ACP achieves the same effect through direct context concatenation.

### Settlement

Experiments are settled (concluded) based on terminal outcomes:

1. Gate results (pass/fail) from the pipeline runner
2. User feedback signals
3. Cost and latency metrics
4. Budget exhaustion events

Settlement is idempotent and uses archived/live terminal facts.

### Config Options for Experiments

Active experiments surface as session config options:

```rust
ConfigOption {
    id: "experiment_variant",
    label: "Prompt Experiment",
    option_type: ConfigOptionType::Enum,
    current_value: ConfigOptionValue::String("control".into()),
    enum_values: Some(vec!["control", "variant_a", "variant_b"]),
}
```

The IDE can display the current variant and allow manual override for
debugging purposes.

## Session Modes

ACP sessions support multiple editing modes that affect prompt composition
and tool availability:

```rust
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct ModesInfo {
    pub available_modes: Vec<ModeInfo>,
    pub current_mode: String,
}
```

Mode changes are applied via `session/set_mode`:

```json
{
  "sessionId": "abc-123",
  "modeId": "plan"
}
```

The server responds with updated config options that reflect the new mode's
available settings.

## Session Config Options

The config option system provides a typed interface for IDE dropdowns and
toggles:

```rust
pub struct ConfigOption {
    pub id: String,
    pub label: String,
    pub option_type: ConfigOptionType,
    pub current_value: ConfigOptionValue,
    pub enum_values: Option<Vec<String>>,
}

pub enum ConfigOptionType {
    String,
    Number,
    Boolean,
    Enum,
}
```

Standard config options include:

| Option ID | Type | Description |
|---|---|---|
| `model` | Enum | Active model selection |
| `effort` | Enum | Reasoning effort (low/medium/high/max) |
| `budget` | Number | Session budget in USD |
| `sandbox_level` | Enum | Sandbox enforcement level |
| `review_strictness` | Enum | Gate review strictness |
| `clippy_enabled` | Boolean | Run clippy gate |
| `tests_enabled` | Boolean | Run test gate |

Config updates trigger `session/update` notifications with the full updated
option list.

## Source

- `crates/roko-acp/src/transport.rs` -- Request/response with pending registry
- `crates/roko-acp/src/runner.rs` -- WorktreeChangeSnapshot and file change detection
- `crates/roko-acp/src/session.rs` -- Session config options and modes
- `crates/roko-acp/src/types.rs` -- ConfigOption, ModeInfo, FileChangeNotification
- `crates/roko-learn/src/prompt_experiment.rs` -- ExperimentStore
