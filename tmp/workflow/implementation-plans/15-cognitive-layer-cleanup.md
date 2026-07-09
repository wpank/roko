# 15 — Cognitive Layer Cleanup: Pheromones, Daimon, HDC, Distillation

> Cross-cutting plan covering `tmp/workflow/14-cognitive-layer-audit.md`. Net: ~110K LOC deleted, ~5K LOC added.

---

## Status (2026-05-01)

**NOT STARTED.** All four cognitive subsystems still present.

| Component | LOC | Status | Verdict |
|---|---|---|---|
| `roko-neuro` (knowledge store) | ~4,047 | Active in `WorkflowEngine` via `PromptAssemblyService` (per plan 02) | KEEP |
| Episode distillation | ~950 | Live but reads `ANTHROPIC_API_KEY` directly (anti-pattern #1) | KEEP, refactor to `ModelCallService` |
| `roko-dreams` (consolidation) | ~2,000 | Live, manually triggered or post-plan | KEEP, simplify trigger |
| `roko-daimon` (PAD model) | ~40,000 | Loaded but PAD model unused for decisions | DELETE → `FailureTracker` |
| Pheromones (`coordination.rs`) | ~68,000 | Created but never observed meaningfully | DELETE → `Vec<String>` warnings |
| HDC fingerprinting | ~ (in `roko-learn`) | Computed per episode; no read consumer | DELETE |
| Custody chain | ~200 | CLI-inspect only | IGNORE for now |

---

## Goal

Delete pheromones (68K LOC). Replace daimon (40K LOC) with `FailureTracker` (~200 LOC). Delete HDC fingerprinting from episode write path. Migrate distillation to `ModelCallService`. Keep `roko-neuro` knowledge store and `roko-dreams` consolidation; they have observable runtime impact.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#1 Just Shell Out** — distillation reads `ANTHROPIC_API_KEY` directly
- **#10 God file** — `coordination.rs` 68K LOC; `roko-daimon/lib.rs` 40K LOC
- **#6 Feedback Afterthought** — knowledge written from dead orchestrate path; pheromones never read back

---

## Existing Code — Read These First

- `crates/roko-orchestrator/src/coordination.rs` — pheromones (delete target)
- `crates/roko-daimon/` — PAD model (delete target, except trait stub)
- `crates/roko-neuro/src/episode_completion.rs` — `Distiller`, calls Claude API directly
- `crates/roko-neuro/` — knowledge store (KEEP)
- `crates/roko-dreams/src/runner.rs` — dream consolidation (KEEP)
- `roko_core::foundation::AffectPolicy` + `NoOpAffectPolicy` — already exists; don't duplicate

---

## Implementation Steps

### Step 1 — Delete pheromones

**File:** `crates/roko-orchestrator/src/coordination.rs` (delete entirely; ~68K LOC)

Pre-conditions:

- `WorkflowEngine` does not consume pheromones (true — already)
- `PromptAssemblyService` Layer 8 (warnings) replaces it (per plan 02 § Step 1 — warnings is `Vec<String>`)

Steps:

1. Find every usage:
   ```bash
   rg '(PheromoneStore|active_pheromone_chunks|deposit_pheromone|coordination::)' crates/ --type rust
   ```
2. For each call site:
   - If it's "create pheromone on gate fail" → instead, `warning_store.push(format!("gate failed: {}", err))` 
   - If it's "read pheromones into prompt" → already replaced by Layer 8 warnings
   - If it's "decay pheromones over time" → delete
3. Add `WarningStore` if not present:
   ```rust
   // crates/roko-runtime/src/warning_store.rs
   pub struct WarningStore {
       warnings: Mutex<VecDeque<String>>,
       max: usize,
   }
   impl WarningStore {
       pub fn push(&self, msg: String) {
           let mut w = self.warnings.lock().unwrap();
           if w.len() >= self.max { w.pop_front(); }
           w.push_back(msg);
       }
       pub fn snapshot(&self) -> Vec<String> {
           self.warnings.lock().unwrap().iter().cloned().collect()
       }
   }
   ```
4. `EffectServices` carries `warnings: Arc<WarningStore>`; `EffectDriver` populates `PromptSpec.warnings = warnings.snapshot()` on every spawn
5. Delete `coordination.rs`
6. Delete `coordination_test.rs` and any pheromone-related tests
7. Delete `crates/roko-orchestrator/src/coordination/` directory if it exists

### Step 2 — Replace daimon with FailureTracker

**Files:** All of `crates/roko-daimon/` (delete after replacement)

Add a `FailureTracker` to `roko-runtime`:

```rust
// crates/roko-runtime/src/failure_tracker.rs
#[derive(Debug, Default)]
pub struct FailureTracker {
    consecutive_failures_by_role: Mutex<HashMap<String, u32>>,
    last_failure_kind: Mutex<HashMap<String, FailureKind>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Compile, Test, Permission, Network, Unknown,
}

impl FailureTracker {
    pub fn record_failure(&self, role: &str, kind: FailureKind) {
        *self.consecutive_failures_by_role.lock().unwrap().entry(role.into()).or_insert(0) += 1;
        self.last_failure_kind.lock().unwrap().insert(role.into(), kind);
    }
    pub fn record_success(&self, role: &str) {
        self.consecutive_failures_by_role.lock().unwrap().insert(role.into(), 0);
        self.last_failure_kind.lock().unwrap().remove(role);
    }
    pub fn consecutive_failures(&self, role: &str) -> u32 {
        *self.consecutive_failures_by_role.lock().unwrap().get(role).unwrap_or(&0)
    }
    pub fn should_restrict_tools(&self, role: &str, threshold: u32) -> bool {
        self.consecutive_failures(role) >= threshold
    }
}
```

Wire it as a `FeedbackSink` so it auto-updates from `FeedbackEvent::TaskCompleted`:

```rust
// crates/roko-learn/src/sinks/failure_tracker_sink.rs
pub struct FailureTrackerSink { tracker: Arc<FailureTracker> }
#[async_trait]
impl FeedbackSink for FailureTrackerSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        if let FeedbackEvent::TaskCompleted { role, success, gate_verdicts, .. } = event {
            if success { self.tracker.record_success(&role); }
            else {
                let kind = classify_failure_kind(&gate_verdicts);
                self.tracker.record_failure(&role, kind);
            }
        }
        Ok(())
    }
}
```

`SafetyLayer::check_agent_recovery` (per plan 09 § Step 6) consults `FailureTracker` to decide on `RecoveryAction::Downgrade` etc. Same for routing context (per plan 08 § Step 1, `prior_failure: bool`).

Then delete:

- `crates/roko-daimon/` entire crate
- `roko_daimon` from `Cargo.toml`
- Any `DaimonState`, `PadVector`, `AlmaLayers`, `SomaticMarker` references
- `daimon_state_path()` calls

The `roko_core::foundation::AffectPolicy` trait + `NoOpAffectPolicy` stay (so future "real" affect could be added behind a feature flag later).

### Step 3 — Migrate distillation to `ModelCallService`

**File:** `crates/roko-neuro/src/episode_completion.rs`

Today `Distiller` reads `ANTHROPIC_API_KEY` directly. Refactor:

```rust
pub struct Distiller {
    service: Arc<dyn ModelCaller>,        // injected
    distill_role: String,                 // "distiller" — define template
    batch_size: usize,
}

impl Distiller {
    pub async fn distill(&self, episodes: &[Episode]) -> Result<Vec<KnowledgeEntry>> {
        let req = ModelCallRequest {
            model: String::new(),               // router decides
            system: Some(distill_system_prompt()),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: format_episodes_for_distillation(episodes),
            }],
            role: Some(self.distill_role.clone()),
            caller: Some("neuro.distillation".into()),
            cache_policy: CachePolicy::ForceRefresh,
            ..Default::default()
        };
        let resp = self.service.call(req).await?;
        parse_knowledge_entries_from_json(&resp.content)
    }
}
```

`Distiller::new` takes `Arc<dyn ModelCaller>` — caller wires it via `ServiceFactory`.

Remove `ClaudeDistillationBackend`. Delete `ANTHROPIC_API_KEY` env reads from this file.

### Step 4 — Simplify dream consolidation triggers

**File:** `crates/roko-dreams/src/runner.rs`

Today triggers:

1. `maybe_auto_dream()` after plan completion
2. `maybe_coordination_dream()` on conductor critical patterns (dies with pheromones)
3. Manual `roko knowledge dream run`

After:

1. Plan completion (keep)
2. ManualDreamCommand (keep)
3. Periodic schedule (NEW): `roko serve` runs a background dream task every N hours if `[learn].auto_dream_interval_hours > 0`
4. Conductor trigger (DELETE — relied on pheromones)

Also: dream agent uses `ModelCallService` not `create_agent_for_model`:

```rust
let req = ModelCallRequest {
    role: Some("dream_reviewer".into()),
    caller: Some("dreams".into()),
    cache_policy: CachePolicy::Default,
    ...
};
let resp = self.service.call(req).await?;
```

### Step 5 — Delete HDC fingerprinting (per plan 03 § 6)

Already in plan 03. Cross-reference here for completeness.

`crates/roko-learn/src/hdc.rs` — delete. Episode struct keeps `hdc_fingerprint: Option<String>` for backward compat reading old episodes; new episodes set `None`.

### Step 6 — Delete custody chain (or move to dedicated crate)

**File:** `crates/roko-orchestrator/src/safety/custody_*` (if exists)

Audit doc 14 § 1: "Custody CLI only — IGNORE". For this plan: leave `custody` CLI inspect command working but remove integration from runtime hot path.

Move to `crates/roko-cli/src/commands/custody.rs` as a self-contained inspect tool. Delete from any runtime path.

### Step 7 — Re-verify knowledge loop

After plans 02 + 03 the knowledge store should be consulted on every prompt assembly and updated on every completed episode. Verify:

```rust
#[tokio::test]
async fn knowledge_round_trip() {
    let temp = tempdir()?;
    let services = ServiceFactory::for_test(temp.path()).await?;

    // Run 1: complete a task with a known pattern
    let outcome = run_task(&services, "implement add(a, b) with type checks").await?;
    assert!(outcome.success);

    // Verify knowledge entry created
    let store = roko_neuro::KnowledgeStore::open(temp.path().join(".roko/neuro/knowledge.jsonl"))?;
    let entries = store.recent(10);
    assert!(entries.iter().any(|e| e.title.contains("type check")));

    // Run 2: similar task; verify knowledge appears in prompt
    let prompt = services.assembler().assemble(PromptSpec {
        role: Some("implementer".into()),
        task: Some("implement subtract(a, b) with type checks".into()),
        ..Default::default()
    }).await?;
    assert!(prompt.diagnostics.knowledge_ids.iter().any(|id| id.starts_with("kn-")));
    assert!(prompt.system.contains("type check"));
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #1 Just shell out | Distillation reads env var | Inject `Arc<dyn ModelCaller>` |
| #10 God file | Pheromones / daimon | Delete |
| #4 Wrong layer | Putting failure tracking back inside daimon | `FailureTracker` is in `roko-runtime` |

---

## Things NOT To Do

1. **Don't keep `roko-daimon` "in case affect is useful later".** Delete the crate. The `AffectPolicy` trait survives in `roko-core` so future implementations can plug in.
2. **Don't keep pheromones behind a feature flag.** They have no readers; flagged dead code rots.
3. **Don't migrate `coordination.rs` to a new file.** Delete.
4. **Don't drop episode distillation.** It's useful — knowledge entries from past runs improve future prompts. Just refactor the API.
5. **Don't add a "soft delete" for daimon.** Hard delete; add `#[deprecated]` aliases for one release if external crates depend.
6. **Don't keep HDC computation "for diagnostics".** No consumer = waste cycles on every episode write.
7. **Don't auto-trigger dreams during plan execution.** They're slow (minutes); confines to post-plan or scheduled.
8. **Don't merge `Distiller` into `ModelCallService`.** Distillation is a use case of the service, not part of it.

---

## Tests / Proof Criteria

```bash
# 1. Pheromones gone
rg 'PheromoneStore|active_pheromone_chunks' crates/ --type rust
# expected: 0

# 2. Daimon crate deleted
ls crates/roko-daimon
# expected: directory does not exist

# 3. FailureTracker exists
rg 'pub struct FailureTracker' crates/roko-runtime/src/ --type rust
# expected: 1

# 4. Distillation uses ModelCallService
rg 'ANTHROPIC_API_KEY' crates/roko-neuro/ --type rust
# expected: 0

# 5. HDC deleted
ls crates/roko-learn/src/hdc.rs 2>/dev/null
# expected: file does not exist
```

Functional proofs:

- [ ] `roko run "implement add(a, b)"` writes a knowledge entry within 5 minutes (via async distillation)
- [ ] Subsequent `roko run "implement subtract(a, b)"` includes the prior entry in Layer 3 (knowledge)
- [ ] After 3 consecutive failures of role `implementer`, `FailureTracker.consecutive_failures("implementer")` == 3 and `should_restrict_tools` returns true
- [ ] `roko knowledge dream run` consolidates recent episodes into knowledge entries
- [ ] `cargo test -p roko-runtime failure_tracker` passes
- [ ] Total `crates/` LOC reduction ≥ 100,000 after this plan

---

## Dependencies

- **Plan 01 (ModelCallService)** — for distillation refactor
- **Plan 02 (PromptAssembly)** — Layer 8 warnings replaces pheromones
- **Plan 03 (FeedbackService)** — `FailureTrackerSink` consumes events
- **Plan 09 (Safety)** — `check_agent_recovery` consults `FailureTracker`

Can start after 01-03 land; not blocking other plans.

---

## Estimated Effort

**M.** ~1-1.5 weeks. Mostly deletion.

- Step 1 (pheromones) — M (3 days; many call sites to clean up)
- Step 2 (daimon → FailureTracker) — M (2-3 days; many references to remove)
- Step 3 (distillation) — S (1 day)
- Step 4 (dreams simplify) — S (1 day)
- Step 5 (HDC) — S (half day; mostly plan 03)
- Step 6 (custody isolation) — S (half day)
- Step 7 (knowledge round-trip test) — S (1 day)
