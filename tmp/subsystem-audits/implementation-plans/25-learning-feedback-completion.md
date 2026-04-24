# 25 — Learning Feedback Completion

The learning subsystem has good plumbing but several "last mile" gaps
that prevent feedback loops from closing. This plan combines the T4-29..
T4-32 fixes (which appear in plan 14) with deeper completion: deleting
the dead modules (which T2-17 only does the obvious cases of), making
the cascade router's stage progression visible, and ensuring every sink
has a real consumer.

Source: doc 35 § Telemetry and learning, doc 37 (learning-feedback dead
code), doc 41 T4-29..T4-33.

This plan **complements** plan 14 (Tier 4) and plan 12 (Tier 2). The
extra detail here:

- Deeper dead-code removal in `roko-learn`.
- Cascade router learning-stage observability.
- Episode/efficiency JSONL schema versioning for safe rotation.
- Knowledge ingestion error budget.

---

## Today's State (verified 2026-05-01)

- Cascade router has 37 model slugs and 28 role→model mappings learned
  from real episodes (per audit 42).
- `RoutingObservationSink` records confidence-only.
- `KnowledgeIngestionSink` writes JSONL but no ingestor consumes at runtime.
- `EpisodeSink` writes; routing data is correct (T1-8 fixed model/provider).
- 14 unused learn modules + 4 orphan files (T2-16, T2-17 in plan 12).
- `playbook` store exists; not consumed by `SystemPromptBuilder` (T4-32).

---

## Anti-Patterns

1. **No new sink without a real consumer.** A sink that writes and is
   never read is dead.
2. **No `Default::default()` for `RoutingContext`.** Use `Option`.
3. **No "shadow mode" learners.** If a learner doesn't influence
   anything, delete or activate it.
4. **No JSONL append without rotation.** All writers respect the size
   bound (T4-33).
5. **No fan-out to consumers in `interested(event)` returning `true` for
   everything.** Hot path; selective subscription.

---

## Plan

### Phase A: Make cascade router learning visible

The router has internal stages (confidence-only → contextual after
threshold). Today this is invisible to the user. Audit 42 cites this as
"learning is invisible."

**File**: `crates/roko-learn/src/cascade_router.rs` and
`crates/roko-cli/src/commands/learn.rs` (or wherever `roko learn` lives).

#### Step 1: Expose stage state

```rust
impl CascadeRouter {
    pub fn learning_stage(&self) -> LearningStage {
        LearningStage {
            stage: self.current_stage(),    // ConfidenceOnly | Contextual
            observations: self.total_observations(),
            stage_threshold: self.stage_threshold(),
            top_models_by_pass_rate: self.top_models(5),
            top_role_mappings: self.top_role_mappings(5),
        }
    }
}
```

#### Step 2: `roko show learning` (or equivalent CLI command)

```rust
// crates/roko-cli/src/commands/learn.rs

pub async fn show_learning(workdir: &Path) -> Result<()> {
    let router = load_cascade_router(workdir)?;
    let stage = router.learning_stage();
    println!("Learning Stage: {:?}", stage.stage);
    println!("Observations:   {}", stage.observations);
    println!();
    println!("Top models by pass rate (last 30 days):");
    for entry in stage.top_models_by_pass_rate {
        println!("  {:<32} {:>3.0}% pass  {:>5} obs  ${:.3} avg",
            entry.slug, entry.pass_rate * 100.0, entry.observations, entry.avg_cost);
    }
    Ok(())
}
```

#### Step 3: Surface in chat / TUI

- Chat REPL `/learn` slash command.
- TUI Learning tab (already exists; populate with stage data).

**Estimated effort**: 4-6 hours.

---

### Phase B: Knowledge ingestion error budget

After T4-29 wires `with_ingestor`, ingestion happens per task. If the
neuro store hits an error (disk full, schema mismatch), today the error
propagates back to the runner, which may abort the task.

The ingestion call is **secondary correctness**: failure shouldn't abort
the run. Bound the failure rate; alert if exceeded.

#### Implementation

In `KnowledgeIngestionSink`:

```rust
pub struct KnowledgeIngestionSink {
    candidates_path: PathBuf,
    file: Mutex<Option<tokio::fs::File>>,
    ingestor: Option<Arc<dyn KnowledgeIngestor>>,
    // New:
    ingestion_failures: AtomicU32,
    ingestion_total: AtomicU32,
    failure_budget_percent: u32,    // e.g. 5 (allow 5% failure)
}

async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
    // ... write JSONL (always; this is the durable record)

    if let Some(ingestor) = &self.ingestor {
        self.ingestion_total.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = ingestor.ingest(&candidate).await {
            let f = self.ingestion_failures.fetch_add(1, Ordering::Relaxed) + 1;
            let t = self.ingestion_total.load(Ordering::Relaxed);
            tracing::warn!(error = %e, failures = f, total = t, "knowledge ingestion failed");
            if t > 20 && (f * 100 / t) > self.failure_budget_percent {
                tracing::error!(
                    "knowledge ingestion failure rate {:.1}% exceeds budget {}%",
                    (f as f64) * 100.0 / (t as f64),
                    self.failure_budget_percent,
                );
                // Surface to operator; do not abort the runner.
            }
        }
    }
    Ok(())
}
```

The JSONL write remains the durable source; ingestion failures don't
lose data.

**Estimated effort**: 1-2 hours.

---

### Phase C: Episode schema versioning for rotation

T4-33 adds JSONL rotation. To safely consume rotated files (and to migrate
between schema versions), each line carries a schema version.

#### Implementation

```rust
#[derive(Serialize, Deserialize)]
pub struct Episode {
    pub schema_version: u32,
    pub episode_id: String,
    pub plan_id: String,
    pub task_id: String,
    pub model: String,
    pub provider: String,
    pub usage: Option<UsageObservation>,
    pub gate_outcomes: Vec<GateOutcomeRecord>,
    pub started_at: i64,
    pub finished_at: i64,
    pub success: bool,
    // ... per existing schema
}

const EPISODE_SCHEMA_VERSION: u32 = 2;  // bump when changing
```

Reader logic:

```rust
pub fn read_episodes(path: &Path) -> impl Iterator<Item = Result<Episode, ReadError>> {
    BufReader::new(File::open(path).unwrap())
        .lines()
        .filter_map(|line| {
            let l = line.ok()?;
            // Try current version first
            if let Ok(ep) = serde_json::from_str::<Episode>(&l) {
                return Some(Ok(ep));
            }
            // Try migration from earlier versions
            if let Ok(legacy) = serde_json::from_str::<EpisodeV1>(&l) {
                return Some(Ok(legacy.into()));
            }
            // Otherwise skip with warning
            Some(Err(ReadError::UnknownSchema(l)))
        })
}
```

**Estimated effort**: 2-3 hours.

---

### Phase D: Delete partial-truth modules (deeper than T2-17)

T2-17 deletes 14 modules with **zero** external callers. Some other
modules have callers but are partial-truth (write something, no real
consumption). Examples to evaluate:

- `roko-learn::error_pattern_store` — does anything read its output?
- `roko-learn::routing_log` — duplicates `EpisodeSink`?
- `roko-learn::costs_log` vs `roko-learn::costs_db` — pick one.
- `roko-learn::routing_extras` — extras of what?

For each, run:

```bash
rg 'roko_learn::<module>' crates/ -g '*.rs' | rg -v 'crates/roko-learn/'
```

If callers exist but the module's output isn't read by any decision-making
code, mark for deletion. If kept, document the consumer.

This phase requires investigation per module; budget 30-60 min each.

**Estimated effort**: 4-8 hours total.

---

### Phase E: Fix `event_subscriber` and `feedback_service`

These two modules in `roko-learn` overlap with `runtime_feedback` in
`roko-cli`. The audit's "feedback as afterthought" anti-pattern shows
up as duplicate event-routing.

**Files**:

- `crates/roko-learn/src/event_subscriber.rs`
- `crates/roko-learn/src/feedback_service.rs`
- `crates/roko-cli/src/runtime_feedback/`

#### Step 1: Map the overlap

```bash
rg 'event_subscriber|feedback_service::' crates/ -g '*.rs'
```

Find: which crate is the canonical source of `FeedbackEvent` /
`FeedbackSink`?

#### Step 2: Pick one canonical home

The trait + facade live in `roko-cli`'s `runtime_feedback/` module
(observed via the construction site at `commands/plan.rs:380`). Move
any unique functionality from `roko-learn::feedback_service` into
`runtime_feedback`. Delete the duplicate.

`event_subscriber` may serve a different purpose (subscribing to the
broadcast event bus); evaluate whether it's needed at all. If unused,
delete; if used, rename to disambiguate.

**Estimated effort**: 4-6 hours.

---

### Phase F: Cascade router stage progression test

Add an integration test that drives the router from confidence-only to
contextual stage and asserts the transition.

```rust
#[tokio::test]
async fn router_progresses_to_contextual_after_threshold() {
    let r = CascadeRouter::new(vec!["a".into(), "b".into()]);
    assert_eq!(r.learning_stage().stage, LearningStageKind::ConfidenceOnly);

    // Feed N observations with full RoutingContext
    for i in 0..50 {
        let ctx = RoutingContext { /* features */ };
        r.observe_multi_objective(&ctx, RoutingOutcome { success: i % 3 != 0, ... }).await;
    }

    // After 50 contextual observations, stage should advance
    assert_eq!(r.learning_stage().stage, LearningStageKind::Contextual);
}
```

This catches regressions where T4-30's plumbing breaks the contextual
update path.

**Estimated effort**: 1-2 hours.

---

## Combined Verification

```bash
cargo test -p roko-learn cascade --lib
cargo test -p roko-cli runtime_feedback --lib

# Learning visible
roko show learning   # or: cargo run -p roko-cli -- learn show

# Ingestion error budget enforced
# (manual test: configure broken ingestor, run plan, verify run completes
# even if ingestion fails)

# Schema version present
head -1 .roko/learn/episodes.jsonl | jq .schema_version

# No duplicate feedback subsystem
rg 'pub mod feedback_service' crates/roko-learn/src/   # 0 if deleted
rg 'pub mod event_subscriber' crates/roko-learn/src/   # 0 or single canonical home
```

---

## Status

- [ ] Phase A — Cascade router learning stage visible
- [ ] Phase B — Knowledge ingestion error budget
- [ ] Phase C — Episode schema versioning
- [ ] Phase D — Delete partial-truth modules (deeper than T2-17)
- [ ] Phase E — Resolve `event_subscriber` / `feedback_service` overlap
- [ ] Phase F — Stage progression integration test

**Estimated total effort**: 16-30 hours, depending on Phase D investigation
breadth.
