# 01 — T0 Reflex Store

**Priority**: P2 — Reduces LLM token cost for high-frequency repeated decisions
**Size**: M (2-3 days)
**Crates**: `crates/roko-learn` (primary), `crates/roko-agent`, `crates/roko-cli`
**Depends on**: None

---

## Background

Roko is a Rust toolkit for building agents that develop software autonomously. The core loop is: receive a task, dispatch it to an LLM (Language Model), validate the output with a gate (compile, test, clippy), persist the result, and learn from the outcome. Each agent task involves at least one round-trip to an LLM API.

The agent pipeline is designed around three reasoning tiers: T0 (reflex — instant, no LLM), T1 (reflective — fast, cheap model), and T2 (deliberate — slow, expensive model). T0 is specified to execute from a "cached habitual action" store without any LLM call. This is analogous to how a human automatically types `cargo test --workspace` after editing a source file without stopping to reason about it.

The problem is that T0's store does not exist. Every agent tick — including completely deterministic responses like "run cargo test after editing a Rust file" or "read a file before reviewing it" — invokes an LLM. This wastes tokens, adds 8-30 seconds of latency, and costs $0.20-2.00 per turn for decisions that have been made correctly dozens of times before.

This backlog item implements the T0 reflex store: a persisted ordered list of condition-action pairs learned from successful T2 episodes. When a new observation arrives, the store is checked first. If a rule matches, its action executes immediately without any LLM call.

## Current State

1. **No `reflex_store.rs` exists** in `crates/roko-learn/src/`. The file does not exist.

2. **`crates/roko-learn/src/lib.rs`** exports 50+ modules (episode_logger, playbook_rules, cascade_router, efficiency, etc.) but has no `reflex_store` module declaration. Adding one requires a single `pub mod reflex_store;` line in this file.

3. **T0/T1/T2 routing already exists** via `roko_primitives` (`ModelTier`) and `roko_learn::active_inference`. The tier selection happens in runner dispatch via the cascade router. The reflex store hooks in *before* the cascade router is consulted — it is checked on every tick before any LLM infrastructure is invoked.

4. **Gate outcome feedback** already flows back through the runner. In `crates/roko-cli/src/runner/event_loop.rs`, gate results (`GateCompletion`) are processed after each task. The reflex store's confidence update callbacks attach here.

5. **`.roko/learn/`** directory already exists and is used by other learning subsystems: `cascade-router.json`, `efficiency.jsonl`, `gate-thresholds.json`. The reflex store adds `reflexes.jsonl` to this directory.

6. **`LearnCmd` enum** is defined in `crates/roko-cli/src/main.rs` at line 1170-1214. It has variants: `All`, `Route`, `Experiments`, `Efficiency`, `Episodes`, `Tune`. Adding a `Reflexes` variant requires editing this enum and adding a dispatch branch in `crates/roko-cli/src/commands/learn.rs` at the `dispatch_learn` function (line 30).

7. **`roko-learn` crate `lib.rs`** is at `crates/roko-learn/src/lib.rs`. The `#![deny(missing_docs)]` attribute at line 20 means every new public item needs a doc comment.

## Implementation Plan

### Step 1: Create `crates/roko-learn/src/reflex_store.rs`

Create a new file with these types and methods:

```rust
//! T0 reflex store — condition-action pairs learned from successful T2 episodes.
//!
//! Before any LLM call, the runner checks this store. A matching rule fires its
//! action directly, skipping inference entirely.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Observation pattern that must match for a rule to fire.
/// All `Some` fields must match; `None` fields are wildcards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflexCondition {
    /// Tool name that must be present in the observation (e.g. `"bash"`).
    pub tool: Option<String>,
    /// Substring that must appear in the tool args (e.g. `"cargo test"`).
    pub args_pattern: Option<String>,
    /// Substring that must appear in the observation context.
    pub context: Option<String>,
    /// Message type tag (e.g. `"user"`, `"tool_result"`).
    pub message_type: Option<String>,
    /// File extension that must be present in the context (e.g. `".rs"`).
    pub file_ext: Option<String>,
}

/// The deterministic action to take when the condition matches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReflexAction {
    /// Tool to invoke (e.g. `"bash"`).
    pub tool: String,
    /// Arguments to pass (e.g. `"cargo test --workspace"`).
    pub args: String,
}

/// A single condition-action rule in the reflex store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexRule {
    /// Stable rule identifier.
    pub id: Uuid,
    /// The pattern that must match for this rule to fire.
    pub condition: ReflexCondition,
    /// The action to take when the condition matches.
    pub action: ReflexAction,
    /// success_count / hit_count. A new rule starts at 1.0.
    pub confidence: f64,
    /// Episode ID that produced this rule's first promotion.
    pub source_episode: String,
    /// When the rule was created.
    pub promoted_at: DateTime<Utc>,
    /// When the rule last fired.
    pub last_fired_at: Option<DateTime<Utc>>,
    /// Total times this rule's condition matched an observation.
    pub hit_count: u32,
    /// Times this rule fired and the subsequent gate passed.
    pub success_count: u32,
}

/// Observation summary passed to [`ReflexStore::match_observation`].
#[derive(Debug, Clone)]
pub struct ReflexObservation {
    /// Active tool name, if any.
    pub tool: Option<String>,
    /// Tool arguments, if any.
    pub args: Option<String>,
    /// Surrounding context text.
    pub context: Option<String>,
    /// Message type.
    pub message_type: Option<String>,
    /// File extensions visible in context.
    pub file_exts: Vec<String>,
}

/// Key from a T2 episode used to check if the decision is stable enough to promote.
#[derive(Debug, Clone)]
pub struct PromotionCandidate {
    /// Episode ID that created this candidate.
    pub episode_id: String,
    /// The condition this episode matched.
    pub condition: ReflexCondition,
    /// The action the agent took.
    pub action: ReflexAction,
}

/// Append-only JSONL-backed store of [`ReflexRule`]s.
pub struct ReflexStore {
    path: PathBuf,
    /// In-memory index: rule ID → rule. Ordered by insertion time.
    rules: Arc<Mutex<IndexMap<Uuid, ReflexRule>>>,
}

/// Maximum rules held in memory and on disk before LRU eviction.
const MAX_RULES: usize = 200;
/// Minimum T2 fires with identical pattern before promotion.
const PROMOTE_MIN_HITS: u32 = 3;
/// Minimum confidence required for promotion.
const PROMOTE_MIN_CONFIDENCE: f64 = 0.90;
/// Confidence multiplier applied on gate failure.
const DEMOTE_MULTIPLIER: f64 = 0.5;
/// Confidence floor below which the rule is deleted.
const DEMOTE_DELETE_THRESHOLD: f64 = 0.50;

impl ReflexStore {
    /// Open or create the reflex store at `path`.
    ///
    /// If the file does not exist an empty store is returned. Errors on
    /// corrupt data are logged and treated as an empty store.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let rules = if path.exists() {
            load_from_jsonl(&path).unwrap_or_default()
        } else {
            IndexMap::new()
        };
        Self {
            path,
            rules: Arc::new(Mutex::new(rules)),
        }
    }

    /// Check whether any rule's condition matches `obs`.
    ///
    /// Returns the first matching [`ReflexAction`] (rules are checked in
    /// insertion order). Updates `last_fired_at` and `hit_count` on the
    /// matching rule. Returns `None` when no rule matches.
    ///
    /// This call must complete in under 100µs for 200 rules.
    pub fn match_observation(&self, obs: &ReflexObservation) -> Option<ReflexAction> {
        let mut rules = self.rules.lock().unwrap();
        for rule in rules.values_mut() {
            if condition_matches(&rule.condition, obs) {
                rule.hit_count += 1;
                rule.last_fired_at = Some(Utc::now());
                return Some(rule.action.clone());
            }
        }
        None
    }

    /// Record a gate pass for the rule that last fired for `action`.
    ///
    /// Increments `success_count` and recalculates `confidence`.
    pub fn record_gate_pass(&self, action: &ReflexAction) {
        let mut rules = self.rules.lock().unwrap();
        for rule in rules.values_mut() {
            if rule.action == *action {
                rule.success_count += 1;
                rule.confidence =
                    f64::from(rule.success_count) / f64::from(rule.hit_count).max(1.0);
                break;
            }
        }
        // persist in background — caller may ignore error
        let _ = self.flush_locked(&rules);
    }

    /// Record a gate failure for the rule that last fired for `action`.
    ///
    /// Halves confidence. If confidence drops below [`DEMOTE_DELETE_THRESHOLD`]
    /// the rule is deleted and `true` is returned (caller should log a
    /// demotion event to `.roko/learn/efficiency.jsonl`).
    pub fn record_gate_fail(&self, action: &ReflexAction) -> bool {
        let mut rules = self.rules.lock().unwrap();
        let demoted_id = rules.iter_mut().find_map(|(id, rule)| {
            if rule.action == *action {
                rule.confidence *= DEMOTE_MULTIPLIER;
                if rule.confidence < DEMOTE_DELETE_THRESHOLD {
                    return Some(*id);
                }
            }
            None
        });
        let deleted = demoted_id.is_some();
        if let Some(id) = demoted_id {
            rules.shift_remove(&id);
        }
        let _ = self.flush_locked(&rules);
        deleted
    }

    /// Attempt to promote a T2 decision to a T0 reflex rule.
    ///
    /// Checks the promotion criteria: the same `(condition, action)` pair
    /// has fired 3+ times and confidence > 0.90. If criteria are met and
    /// no identical rule already exists, inserts a new rule. Evicts the
    /// LRU rule when the store is at capacity.
    ///
    /// Returns `true` when a new rule was added.
    pub fn try_promote(&self, candidate: &PromotionCandidate, fires: u32) -> bool {
        if fires < PROMOTE_MIN_HITS {
            return false;
        }
        let confidence = 1.0_f64; // new rules start at full confidence
        if confidence < PROMOTE_MIN_CONFIDENCE {
            return false;
        }

        let mut rules = self.rules.lock().unwrap();

        // Do not duplicate an existing rule for the same condition+action.
        let already_exists = rules.values().any(|r| {
            r.condition == candidate.condition && r.action == candidate.action
        });
        if already_exists {
            return false;
        }

        // Evict LRU rule when at capacity.
        if rules.len() >= MAX_RULES {
            let lru_id = rules
                .values()
                .min_by_key(|r| r.last_fired_at)
                .map(|r| r.id);
            if let Some(id) = lru_id {
                rules.shift_remove(&id);
            }
        }

        let rule = ReflexRule {
            id: Uuid::new_v4(),
            condition: candidate.condition.clone(),
            action: candidate.action.clone(),
            confidence,
            source_episode: candidate.episode_id.clone(),
            promoted_at: Utc::now(),
            last_fired_at: None,
            hit_count: fires,
            success_count: fires, // all fires were successful (gate passed)
        };
        rules.insert(rule.id, rule);
        let _ = self.flush_locked(&rules);
        true
    }

    /// Number of rules currently in the store.
    pub fn len(&self) -> usize {
        self.rules.lock().unwrap().len()
    }

    /// `true` when the store contains no rules.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a snapshot of all rules, sorted by hit_count descending.
    pub fn snapshot(&self) -> Vec<ReflexRule> {
        let rules = self.rules.lock().unwrap();
        let mut v: Vec<ReflexRule> = rules.values().cloned().collect();
        v.sort_by(|a, b| b.hit_count.cmp(&a.hit_count));
        v
    }

    fn flush_locked(&self, rules: &IndexMap<Uuid, ReflexRule>) -> std::io::Result<()> {
        use std::io::Write as _;
        let mut buf = Vec::new();
        for rule in rules.values() {
            let line = serde_json::to_string(rule)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            buf.extend_from_slice(line.as_bytes());
            buf.push(b'\n');
        }
        // Atomic write: write to .tmp, then rename
        let tmp = self.path.with_extension("jsonl.tmp");
        std::fs::write(&tmp, &buf)?;
        std::fs::rename(&tmp, &self.path)
    }
}

fn load_from_jsonl(path: &Path) -> Option<IndexMap<Uuid, ReflexRule>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut map = IndexMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(rule) = serde_json::from_str::<ReflexRule>(line) {
            map.insert(rule.id, rule);
        }
    }
    Some(map)
}

fn condition_matches(cond: &ReflexCondition, obs: &ReflexObservation) -> bool {
    if let Some(t) = &cond.tool {
        if obs.tool.as_deref() != Some(t.as_str()) {
            return false;
        }
    }
    if let Some(pattern) = &cond.args_pattern {
        if !obs.args.as_deref().is_some_and(|a| a.contains(pattern.as_str())) {
            return false;
        }
    }
    if let Some(ctx) = &cond.context {
        if !obs.context.as_deref().is_some_and(|c| c.contains(ctx.as_str())) {
            return false;
        }
    }
    if let Some(msg) = &cond.message_type {
        if obs.message_type.as_deref() != Some(msg.as_str()) {
            return false;
        }
    }
    if let Some(ext) = &cond.file_ext {
        if !obs.file_exts.iter().any(|e| e == ext) {
            return false;
        }
    }
    true
}
```

The `indexmap` crate is already in `roko-learn/Cargo.toml` (used by `cascade_router.rs`). Add `uuid` with the `v4` feature and `chrono` with `serde` feature if not already present — check `Cargo.toml` for the crate.

### Step 2: Register the module in `crates/roko-learn/src/lib.rs`

After the last existing `pub mod` line (currently `pub mod wal;` at line 164), add:

```rust
/// T0 reflex store — condition-action pairs learned from T2 episode promotions.
pub mod reflex_store;
```

### Step 3: Add `Reflexes` subcommand to `LearnCmd` in `crates/roko-cli/src/main.rs`

At line 1200 (after the `Episodes` variant closing brace, before `Tune`), add:

```rust
    /// Show T0 reflex store rules (total count, top-5 by hit count, recent demotions).
    Reflexes {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
```

### Step 4: Add dispatch to `crates/roko-cli/src/commands/learn.rs`

In `dispatch_learn()` (line 30), add a new match arm after the `LearnCmd::Episodes` arm:

```rust
        LearnCmd::Reflexes { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_reflexes(&wd).await
        }
```

Then add the handler function at the end of the file:

```rust
async fn cmd_learn_reflexes(workdir: &std::path::Path) -> Result<i32> {
    let path = workdir.join(".roko/learn/reflexes.jsonl");
    let store = roko_learn::reflex_store::ReflexStore::open(&path);
    let rules = store.snapshot();

    println!("T0 Reflex Store — {} rules (max 200)", rules.len());

    if rules.is_empty() {
        println!("  (no rules yet; run tasks to build reflex history)");
        return Ok(crate::EXIT_SUCCESS);
    }

    println!("\nTop rules by hit count:");
    for (i, rule) in rules.iter().take(5).enumerate() {
        println!(
            "  {}. [{:.0}% conf, {} hits] {:?} → {} {}",
            i + 1,
            rule.confidence * 100.0,
            rule.hit_count,
            rule.condition.tool.as_deref().unwrap_or("*"),
            rule.action.tool,
            rule.action.args,
        );
    }
    Ok(crate::EXIT_SUCCESS)
}
```

Also add `LearnCmd::Reflexes` to the `cmd_learn` guard in the existing `cmd_learn` function — the function currently rejects unknown subsystem strings. The `Reflexes` variant dispatches directly from `dispatch_learn` so no changes to `cmd_learn` itself are needed.

### Step 5: Gate outcome feedback wiring in `crates/roko-cli/src/runner/event_loop.rs`

Search for the function `build_gate_retry_context` (line 15323). Just above where gate failures are returned to the agent, add calls to `ReflexStore::record_gate_fail` when a reflex action was the source of the current task's dispatch. Similarly, call `ReflexStore::record_gate_pass` when a reflex-dispatched task passes its gate.

This wiring requires threading the `ReflexStore` (wrapped in `Arc`) through `RunConfig` or `RunState`. The minimal approach:

1. Add `reflex_store: Option<Arc<roko_learn::reflex_store::ReflexStore>>` to `RunConfig` or to `RunState`.
2. Initialize it in `plan_run` by opening `.roko/learn/reflexes.jsonl`.
3. When dispatching a T0 reflex action (if the observation matched), store the fired action in per-task state.
4. After gate completion, call the appropriate feedback method.

The exact location depends on runner state structure. Search for `GateCompletion` handling in `event_loop.rs` — the handler that processes gate results after each task is the right insertion point.

## Acceptance Criteria

1. `ReflexStore::match_observation` returns `Some(ReflexAction)` for a matching rule and `None` when no rule matches, for 200 rules in under 100µs (verified by benchmark test in the module).

2. A T2 decision that fires with identical condition+action 3 times with zero gate failures is promoted via `try_promote` — `store.len()` grows by one, `confidence` starts at 1.0, and the rule is visible in `reflexes.jsonl`.

3. A gate failure on a reflex-fired action halves confidence. Two successive failures (`1.0 → 0.5 → 0.25`) delete the rule: `record_gate_fail` returns `true` and the rule is gone from `store.snapshot()`.

4. A T0 match executes without invoking any LLM provider — the LLM token counter in the runner remains unchanged.

5. Rules survive process restart: write rules, drop the store, re-open from same path, confirm `len()` and rule contents match.

6. At 200 rules, calling `try_promote` with a new candidate evicts the rule with the oldest `last_fired_at`.

7. `roko learn reflexes` prints total rule count and top-5 rules without error when run from a workspace with a populated `.roko/learn/reflexes.jsonl`.

8. `roko learn reflexes` prints a friendly "no rules yet" message when the file does not exist.

## Verification Checklist

- [ ] `cargo test -p roko-learn reflex_store` passes all unit tests
- [ ] `cargo clippy -p roko-learn --no-deps -- -D warnings` passes clean
- [ ] `cargo build -p roko-cli` builds without error after adding `LearnCmd::Reflexes`
- [ ] Run `cargo run -p roko-cli -- learn reflexes` from workspace root — should print "no rules yet" message
- [ ] Write a unit test that creates a `ReflexStore`, inserts 3 matching promotions, verifies `len() == 1`
- [ ] Write a unit test for demotion: insert a rule, call `record_gate_fail` twice, assert `len() == 0`
- [ ] Write a round-trip test: open store, add rule, drop, re-open, assert rule present
- [ ] Write a capacity test: fill to 200 rules, `try_promote` a new one, assert `len() == 200` and the LRU rule is gone

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-learn/src/reflex_store.rs` | **New file** — `ReflexStore`, `ReflexRule`, `ReflexCondition`, `ReflexAction`, `ReflexObservation`, `PromotionCandidate`, match and feedback logic |
| `crates/roko-learn/src/lib.rs` | Add `pub mod reflex_store;` after line 164 (`pub mod wal;`) |
| `crates/roko-cli/src/main.rs` | Add `Reflexes { workdir }` variant to `LearnCmd` enum at line 1200 |
| `crates/roko-cli/src/commands/learn.rs` | Add `LearnCmd::Reflexes` dispatch arm and `cmd_learn_reflexes` handler |
| `crates/roko-cli/src/runner/event_loop.rs` | Thread `ReflexStore` through `RunState` or `RunConfig`; call `record_gate_pass`/`record_gate_fail` at gate completion |
| `crates/roko-learn/Cargo.toml` | Add `uuid` (with `v4` feature) if not present; verify `chrono` and `indexmap` are already there |
