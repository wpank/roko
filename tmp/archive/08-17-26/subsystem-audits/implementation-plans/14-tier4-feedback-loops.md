# 14 — Tier 4: Feedback Loop Completion (6 items, all OPEN)

Turn write-only paths into real learning loops. ~2-3 sessions.

**Source**: doc 41 backlog T4-29..T4-34, doc 37 (learning dead code).

---

## Cross-Cutting Notes

The roko learning subsystem has good plumbing (sinks, observers, EMA
threshold updates) but several "last mile" connections are missing:

- Knowledge candidates write to disk but no ingestor consumes them at
  runtime (T4-29).
- `RoutingObservationSink` records confidence-only because no
  `RoutingContext` is constructed (T4-30).
- Provider-specific usage parsers convert unknown to zero, poisoning cost
  telemetry (T4-31).
- Playbook store is populated; nothing reads it back into the prompt
  (T4-32).
- JSONL grows without bound (T4-33).
- Chat `/model` switch can mutate state then fail (T4-34).

### Anti-patterns to enforce

1. **No `Default::default()` for `RoutingContext` / `UsageObservation`.**
   Use `None` for unknown.
2. **No new `dispatch_*` paths.** Plumb through existing ones.
3. **Don't make a sink synchronous if it can block dispatch.** All writes
   are spawn-and-forget unless the contract demands acknowledgement.
4. **One provider parser per commit** for T4-31.
5. **No "rotate when convenient" — define the boundary as bytes-on-disk.**
   T4-33 is concrete: ≥10 MiB triggers rotation.

---

## [ ] T4-29: Wire `KnowledgeIngestionSink::with_ingestor()`

**Depends on**: T0-6 (filename alignment) — done.

**Why**: Today the sink writes JSONL at `.roko/learn/knowledge-candidates.jsonl`.
A separate offline pass is supposed to consume it. The runner-time
ingestion path exists (`.with_ingestor()`) but is not wired in
`commands/plan.rs`. Result: knowledge accumulates on disk without ever
becoming part of the live store; routing/prompt assembly never sees it.

**Files**:

- `crates/roko-cli/src/commands/plan.rs:380-396` — sink construction.
- `crates/roko-cli/src/runtime_feedback/knowledge.rs:62-91` — trait + setter.
- `crates/roko-neuro/src/admission.rs` — `KnowledgeStore` and admission API.
- `crates/roko-neuro/src/lib.rs` — re-exports.

### Step 1: Implement an ingestor adapter

In `crates/roko-cli/src/runtime_feedback/knowledge.rs` (or a new file
`runtime_feedback/knowledge_neuro.rs`):

```rust
use std::sync::Arc;
use roko_neuro::admission::{KnowledgeStore, KnowledgeAdmissionInput};

#[derive(Debug)]
pub struct NeuroKnowledgeIngestor {
    store: Arc<KnowledgeStore>,
}

impl NeuroKnowledgeIngestor {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl KnowledgeIngestor for NeuroKnowledgeIngestor {
    async fn ingest(&self, candidate: &KnowledgeCandidate) -> Result<(), anyhow::Error> {
        let input = KnowledgeAdmissionInput {
            plan_id: candidate.plan_id.clone(),
            task_id: candidate.task_id.clone(),
            model: candidate.model.clone(),
            provider: candidate.provider.clone(),
            kind: candidate.kind,
            // ...map remaining fields per KnowledgeAdmissionInput contract
        };
        self.store.admit(input).await?;
        Ok(())
    }
}
```

The exact shape of `KnowledgeAdmissionInput` is defined in
`crates/roko-neuro/src/admission.rs`. **Read that file first** and match
its API rather than guessing.

### Step 2: Wire it from `commands/plan.rs`

```rust
// commands/plan.rs around line 388
let neuro_store = Arc::new(roko_neuro::admission::KnowledgeStore::open(
    wd.join(".roko").join("neuro"),
)?);
let knowledge_ingestor = Arc::new(
    roko_cli::runtime_feedback::NeuroKnowledgeIngestor::new(neuro_store.clone()),
);

let feedback_facade = Arc::new(
    roko_cli::runtime_feedback::FeedbackFacade::new()
        .with_sink(Arc::new(EpisodeSink::at(&episodes_path)))
        .with_sink(Arc::new(RoutingObservationSink::new(cascade_router.clone())))
        .with_sink(Arc::new(
            KnowledgeIngestionSink::at(&knowledge_path)
                .with_ingestor(knowledge_ingestor.clone()),
        ))
        // After T2-20: ConductorObservationSink and DreamTriggerSink are gone.
);
```

Note T4-29 depends on T2-20 only as far as the construction site is concerned —
both touch the same lines. If T2-20 hasn't landed, leave the conductor/dream
sinks in place and just add `with_ingestor()`.

### Step 3: Confirm the store path is consistent

The store is initialized at `.roko/neuro/`. Verify other places that read
the store (chat dispatch, prompt assembly, cascade router context) point at
the same path. Search:

```bash
rg 'KnowledgeStore::open|neuro::admission::KnowledgeStore' crates/ -g '*.rs'
```

If two paths exist (`.roko/neuro/` vs `.roko/knowledge/`), this is a
larger drift bug; report and stop.

### Step 4: Tests

```rust
#[tokio::test]
async fn ingestor_called_on_successful_task() {
    let dir = tempdir().unwrap();
    let store = Arc::new(KnowledgeStore::open(dir.path()).await.unwrap());
    let ingestor = Arc::new(NeuroKnowledgeIngestor::new(store.clone()));
    let sink = KnowledgeIngestionSink::at(dir.path().join("kc.jsonl"))
        .with_ingestor(ingestor);
    sink.on_event(&FeedbackEvent::TaskCompleted {
        plan_id: "p".into(),
        task_id: "t".into(),
        outcome: outcome(),
        model_source: ModelChoiceSource::Router,
        succeeded: true,
    }).await.unwrap();
    let stored = store.recent_candidates(10).await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].task_id, "t");
}
```

### Verify

```bash
cargo test -p roko-cli runtime_feedback::knowledge --lib
cargo test -p roko-neuro admission --lib
rg '\.with_ingestor\(' crates/roko-cli/src/commands/plan.rs
# Should match (product use)
```

After landing, run an end-to-end plan and verify
`roko knowledge query <task>` returns the new entry. (If `roko knowledge query`
doesn't exist or is broken, that's plan 33 / 30 territory — file a follow-up,
don't fix here.)

### Do not

- Make ingestion synchronous if it blocks dispatch. The `.ingest()` call is
  in the sink's `on_event` which is awaited by the facade. If admission is
  slow, that's an admission bug to fix in `roko-neuro`, not in this sink.
- Skip the JSONL write when an ingestor is present. The disk record is the
  durable source of truth for replay/audit; ingestion is a runtime
  shortcut.
- Open a new store handle per event. Construct once at startup, share via
  `Arc`.
- Delete `KnowledgeIngestionSink::at()` (no-ingestor constructor) — it's
  used in tests and deferred-ingest scenarios.

**Estimated effort**: 3-4 hours.

---

## [ ] T4-30: Thread real `RoutingContext` through dispatch

**Depends on**: T1-8 (already done — model/provider on `TaskAttemptCompleted`).

**Why**: The cascade router has two learning APIs:

- `record_confidence_outcome(model, success)` — confidence-only update;
  doesn't use context features.
- `observe_multi_objective(ctx: &RoutingContext, outcome)` — full
  contextual update via LinUCB; uses the feature vector.

Today, `RoutingObservationSink` (`runtime_feedback/routing.rs:78-81`)
calls **only** the confidence-only API because it doesn't have a
`RoutingContext`. The contextual learner is starved.

The `RoutingContext` features that need plumbing:

- task complexity (small/medium/large hint)
- budget pressure (cost remaining vs budget)
- selection reason (router/override/default)
- task domain (research/code/test/etc)
- file count touched (estimate)
- prior gate pass rate for this task family

### Step 1: Define what `RoutingContext` needs (read first)

Check `crates/roko-learn/src/cascade_router.rs` for the `RoutingContext`
struct definition. It's a feature vector; the LinUCB observer accepts a
`(context, outcome)` pair.

### Step 2: Capture context at dispatch time

In `crates/roko-cli/src/orchestrate.rs::dispatch_agent_with` (line ~14575+),
construct a `RoutingContext` snapshot just before the model call:

```rust
let routing_context = RoutingContext {
    model: selected_model.clone(),
    provider: selected_provider.clone(),
    selection_reason: model_choice_source.into(),
    task_complexity: estimate_task_complexity(&task),
    budget_pressure: budget.pressure_ratio(),
    domain_tag: task.domain.clone(),
    files_touched_estimate: task.files.len() as u32,
    prior_pass_rate: prior_pass_rate_for_task_family(&task),
};
```

This struct must be serializable; its values flow as part of the agent
turn record.

### Step 3: Plumb through `AgentOutcome` / `RunnerEvent`

Add `routing_context: Option<RoutingContext>` to:

- `crates/roko-cli/src/dispatch::AgentOutcome` (alongside `model`,
  `provider`).
- `crates/roko-cli/src/runner/types::RunnerEvent::TaskAttemptCompleted`.
- `crates/roko-cli/src/runtime_feedback::FeedbackEvent::TaskCompleted`.

The conversion in `runner_event_to_feedback` populates it from the runner
event.

### Step 4: Update the sink

`crates/roko-cli/src/runtime_feedback/routing.rs:61-82`:

```rust
async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
    let FeedbackEvent::TaskCompleted {
        outcome,
        model_source,
        succeeded,
        ..
    } = event else { return Ok(()); };

    if let Some(ctx) = &outcome.routing_context {
        self.router.observe_multi_objective(ctx, RoutingOutcome {
            success: *succeeded,
            // ... cost/duration/quality outcome fields
        });
    } else {
        // Fallback if context unavailable; record confidence-only.
        self.router.record_confidence_outcome(&outcome.model, *succeeded);
    }

    if matches!(model_source, ModelChoiceSource::Override) {
        self.router.record_override_outcome(...);
    }
    Ok(())
}
```

### Step 5: Tests

```rust
#[tokio::test]
async fn contextual_observation_recorded_when_context_present() {
    let r = router();
    let sink = RoutingObservationSink::new(r.clone());
    let mut o = outcome(true);
    o.routing_context = Some(RoutingContext {
        task_complexity: TaskComplexity::Medium,
        budget_pressure: 0.3,
        // ...
    });
    sink.on_event(&FeedbackEvent::TaskCompleted {
        outcome: o,
        model_source: ModelChoiceSource::Router,
        succeeded: true,
        plan_id: "p".into(),
        task_id: "t".into(),
    }).await.unwrap();
    let stage = r.learning_stage();
    // Assert observation count incremented in contextual path
    assert_eq!(stage.contextual_observations(), 1);
}

#[tokio::test]
async fn missing_context_falls_back_to_confidence_only() { /* ... */ }
```

### Verify

```bash
cargo test -p roko-cli routing --lib
cargo test -p roko-learn cascade --lib
rg 'observe_multi_objective' crates/roko-cli/src/runtime_feedback/
```

After 50+ observations, `roko learn router status` (if exists) should
show "Stage 2: contextual" instead of "Stage 1: confidence-only."

### Do not

- Use `RoutingContext::default()` when fields are missing. Use
  `Option<RoutingContext>` and fall back to confidence-only.
- Move `record_override_outcome` behind contextual gating. Operator
  overrides are recorded regardless.
- Add `routing_context` to ALL feedback events — only `TaskCompleted`
  needs it.
- Plumb `RoutingContext` through `roko-acp` or `roko-serve` in this PR;
  one transport at a time. Land the runner path first.

**Estimated effort**: 4-6 hours.

---

## [ ] T4-31: Migrate provider parsers to `UsageObservation`

**Why**: `UsageObservation` exists in `roko-core` with `Option<u64>` fields
that distinguish "absent" from "zero." OpenAI-compatible and Perplexity
parsers preserve it; Anthropic / Ollama / Gemini / Cerebras / Cursor
parsers still convert absent to zero.

Effect: cost telemetry shows fictitious zero-cost calls; routing learning
rewards "free" models that the provider just didn't report usage for.

**Files** (one per commit):

- `crates/roko-agent/src/providers/anthropic*.rs`
- `crates/roko-agent/src/providers/ollama*.rs`
- `crates/roko-agent/src/providers/gemini*.rs`
- `crates/roko-agent/src/providers/cerebras*.rs`
- `crates/roko-agent/src/providers/cursor*.rs`
- `crates/roko-agent/src/translate/openai.rs` — already migrated; verify
  it serves as a template.

### Per-provider procedure

For each provider:

1. **Locate the usage parser** — find where the provider's response is
   converted to `Usage` / `UsageObservation`. Often in
   `parse_response`, `consume_stream_event`, or a helper named
   `extract_usage`.

2. **Identify legacy fields**: today the parser may produce
   `Usage { input_tokens: u64, output_tokens: u64, ... }`. Convert it to
   `UsageObservation { input_tokens: Option<u64>, ... }`.

3. **Distinguish absent vs zero**:
   - JSON field missing → `None`.
   - JSON field `null` → `None`.
   - JSON field `0` → `Some(0)`.

4. **Plumb through the call site**: the `ModelCallResponse` (or its
   streaming equivalent) carries `UsageObservation` instead of `Usage`.
   Update `roko-agent/src/model_call_service.rs` if needed; coordinate
   with the equivalent ACP / serve consumers (compatibility adapter
   exists).

5. **Update consumer code** to preserve `Option`:
   - Cost calculation: `let cost = match usage.cost_usd {
     Some(c) => c, None => return TelemetryStatus::UsageUnknown };`
     (or similar). Don't default to 0.
   - Episode logging: if usage is `None`, write `null` in JSON, not `0`.

6. **Test with a fixture that has no usage block**:

```rust
#[test]
fn anthropic_parser_preserves_absent_usage() {
    let response = r#"{"id":"x","content":[{"type":"text","text":"hello"}],"stop_reason":"end_turn"}"#;
    let parsed = parse_response(response).unwrap();
    assert!(parsed.usage.is_none() || parsed.usage.input_tokens.is_none());
}

#[test]
fn anthropic_parser_distinguishes_zero_usage() {
    let response = r#"{"...","usage":{"input_tokens":0,"output_tokens":0}}"#;
    let parsed = parse_response(response).unwrap();
    assert_eq!(parsed.usage.input_tokens, Some(0));
}
```

### Per-provider notes

| Provider | Notes |
|---|---|
| Anthropic | Uses `usage` block in non-streaming; in streaming, `message_delta` events carry `usage`. Handle both. |
| Ollama | Returns `prompt_eval_count` / `eval_count` (different field names); map carefully. |
| Gemini | Returns `usageMetadata` with `promptTokenCount` / `candidatesTokenCount` / `totalTokenCount`. Cost is computed client-side; preserve `None` if any is missing. |
| Cerebras | OpenAI-compatible; uses the OpenAI-compatible parser. **Verify** it's not parsing through a separate path. If yes, share the parser. |
| Cursor | The Cursor proxy returns OpenAI-compatible bodies. Same as Cerebras. |

### Verify (per provider)

```bash
cargo test -p roko-agent providers::<name> --lib
cargo test -p roko-agent <name>_usage --lib
```

### Verify (after all providers)

```bash
rg 'cost_usd: 0\.0|input_tokens: 0' crates/roko-agent/src/providers/
# Should be empty in non-test code

rg 'UsageObservation' crates/roko-agent/src/providers/ -l
# Should list every provider adapter (~5+)
```

### Do not

- Bundle multiple providers in one commit. One per commit; easier to
  bisect.
- Change pricing math in the same commit. Pricing fixes go in a separate
  PR.
- Add a "default to zero if missing" fallback. Absent stays absent.
- Update the `Usage` type's API in this PR. Use `UsageObservation` where
  it already exists; deprecate the legacy `Usage` later.

**Estimated effort**: 30-60 minutes per provider (5 providers × ~45 min ≈
4 hours total).

---

## [ ] T4-32: Wire playbook store into system prompt builder

**Why**: The playbook store collects "this kind of task succeeded with this
strategy" entries from successful runs (T4-29 makes this real). The
prompt builder doesn't read them. Result: learned patterns don't surface
to the next agent.

**Files**:

- `crates/roko-prompt/src/lib.rs` — `SystemPromptBuilder`
- `crates/roko-prompt/src/playbook_layer.rs` (or wherever the playbook
  layer lives in the 9-layer assembly)
- `crates/roko-cli/src/orchestrate.rs::dispatch_agent_with` — site that
  constructs the builder
- `crates/roko-learn/src/playbook.rs` — the store

### Step 1: Define the playbook query API

Read `crates/roko-learn/src/playbook.rs`. The query is something like
`fn query(task_fingerprint: &str, top_k: usize) -> Vec<PlaybookEntry>`.

If the API is missing, add:

```rust
impl PlaybookStore {
    pub fn query_for_task(&self, task: &TaskFingerprint, top_k: usize) -> Vec<PlaybookEntry> {
        // Implementation: fingerprint-based similarity match
    }
}
```

### Step 2: Pass results into `SystemPromptBuilder`

In `dispatch_agent_with`, before building the prompt:

```rust
let playbook_hits = playbook_store.query_for_task(&task_fingerprint, 3);
let prompt = SystemPromptBuilder::new()
    .identity(&agent.identity)
    .capability(&agent.capability)
    .role(&agent.role)
    .task(&task.description)
    .context(&context_pack)
    .playbooks(&playbook_hits)        // <-- new layer feed
    .build();
```

The builder's `.playbooks(...)` method takes the typed `Vec<PlaybookEntry>`
and renders them into the prompt's playbook section. Format:

```
## Relevant Playbooks
1. [success_rate: 0.92, used: 12 times] When refactoring auth handlers,
   <playbook body>
2. ...
```

### Step 3: Confirm only retrieved playbooks land

The playbook layer must be **empty** when no playbooks match. Today, the
9-layer assembly may insert a placeholder section like
`"## Relevant Playbooks\n(none)"` — that wastes tokens and adds noise.
Suppress when empty.

### Step 4: Tests

```rust
#[test]
fn playbook_layer_omitted_when_empty() {
    let prompt = SystemPromptBuilder::new()
        .task("hello")
        .playbooks(&[])
        .build();
    assert!(!prompt.contains("Playbook"));
}

#[test]
fn playbook_entries_inserted_in_section() {
    let entries = vec![
        PlaybookEntry {
            title: "Auth refactor".into(),
            body: "Use AsyncRead/AsyncWrite at the boundary.".into(),
            success_rate: 0.92,
            used: 12,
        },
    ];
    let prompt = SystemPromptBuilder::new()
        .task("refactor auth")
        .playbooks(&entries)
        .build();
    assert!(prompt.contains("AsyncRead/AsyncWrite at the boundary"));
    assert!(prompt.contains("0.92") || prompt.contains("92%"));
}
```

### Step 5: Confirm closed loop

After T4-29 + T4-32 land, run a plan that exercises a known task family
twice. The second run's prompt (capture via tracing) should contain a
playbook entry derived from the first run.

### Verify

```bash
cargo test -p roko-prompt playbook --lib
cargo test -p roko-cli orchestrate_playbook --lib
rg 'playbook_store|PlaybookStore' crates/roko-cli/src/orchestrate.rs
```

### Do not

- Insert all playbooks regardless of relevance. Cap at top-3 and use
  fingerprint similarity.
- Inject playbook bodies into the system prompt unfiltered. Validate they
  pass safety contract checks first if they contain commands/URLs.
- Hardcode an "always include" canonical playbook in this layer. If we
  want canonical guidance, that's the `identity` or `role` layer.
- Plumb playbook retrieval through ACP/serve in this PR. Land the runner
  path first.

**Estimated effort**: 3-4 hours, more if the playbook query API is missing.

---

## [ ] T4-33: Add JSONL rotation for episodes / efficiency / knowledge

**Why**: `.roko/learn/episodes.jsonl`, `.roko/learn/efficiency.jsonl`,
`.roko/learn/knowledge-candidates.jsonl` grow unbounded. After a few weeks
of heavy use they degrade IO performance and complicate ingestion.

**Files**:

- `crates/roko-cli/src/runtime_feedback/episodes.rs` (or wherever
  `EpisodeSink` writes)
- `crates/roko-cli/src/runtime_feedback/knowledge.rs:99-112` (`write` fn)
- Whatever writes `efficiency.jsonl` (likely `roko-learn` or a
  serve/runner helper)

### Step 1: Add a shared rotator helper

Create `crates/roko-cli/src/runtime_feedback/rotator.rs`:

```rust
use std::path::Path;

const ROTATION_THRESHOLD_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
const KEEP_ROTATED: usize = 5;

/// Rotate `path` if it exceeds `threshold`. Keeps the last `keep` archives.
/// Naming: `path` → `path.1`, `path.1` → `path.2`, etc.
pub async fn maybe_rotate(path: &Path, threshold: u64, keep: usize) -> Result<(), std::io::Error> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if metadata.len() < threshold {
        return Ok(());
    }
    // Shift n..1 down: path.{n} -> deleted; path.{n-1} -> path.{n}; path -> path.1
    for i in (1..keep).rev() {
        let from = path.with_extension(format!("jsonl.{i}"));
        let to = path.with_extension(format!("jsonl.{}", i + 1));
        let _ = tokio::fs::rename(&from, &to).await; // ignore not-found
    }
    let oldest = path.with_extension(format!("jsonl.{keep}"));
    let _ = tokio::fs::remove_file(&oldest).await; // best-effort
    let first_archive = path.with_extension("jsonl.1");
    tokio::fs::rename(path, &first_archive).await?;
    Ok(())
}

pub fn default_rotation() -> (u64, usize) {
    (ROTATION_THRESHOLD_BYTES, KEEP_ROTATED)
}
```

(Filename convention: if the source is `episodes.jsonl`, the rotated files
are `episodes.jsonl.1` through `episodes.jsonl.5`. Keep the canonical file
extension intact for tooling.)

### Step 2: Call before each append

For each sink that writes JSONL:

```rust
async fn write(&self, candidate: &KnowledgeCandidate) -> Result<(), anyhow::Error> {
    if let Some(parent) = self.candidates_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let mut guard = self.file.lock().await;

    // Check rotation. If rotated, drop our cached file handle.
    if guard.is_some() {
        let (threshold, keep) = crate::runtime_feedback::rotator::default_rotation();
        if let Err(e) = crate::runtime_feedback::rotator::maybe_rotate(
            &self.candidates_path, threshold, keep,
        ).await {
            tracing::warn!(error = %e, "knowledge candidate rotation failed");
        } else if let Ok(meta) = tokio::fs::metadata(&self.candidates_path).await {
            // If file was renamed, rotator created `path.1` and `path` is now missing.
            if !self.candidates_path.exists() {
                *guard = None;  // force re-open
            }
        }
    }

    if guard.is_none() {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.candidates_path)
            .await?;
        *guard = Some(file);
    }
    // ... rest of write
}
```

Performance note: `tokio::fs::metadata` on each event is ~1-2 µs.
Acceptable.

### Step 3: Apply to all three writers

- `EpisodeSink` (the path is something like `.roko/learn/episodes.jsonl`)
- `KnowledgeIngestionSink`
- Efficiency JSONL writer (find via `rg 'efficiency.jsonl' crates/`)

### Step 4: Tests

```rust
#[tokio::test]
async fn rotates_when_threshold_exceeded() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    let big = "x".repeat(1_000_000) + "\n";
    for _ in 0..15 {
        tokio::fs::OpenOptions::new()
            .create(true).append(true).open(&path).await.unwrap()
            .write_all(big.as_bytes()).await.unwrap();
    }
    // file is now ~15 MB
    crate::runtime_feedback::rotator::maybe_rotate(&path, 10_000_000, 3).await.unwrap();
    assert!(path.with_extension("jsonl.1").exists());
    assert!(!path.exists() || tokio::fs::metadata(&path).await.unwrap().len() < 10_000_000);
}

#[tokio::test]
async fn keeps_only_n_archives() {
    // Create path.1 through path.5 with different content.
    // Trigger rotation. Confirm path.1..5 retained, none beyond.
}
```

### Verify

```bash
cargo test -p roko-cli rotator --lib
rg 'maybe_rotate' crates/roko-cli/src/runtime_feedback/
```

### Do not

- Compress rotated files in this PR. That's a separate task.
- Rotate based on age (e.g., daily). Size-based is simpler and adequate.
- Use `std::fs` blocking calls in async sinks. Use `tokio::fs`.
- Remove a rotated file mid-rotation; the rename-shift sequence handles
  cleanup atomically.

**Estimated effort**: 2-3 hours.

---

## [ ] T4-34: Make chat `/model` switch atomic

**Why**: Today the chat REPL `/model <name>` may mutate `agent_session.model`
even when full resolution fails. The user ends up with a session whose
display says one thing and whose dispatch says another.

**File**: `crates/roko-cli/src/chat_inline.rs` (line ~2761; search for
`/model` handler).

### Step 1: Refactor to resolve-then-commit

```rust
async fn handle_model_command(&mut self, args: &str) -> Result<(), ChatError> {
    let candidate_slug = args.trim();
    if candidate_slug.is_empty() {
        return Err(ChatError::usage("/model <slug>"));
    }

    // Build the entire next state in temporary values; do not mutate self yet.
    let next_model = self.config.resolve_model(candidate_slug)
        .ok_or_else(|| ChatError::unknown_model(candidate_slug))?;
    let next_provider = next_model.provider.clone();
    let next_auth = self.auth_for_provider(&next_provider)
        .ok_or_else(|| ChatError::missing_auth(&next_provider))?;
    let next_call_config = self.build_model_call_config(&next_model, &next_auth)?;
    let next_display = self.display_for_model(&next_model);

    // Commit. From here, no fallible operations.
    self.agent_session.model = next_model;
    self.agent_session.model_call_config = next_call_config;
    self.agent_session.display.set_model(next_display);
    Ok(())
}
```

### Step 2: Identify all the fields that change on switch

Audit `chat_inline.rs` for every site that reads or writes a field
related to "current model":

```bash
rg 'agent_session\.(model|model_call_config|display|provider|adapter)' crates/roko-cli/src/chat_inline.rs
```

The `next_*` block must capture all of them.

### Step 3: Tests

```rust
#[tokio::test]
async fn failed_model_switch_leaves_session_unchanged() {
    let mut session = test_chat_session();
    let original_model = session.agent_session.model.clone();
    let result = session.handle_model_command("totally-fake-model").await;
    assert!(result.is_err());
    assert_eq!(session.agent_session.model, original_model);
    assert_eq!(session.agent_session.display.current_model(), original_model.display);
}

#[tokio::test]
async fn successful_switch_updates_all_fields_atomically() {
    let mut session = test_chat_session();
    session.handle_model_command("gpt-5").await.unwrap();
    assert_eq!(session.agent_session.model.slug, "gpt-5");
    assert_eq!(session.agent_session.display.current_model(), "GPT-5");
    // model_call_config also updated
}
```

### Step 4: Document the contract

Add a doc-comment to `handle_model_command`:

```rust
/// Switches the active model atomically.
///
/// On failure (unknown model, missing auth, invalid config), no field of
/// `agent_session` is mutated. The error is returned to the user; the
/// previous model remains in effect.
async fn handle_model_command(&mut self, args: &str) -> Result<(), ChatError>
```

### Verify

```bash
cargo test -p roko-cli chat_model_switch --lib
rg 'agent_session\.model = ' crates/roko-cli/src/chat_inline.rs
# Should appear inside the commit block of handle_model_command, nowhere else
```

### Do not

- Use a "best effort rollback" pattern (mutate, then try to restore).
  Resolve-then-commit is the only correct shape.
- Skip auth resolution in fast-path "model is already known" cases. Auth
  may have changed.
- Land partial fields independently — every changed field belongs to the
  same atomic switch.
- Change ACP `/model` semantics in this PR (different surface, separate
  commit).

**Estimated effort**: 2-3 hours.

---

## Combined Verification

```bash
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Knowledge ingestion path live
rg '\.with_ingestor\(' crates/roko-cli/src/commands/plan.rs

# Routing context plumbed
rg 'observe_multi_objective' crates/roko-cli/src/runtime_feedback/

# Provider parsers preserve unknown
rg 'cost_usd: 0\.0|input_tokens: 0' crates/roko-agent/src/providers/  # 0 results in non-test code

# Playbook layer reads store
rg 'playbook_store|playbooks\(' crates/roko-cli/src/orchestrate.rs

# Rotation in place
rg 'maybe_rotate' crates/roko-cli/src/runtime_feedback/

# Atomic /model
rg '_ Resolved =' crates/roko-cli/src/chat_inline.rs # commit-pattern marker, e.g.
```

---

## Status

- [ ] T4-29 — Wire `KnowledgeIngestionSink::with_ingestor()`
- [ ] T4-30 — Thread real `RoutingContext` through dispatch
- [ ] T4-31 — Migrate provider parsers to `UsageObservation`
- [ ] T4-32 — Wire playbook store into system prompt builder
- [ ] T4-33 — Add JSONL rotation
- [ ] T4-34 — Make chat `/model` switch atomic

**After completion**: every learning sink has a real consumer, every
provider preserves unknown vs zero, the prompt builder uses learned
playbooks, and chat model switching can never leave a torn state.

Move on to Tier 5 (`15-tier5-architectural.md`).
