# Orchestrator and Learning Gaps

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Folded from `tmp/bardo-integration-plan.md` Phases 2-3. Original bardo source references preserved.

---

## Orchestrator gaps (from mori)

These features exist in bardo/mori but are not yet in roko's orchestrator (`crates/roko-cli/src/orchestrate.rs`).

---

### 1. Structured review verdict system

**Source**: `bardo/apps/mori/src/orchestrator/review.rs`
**Target**: `crates/roko-gate/src/review_verdict.rs` + wire into orchestrate.rs

Parse agent review output into structured verdicts with issue classification.

**Types**:
- `ReviewVerdict` enum: `Approve | Revise | Skip`
- `ReviewIssue { severity: IssueSeverity, category: IssueCategory, file: Option<String>, line: Option<u32>, description: String }`
- `IssueSeverity` enum: `Blocking | Major | Minor`
- `IssueCategory` enum: `Compilation | Test | TypeMismatch | MissingImpl | Docs | Style | SpecDeviation`
- `IssueCategory::is_quick_fixable()` → true for Compilation, Docs, Style
- `StructuredReview { verdict, issues: Vec<ReviewIssue>, summary: String }`
- `StructuredReview::all_issues_quick_fixable()` → true when all issues are quick-fixable

**Parsing**: Try JSON first, then JSON code block, then TOML block. Provide JSON schema for reviewer agents.

**Integration**: In orchestrate.rs, after review phase, parse agent output as StructuredReview. If `all_issues_quick_fixable()`, skip strategist and go directly to implementer (express mode).

**Acceptance criteria**:
- [ ] `StructuredReview` parses from JSON agent output
- [ ] `IssueCategory::is_quick_fixable()` returns correct values
- [ ] `all_issues_quick_fixable()` correctly identifies trivial-fix scenarios
- [ ] Fallback parsing handles malformed JSON gracefully (returns Revise with raw text)
- [ ] Integration test: mock review JSON → parsed verdict → correct phase transition

**Size**: M (2-3 days)

---

### 2. Compile error classification + auto-fix

**Source**: `bardo/apps/mori/src/orchestrator/autofix.rs`
**Target**: `crates/roko-gate/src/compile_errors.rs` + wire into orchestrate.rs

Parse `cargo check --message-format=json` output into classified error types for targeted auto-fix.

**CompileErrorClass** enum:
- `ImportNotFound { module, item, file, line }`
- `TypeMismatch { expected, found, file, line }`
- `MissingField { struct_name, field, file, line }`
- `TraitNotImplemented { type_name, trait_name, file, line }`
- `Other { code: String, message, file, line }`

**Functions**:
- `parse_cargo_json_errors(json_output: &str) -> Vec<CompileErrorClass>` — Extract `rendered`, `code`, `spans[0].file_name`, `spans[0].line_start`
- `collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>` — Extract `children[].suggested_replacement` from diagnostic JSON
- `apply_rustc_fixes(worktree: &Path)` — Run `cargo fix --allow-dirty` + `cargo fmt` to apply compiler-suggested fixes directly (no agent needed)

**Integration**: In orchestrate.rs autofix path, first try `apply_rustc_fixes()`. If that resolves all errors, skip agent retry. Otherwise, pass classified errors to agent instead of raw cargo output.

**Acceptance criteria**:
- [ ] `parse_cargo_json_errors()` extracts structured errors from real cargo JSON
- [ ] `CompileErrorClass` variants populated with correct file/line/details
- [ ] `collect_rustc_suggestions()` finds and extracts suggested replacements
- [ ] `apply_rustc_fixes()` runs cargo fix + fmt successfully
- [ ] Agent receives classified errors instead of raw output

**Size**: M (2-3 days)

---

### 3. Error pattern discovery + sharing

**Source**: `bardo/apps/mori/src/orchestrator/gates.rs`
**Target**: `crates/roko-gate/src/error_patterns.rs` + wire into orchestrate.rs

Share discovered error patterns across parallel agents.

**Functions**:
- `extract_error_digest(output: &str) -> String` — Parse cargo/test output, extract `error[E...]` blocks, deduplicate via HashSet, cap at 10 unique errors, cap each at 200 chars. Return compact digest.
- `append_discovered_pattern(repo_root, plan, error_digest)` — Write to `.roko/learn/discovered-patterns.json`. Format: `{ "patterns": [{ "plan", "digest", "timestamp", "resolved": bool }] }`
- `read_discovered_patterns() -> Vec<DiscoveredPattern>` — Read last 5 unresolved patterns (200 chars each). Used to inject into agent context so parallel agents learn from each other's failures.
- `GateResult::is_mostly_passing(results) -> bool` — >90% pass rate with >20 tests and ≥1 failure = "mostly passing". Means a targeted fix should suffice (not full replan).

**Integration**: In orchestrate.rs, after gate failure: call `extract_error_digest()` → `append_discovered_pattern()`. Before agent dispatch: call `read_discovered_patterns()` → inject into system prompt.

**Acceptance criteria**:
- [ ] `extract_error_digest()` produces compact, deduped error signatures from real cargo output
- [ ] Patterns persisted to `.roko/learn/discovered-patterns.json`
- [ ] Parallel agents see each other's patterns (read from shared file)
- [ ] `is_mostly_passing()` returns true for 95% pass with 1 failure, false for 50% pass
- [ ] Pattern injection visible in agent system prompt

**Size**: M (2 days)

---

### 4. Post-gate reflection loop

**Source**: `bardo/apps/mori/src/orchestrator/reflection.rs`, `bardo/apps/mori/src/orchestrator/iteration_memory.rs`
**Target**: `crates/roko-cli/src/orchestrate.rs` (new function) + `crates/roko-learn/src/episode_logger.rs` (add field)

After gate failure, spawn a lightweight agent to analyze what went wrong.

**Specification**:
1. **Trigger**: After any gate failure (compile, test, clippy), before replanning
2. **Reflection agent**: Use cheapest model (haiku-4-5). Prompt: "Analyze this gate failure. What went wrong? What should the next attempt do differently? Gate output: {error_digest}. Files changed: {file_list}. Previous attempts: {iteration_count}."
3. **Output**: Store reflection text in episode's `reflection` field (add this field to Episode struct if missing)
4. **Injection**: On retry, inject last reflection into agent's system prompt as "Lessons from previous attempt: {reflection}"
5. **Deduplication**: If error_digest matches a previous reflection's error pattern, skip re-generating
6. **Cost guard**: Reflection must cost <$0.02 (cap max_tokens at 500)

**Acceptance criteria**:
- [ ] Reflection generated on gate failure (visible in episode log)
- [ ] Reflection injected into retry agent's prompt
- [ ] Deduplication: same error pattern doesn't trigger second reflection
- [ ] Cost capped: max_tokens=500, model=haiku
- [ ] Episode struct has `reflection: Option<String>` field

**Size**: M (2-3 days)

---

### 5. Context injection scoping

**Source**: `bardo/apps/mori/src/orchestrator/inject.rs`
**Target**: `crates/roko-compose/src/context_scoping.rs` + wire into orchestrate.rs

Scope playbook rules to plan's touched files and enable per-category toggles.

**KnowledgeConfig** struct with toggles:
- `file_intel_enabled: bool` (default true), `file_intel_max_entries: usize` (default 5)
- `warnings_enabled: bool`, `warning_max_entries: usize`
- `error_patterns_enabled: bool`, `error_pattern_min_cluster: usize`
- `wave_context_enabled: bool` (read context from sibling tasks in same wave)
- `dynamic_budget_enabled: bool` (adjust context size per file difficulty)

**`collect_plan_playbook_scope(plan, tasks) -> PlaybookScope`**: Extract file globs + tags from task checklist. Only match playbook rules whose `trigger_files` overlap with plan's file scope.

**Role-filtered context**: Different roles get different context sizes. Implementer gets full file intel. Reviewer gets summary only. Strategist gets none (sees plan-level only).

**Integration**: In orchestrate.rs `dispatch_agent_with()`, apply KnowledgeConfig to filter playbook rules and context before prompt assembly.

**Acceptance criteria**:
- [ ] `KnowledgeConfig` loadable from `roko.toml` (with defaults)
- [ ] `collect_plan_playbook_scope()` narrows rule matching to plan's files
- [ ] Implementer gets full context; reviewer gets summary; verified by prompt inspection
- [ ] Config toggles actually suppress sections

**Size**: M (2-3 days)

---

### 6. Warm agent spawning

**Source**: `bardo/apps/mori/src/agent/mod.rs` — `MultiAgentPool`, `pre_spawn_warm()`, `promote_warm()`, `evict_warm()`
**Target**: `crates/roko-runtime/src/warm_pool.rs` + wire into orchestrate.rs

Pre-spawn agents during gate execution for faster phase transitions.

**Specification**:
1. **WarmPool**: `HashMap<AgentRole, WarmAgent>` where `WarmAgent` = pre-spawned process ready for promotion
2. **`pre_spawn_warm(role, effort)`**: During gate pipeline execution, spawn the next phase's agent in the background. The agent initializes but doesn't receive a task yet.
3. **`promote_warm(role) -> AgentConnection`**: Swap warm agent to active. The agent receives its task and starts working immediately. Saves 5-15s vs cold spawn.
4. **`evict_warm(role)`**: Kill warm agent on gate failure (no point keeping it if plan is replanning)

**Integration**: In orchestrate.rs, after dispatching compile gate, call `pre_spawn_warm(Reviewer)`. When gate passes, call `promote_warm(Reviewer)`. When gate fails, call `evict_warm(Reviewer)`.

**Acceptance criteria**:
- [ ] Warm agent spawns in background during gate execution
- [ ] `promote_warm()` returns usable agent connection without re-spawn delay
- [ ] `evict_warm()` kills process and frees resources
- [ ] Timing test: promote is <100ms vs 5-15s for cold spawn
- [ ] No leaked processes on gate failure path

**Size**: M (2-3 days)

---

### 7. Conductor watchers (10 rules)

**Source**: `bardo/apps/mori/src/conductor/mod.rs` (600+ LOC), `bardo/apps/mori/src/conductor/watchers.rs`
**Target**: Extend `crates/roko-conductor/src/`

Battle-tested detection rules for agent stalls, loops, and resource exhaustion.

| # | Watcher | Trigger | Action |
|---|---------|---------|--------|
| 1 | **GhostTurn** | No output + fast turn (<5s) + not in gating | Restart agent |
| 2 | **ReviewLoop** | 3+ consecutive REVISE verdicts + gates pass | Skip remaining reviews |
| 3 | **IterationLoop** | Iteration ≥6 + cycling strategist/implementer | Force advance |
| 4 | **TestFailureBudget** | 70%+ tests pass but some fail | Force advance (good enough) |
| 5 | **SilenceTimeout** | No output for 180s | Restart agent |
| 6 | **CompileFailThreshold** | 3+ consecutive compile failures | Force advance |
| 7 | **TaskStall** | Single task blocking for 300s | Restart agent |
| 8 | **ContextPressure** | Prompt >80% of context window | Trim context |
| 9 | **PhaseTimeout** | Phase exceeds 30min wall-clock | Restart |
| 10 | **CooldownFilter** | Last intervention within 120s | Skip (debounce) |

Each watcher returns `Option<Intervention { tier, watcher, target_role, message, action }>`.

**Acceptance criteria**:
- [ ] All 10 watchers implemented and registered in conductor
- [ ] CooldownFilter prevents intervention storms (tested with rapid triggers)
- [ ] Each watcher's threshold configurable (in `roko.toml` or conductor config)
- [ ] Interventions logged with tier/watcher/target for observability
- [ ] Unit tests for each watcher with mock ConductorContext

**Size**: L (3-4 days)

---

## Learning loop gaps

These extend roko's learning subsystem with patterns from mori/bardo.

---

### 8. Wire neuro store into cascade router

**Source**: `bardo/crates/golem-grimoire/src/` (grimoire retrieval scoring)
**Target**: `crates/roko-learn/src/cascade_router.rs`
**Existing**: `crates/roko-neuro/src/` (knowledge store), `crates/roko-learn/src/cascade_router.rs` (model routing)

Currently the cascade router selects models based on observations (pass/fail history) but does NOT consult the neuro store.

**Specification**:
1. At `decide()` time, query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge
2. If knowledge entries mention specific model preferences, bias model scoring by +0.1 for mentioned model
3. If knowledge entries describe failure patterns with a model, bias by -0.1
4. Add knowledge context to LinUCB feature vector (add 2 dims: `knowledge_match_score`, `knowledge_model_bias`)
5. Make opt-in via `cascade_router.consult_knowledge: bool` in config (default true)

**Acceptance criteria**:
- [ ] Cascade router queries neuro store at decide time
- [ ] Model bias applied based on knowledge entries
- [ ] LinUCB context vector extended with knowledge features
- [ ] Config toggle works (disabled = no knowledge query)
- [ ] No performance regression: knowledge query <10ms

**Size**: M (2 days)

---

### 9. Episode clustering for error patterns

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: Extend `crates/roko-learn/src/pattern_discovery.rs`

Cluster failed episodes by error signature to recommend model fallbacks.

**Functions**:
- `cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster>` — Group by `error_signature` (failures) or `file_pattern` (successes). Minimum cluster size: 3.
- `EpisodeCluster`: `{ key, count, success_rate, common_files, best_model, best_provider, avg_cost_usd }`
- Per cluster, compute which model has highest success_rate → `recommended_model`

**Integration**: Feed cluster recommendations into cascade_router as soft priors. When a new task matches a cluster's file pattern, bias toward recommended_model.

**Cadence**: Run clustering every 10 new episodes (use existing `UpdateFrequency` mechanism).

**Acceptance criteria**:
- [ ] `cluster_episodes()` groups episodes with matching error signatures
- [ ] Clusters with 3+ episodes produce model recommendations
- [ ] Recommendations integrated as soft bias in cascade_router
- [ ] Clustering runs on cadence (every 10 episodes)
- [ ] Test: 5 episodes with same error + model A succeeding → recommends model A

**Size**: M (2-3 days)

---

### 10. Provider pass-rate into model scoring

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: `crates/roko-learn/src/cascade_router.rs`
**Existing**: `crates/roko-learn/src/provider_health.rs`

Bias model selection toward proven providers.

**Specification**:
1. `compute_provider_metrics(episodes)` → per-provider: pass_rate, avg_cost, avg_duration (min 5 episodes)
2. `recommend_provider(metrics)` → pick provider with highest pass_rate
3. In cascade_router Stage 2 (confidence) and Stage 3 (LinUCB): multiply model score by `provider_pass_rate`
4. Use existing ProviderHealthTracker data if available, fall back to episode-derived metrics

**Acceptance criteria**:
- [ ] Provider metrics computed from episode history
- [ ] Model scores multiplied by provider pass_rate in stages 2-3
- [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6
- [ ] Minimum 5 episodes before provider metrics affect scoring

**Size**: S (1 day)

---

### 11. Reflection-derived playbook rules

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: Extend `crates/roko-learn/src/playbook_rules.rs`

Auto-generate playbook rules from agent reflections (§4 above).

**Specification**:
1. After reflection is stored in episode, extract actionable patterns:
   - If reflection mentions specific files → create rule with `trigger_files` glob
   - If reflection mentions error type → create rule with `trigger_tags`
   - Context injection = the reflection's key insight
2. **Confidence**: New rules start at 0.5 (neutral). Boost +0.05 on gate pass, penalize -0.10 on gate fail. Remove rules below 0.2 confidence (unless manually created).
3. **Cadence**: Run after every 3 new reflections
4. **Persistence**: Append to `.roko/learn/playbook-rules.json` with `source: "reflection"` tag

**Acceptance criteria**:
- [ ] Reflections with file mentions → playbook rules with trigger_files
- [ ] Confidence tracking: +0.05 on success, -0.10 on failure
- [ ] Rules below 0.2 auto-removed
- [ ] Manually created rules preserved (never auto-removed)
- [ ] Persistence in playbook-rules.json with `source: "reflection"` tag

**Size**: M (2 days)

---

### 12. A-MAC admission gate for neuro store

**Source**: `bardo/crates/golem-grimoire/src/` (A-MAC 5-factor admission gate)
**Target**: Extend `crates/roko-neuro/src/`

Prevent hallucinated or contradictory knowledge from entering the store.

**5-factor validation before any knowledge entry is stored**:
1. **Similarity**: Too similar to existing knowledge? (cosine sim > 0.95 → reject as duplicate)
2. **Novelty**: Does this add new information? (cosine sim < 0.3 to all existing → novel)
3. **Contradiction**: Does this contradict existing high-confidence entries? (semantic opposition check)
4. **Relevance**: Is this relevant to the agent's domain? (keyword match against domain tags)
5. **Confidence**: Does the source have sufficient credibility? (gate pass rate of the episode that generated this)

Gate result: `Admit | Reject { reason }`. Log rejections for debugging.

**Acceptance criteria**:
- [ ] Near-duplicate entries rejected (similarity > 0.95)
- [ ] Contradictory entries flagged (if existing entry has confidence > 0.8)
- [ ] Novel entries admitted with appropriate confidence score
- [ ] Rejections logged with reason
- [ ] Unit test: insert duplicate → rejected; insert novel fact → admitted; insert contradiction → flagged

**Size**: M (2-3 days)

---

## Current state reconciliation

> Added 2026-04-24. Cross-references actual crate state to identify what already exists vs what needs building.

### Already implemented (do NOT rebuild)

| Gap | Item | Location | Status |
|-----|------|----------|--------|
| 1 | `ReviewDecision` enum (Approve, Revise, Skip) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 1 | `ReviewIssue` struct (category, gate, rung, file, line, suggestion, blocking) | `roko-gate/src/review_verdict.rs` | **EXISTS** — 10 issue categories |
| 1 | `ReviewVerdict` struct (decision, summary, issues, rung_results) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 2 | `CompileError` struct (category, code, message, file, line, column, suggestion) | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 2 | `ErrorCategory` enum (10 categories) | `roko-gate/src/compile_errors.rs` | **EXISTS** — Syntax, UnresolvedImport, TypeMismatch, Lifetime, MissingMember, Unused, Visibility, Macro, TraitBound, Ownership, Other |
| 2 | `classify_error_code()` function | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 3 | `ErrorPattern` struct for cross-error pattern matching | `roko-conductor/src/diagnosis.rs` | **EXISTS** — conductor diagnoses at policy level |
| 7 | All 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — GhostTurn, ReviewLoop, IterationLoop, TestFailureBudget, Silence, CompileFailRepeat, TaskStall, ContextPressure, TimeOverrun, CooldownFilter |
| 7 | Intervention system with tiers | `roko-conductor/src/interventions.rs` | **EXISTS** — BanditPolicy, WorstSeverityPolicy |
| 7 | Circuit breaker | `roko-conductor/src/circuit_breaker.rs` | **EXISTS** — Holt forecasting |

### Remaining work (gaps that still need implementation)

| Gap | What | Notes |
|-----|------|-------|
| 1 | **Wire ReviewVerdict parsing into orchestrate.rs** | Types exist but parsing agent output → verdict is not wired |
| 1 | **Express mode** (skip strategist when all issues quick-fixable) | Phase transition logic not wired |
| 2 | **`apply_rustc_fixes()` auto-fix path** | Run `cargo fix --allow-dirty` + `cargo fmt` before spawning agent |
| 2 | **Wire classified errors into agent prompt** | Agent still gets raw cargo output instead of structured errors |
| 3 | **Error pattern sharing between parallel agents** | Pattern file exists but not injected into system prompt |
| 3 | **`is_mostly_passing()` check** | Not used to decide between targeted fix vs full replan |
| 4 | **Post-gate reflection loop** | Full gap — not implemented at all |
| 5 | **Context injection scoping** | Full gap — KnowledgeConfig, role-filtered context |
| 6 | **Warm agent spawning** | Full gap — WarmPool not implemented |
| 7 | **Configurable watcher thresholds** | Watchers exist but thresholds may be hardcoded; verify configurability |
| 8 | **Neuro store → cascade router** | Full gap — router doesn't consult knowledge store |
| 9 | **Episode clustering** | Full gap — no clustering in pattern_discovery.rs |
| 10 | **Provider pass-rate bias** | Full gap — provider metrics not multiplied into model scores |
| 11 | **Reflection-derived playbook rules** | Full gap — no auto-generation from reflections |
| 12 | **A-MAC admission gate** | Full gap — no 5-factor validation |

---

## Spec clarifications (resolving ambiguities)

> Added 2026-04-24. These resolve gaps identified during architecture audit.

### Gap 1: Parsing fallback chain

The spec says "Try JSON first, then JSON code block, then TOML block." Full algorithm:

```rust
fn parse_review(output: &str) -> StructuredReview {
    // 1. Try parsing entire output as JSON
    if let Ok(review) = serde_json::from_str::<StructuredReview>(output) {
        return review;
    }
    // 2. Try extracting JSON from ```json ... ``` code block
    if let Some(json_block) = extract_code_block(output, "json") {
        if let Ok(review) = serde_json::from_str::<StructuredReview>(&json_block) {
            return review;
        }
    }
    // 3. Try extracting TOML from ```toml ... ``` code block
    if let Some(toml_block) = extract_code_block(output, "toml") {
        if let Ok(review) = toml::from_str::<StructuredReview>(&toml_block) {
            return review;
        }
    }
    // 4. Fallback: treat entire output as a Revise verdict with raw text
    StructuredReview {
        verdict: ReviewDecision::Revise,
        issues: vec![],
        summary: output.chars().take(500).collect(),  // cap at 500 chars
    }
}
```

The fallback (step 4) means **parsing never fails** — worst case, the raw text becomes the summary and the orchestrator treats it as a revision request.

### Gap 1: `is_quick_fixable()` categories

Quick-fixable categories: `Compilation`, `Docs`, `Style`, `LintViolation`, `Unused`.

NOT quick-fixable (even if they seem small): `TestFailure` (tests can reveal deeper bugs), `TypeMismatch` (may require API changes), `SpecDeviation` (needs design discussion), `SecurityVulnerability` (needs careful review).

Rationale: "quick-fixable" means "an implementer agent can fix this without strategic planning." Compilation errors have deterministic fixes (add import, fix syntax). Style/docs/lint are mechanical. Tests and types can cascade.

### Gap 2: `cargo fix` merge conflict handling

If `cargo fix --allow-dirty` modifies a file that was also edited by a previous agent in the same task:
1. Run `cargo fix --allow-dirty` on the worktree
2. If it succeeds, run `cargo fmt`
3. If `cargo fix` exits non-zero (e.g., conflicting suggestions), skip auto-fix and fall through to agent-assisted fix
4. Never run `cargo fix` with `--allow-staged` (could corrupt staging area)

The key insight: `cargo fix` operates on the working tree. If the previous agent's changes are already in the working tree, `cargo fix` applies on top of them. Conflicts are rare because `cargo fix` only applies compiler suggestions, which don't overlap with semantic changes.

### Gap 3: Error deduplication algorithm

Error patterns are deduplicated by **normalized error code + file path** (not full error text):

```rust
fn error_key(error: &CompileError) -> String {
    format!("{}::{}", error.code, error.file.as_deref().unwrap_or("unknown"))
}
```

Two `error[E0425]` in different files ARE different patterns (they need different fixes). Two `error[E0425]` in the same file with different line numbers are the SAME pattern (likely the same root cause).

### Gap 4: Reflection cost guard with variable pricing

The "$0.02 max" cost guard assumes Haiku pricing. The actual guard is:

```rust
let max_reflection_tokens = 500;  // output tokens
let model = "claude-haiku-4-5-20251001";  // always use cheapest model
// At Haiku pricing ($0.25/M output), 500 tokens = $0.000125
// The $0.02 cap is generous — actual cost is ~$0.0001
```

The guard is not price-based, it's **token-based**: max_tokens=500 on the cheapest available model. This works regardless of provider pricing changes.

### Gap 5: Context size numbers for role filtering

| Role | File intel entries | Warning entries | Error pattern entries |
|------|-------------------|-----------------|----------------------|
| Implementer | 10 (full: file path, key functions, recent changes) | 5 | 5 |
| Reviewer | 3 (summary: file path + one-line description) | 3 | 3 |
| Strategist | 0 (sees plan-level only) | 0 | 0 |

These are defaults from `KnowledgeConfig`. All configurable via roko.toml:

```toml
[knowledge]
file_intel_max_entries = 10
warnings_max_entries = 5
error_pattern_min_cluster = 3
```

### Gap 7: Conductor watcher threshold config

Watcher thresholds are configurable in roko.toml under `[conductor]`:

```toml
[conductor]
ghost_turn_max_secs = 5
review_loop_max_consecutive = 3
iteration_loop_max = 6
test_failure_budget_pass_rate = 0.70
silence_timeout_secs = 180
compile_fail_max_consecutive = 3
task_stall_secs = 300
context_pressure_percent = 80
phase_timeout_secs = 1800
cooldown_filter_secs = 120
```

If a key is missing, the hardcoded default from the watchers table applies.

### Gap 8: Cascade router knowledge bias clamping

Model scores in the cascade router range from 0.0 to 1.0. The knowledge bias is additive:

```
final_score = base_score + knowledge_bias
knowledge_bias ∈ [-0.1, +0.1]
final_score = clamp(final_score, 0.05, 1.0)  // never zero (always possible), cap at 1.0
```

The 0.05 floor ensures every model has a non-zero chance of selection (exploration).

### Gap 9: Episode clustering with fewer than 3 matches

Clusters with fewer than 3 episodes are **not discarded** — they're stored as `immature` clusters:

```rust
pub struct EpisodeCluster {
    pub key: String,
    pub count: usize,
    pub maturity: ClusterMaturity,  // Immature (< 3) | Mature (>= 3)
    pub recommended_model: Option<String>,  // None for immature
    // ...
}
```

Immature clusters don't produce model recommendations. They become mature when they accumulate 3+ episodes. This prevents premature conclusions from small samples.

### Gap 12: A-MAC contradiction detection

Contradiction is detected via **cosine distance inversion**, not a separate "semantic opposition" model:

```rust
fn check_contradiction(new_entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> bool {
    for entry in existing.iter().filter(|e| e.confidence > 0.8) {
        let sim = cosine_similarity(&new_entry.hdc_vector, &entry.hdc_vector);
        // High similarity but opposite conclusion = contradiction
        // Measured by: topic vectors are similar (sim > 0.7) but
        // the assertion differs (new claims opposite of existing)
        if sim > 0.7 {
            let assertion_sim = cosine_similarity(
                &new_entry.assertion_vector,
                &entry.assertion_vector,
            );
            if assertion_sim < -0.3 {
                return true;  // Contradiction: same topic, opposite claim
            }
        }
    }
    false
}
```

The key insight: entries are encoded with two HDC vectors — one for the **topic** (what it's about) and one for the **assertion** (what it claims). High topic similarity + negative assertion similarity = contradiction.

If HDC vectors are not available (e.g., pre-HDC entries), fall back to keyword overlap for topic similarity and skip contradiction checking (return false).
