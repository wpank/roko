# T0 Reflex Store

**Priority**: P2
**Size**: M (2-3 days)

---

## Problem

Every agent tick in roko requires a round-trip to an LLM provider, even for decisions the
agent has made correctly dozens of times before — running `cargo test --workspace` after a
source edit, reading a file before reviewing it, checking `git status` before a commit.
These are deterministic responses to observed patterns. Routing them through a model wastes
tokens, adds latency, and burns budget that could go to genuinely uncertain decisions.

The agent pipeline already defines three reasoning tiers — T0 (reflex), T1 (reflective),
T2 (deliberate) — with T0 specified to skip LLM inference entirely and execute from a
"cached/habitual action" store. That store does not exist yet. No types for it exist in
the codebase. The promotion of stable T2 decisions to zero-cost T0 reflex rules is the
missing implementation.

---

## Solution

A **reflex store**: a persisted, ordered list of condition-action pairs learned from
successful T2 episodes. On each agent tick, before any LLM call, the runtime checks
whether the current observation matches a reflex rule. If it matches, the rule's action
is executed directly. If not, the tick escalates to T1 or T2 as normal.

### Key types (all new, in `crates/roko-learn/src/reflex_store.rs`)

```rust
/// A single condition-action pair held in the reflex store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexRule {
    pub id: Uuid,
    pub condition: ReflexCondition,
    pub action: ReflexAction,
    pub confidence: f64,            // success_count / hit_count
    pub source_episode: String,     // episode ID that created this rule
    pub promoted_at: DateTime<Utc>,
    pub last_fired_at: Option<DateTime<Utc>>,
    pub hit_count: u32,
    pub success_count: u32,
}

/// Observation pattern that must match for the rule to fire.
/// All non-None fields must match; absent fields are wildcards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexCondition {
    pub tool: Option<String>,
    pub args_pattern: Option<String>,   // substring match
    pub context: Option<String>,
    pub message_type: Option<String>,
    pub file_ext: Option<String>,
}

/// The deterministic action to execute when the condition matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexAction {
    pub tool: String,
    pub args: String,
}

/// Load/save handle for the reflex store.
pub struct ReflexStore {
    path: PathBuf,
    rules: Arc<Mutex<IndexMap<Uuid, ReflexRule>>>,
}
```

### Promotion lifecycle

A T2 decision is promoted to T0 when:
1. The same observation pattern has triggered the same action **3+ times**.
2. **Every** execution passed its gate (zero gate failures).
3. Computed confidence (`success_count / total_count`) is **> 0.90**.

Promotion is evaluated at the end of each T2 tick. `ReflexStore::try_promote()` atomically
checks criteria and appends a new rule if satisfied.

### Demotion lifecycle

- **Gate pass on reflex-fired action**: `success_count += 1`, confidence recalculated.
- **Gate fail on reflex-fired action**: `confidence *= 0.5`. If confidence < 0.50, delete
  the rule and log a demotion event to `.roko/learn/efficiency.jsonl`.

### Storage

| Property | Value |
|---|---|
| Path | `.roko/learn/reflexes.jsonl` |
| Format | Append-only JSONL, one `ReflexRule` per line |
| Max rules | 200 |
| Eviction | LRU on `last_fired_at` |
| Concurrency | `Arc<Mutex<IndexMap>>` |

### Execution flow

```
Observation arrives → Gate step computes prediction_error (PE)
  PE < 0.15 → T0 path:
    ReflexStore::match_observation(&observation)
      match found → execute action (no LLM), record outcome
      no match    → escalate to T1
  PE 0.15-0.40 → T1 path (Haiku-class)
  PE > 0.40    → T2 path (Sonnet/Opus)
    → step 9 Reflect → ReflexStore::try_promote()
```

---

## Where to implement

| Component | Path |
|---|---|
| Types + store + match + promote + demote | `crates/roko-learn/src/reflex_store.rs` (new) |
| Module export | `crates/roko-learn/src/lib.rs` (add `pub mod reflex_store`) |
| Agent tick integration | `crates/roko-agent/src/lifecycle.rs` (check reflex before LLM) |
| Gate outcome callback | `crates/roko-cli/src/runner/event_loop.rs` (feed gate results back) |
| CLI subcommand | `crates/roko-cli/src/commands/learn.rs` (`roko learn reflexes`) |

### Prerequisites

- The T0/T1/T2 gate discriminator function (E23 introduced `ModelTier` and EFE routing,
  but the three-level `GatingTier` callable at tick time needs to be standalone or created
  as part of this work).
- Gate outcome callback from runner to agent reflex confidence updater.
- `.roko/learn/` directory (already exists, used by cascade router and efficiency log).

---

## Acceptance criteria

1. `ReflexStore::match_observation` returns `Some(ReflexAction)` for a matching rule and
   `None` when no rule matches, in under 100µs for 200 rules.
2. A T2 decision that fires successfully 3 times with identical pattern and zero gate
   failures is automatically promoted — `rules().len()` grows by one, confidence >= 0.90,
   rule is persisted to `reflexes.jsonl`.
3. A gate fail on a reflex-fired action halves confidence. Two halves (1.0 → 0.5 → 0.25)
   deletes the rule from memory and disk.
4. A T0 match executes without invoking any LLM provider — provider token counter stays
   at zero.
5. Rules survive agent restart (round-trip serialization test).
6. At 200 rules, a new promotion evicts the rule with the oldest `last_fired_at`.
7. Demotion events appear in `.roko/learn/efficiency.jsonl` and `roko learn efficiency`.
8. `roko learn reflexes` prints: total rule count, top-5 by hit count, recent demotions.
