# 08-learning/02 -- Playbook Store

> When/then rules with globset triggers, bounded confidence dynamics,
> demurrage-governed freshness, and GRASP-style regression-gated admission.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/playbook.rs`,
`crates/roko-learn/src/playbook_rules.rs`

**Persistence:** `.roko/learn/playbooks/` (JSON per playbook),
`.roko/learn/playbook-rules.toml`

**Cross-references:** [episode-logger](episode-logger.md),
[skill-library-voyager](skill-library-voyager.md),
[pattern-discovery-trigram](pattern-discovery-trigram.md),
[regression-detection](regression-detection.md)

---

## 1. Purpose

The playbook system is the concrete procedural projection of Roko's learning
stack. Where episodes record raw observations and patterns identify recurring
structure, playbook rules capture validated predictions that are injected
directly into agent prompts to prevent known failure modes. When a rule
correctly predicts outcomes across multiple subsequent executions, it earns
enough reinforcement to stay warm and gets injected at dispatch time. Freshness
is not governed by confidence alone: demurrage, successful reuse, and
contradiction-driven penalties decide whether a rule remains active or cools
into cold storage.

The system has two components:

1. **PlaybookStore** -- manages named sequences of steps (playbooks) with
   success/failure counters and freshness balance.
2. **PlaybookRules** -- manages if-then rules with globset-based triggers,
   bounded confidence dynamics, and demurrage-driven reinforcement.

---

## 2. Playbooks in the Learning Stack

```
+--------------------------------------------------------------+
|              Tier 4: Playbook Rules And Playbooks              |
|   Concrete instructions compiled from validated heuristics.    |
|   Confidence: 0.0 -- 0.95 bounded. Reinforcement + balance    |
|   keep rules warm; demurrage cools stale rules.               |
+--------------------------------------------------------------+
|             Tier 3: Heuristics And Worldview Priors            |
|   Reusable rules of thumb with falsifier surfaces.             |
+--------------------------------------------------------------+
|                    Tier 2: Patterns                             |
|   Extracted hypotheses from episode clustering.                 |
+--------------------------------------------------------------+
|                    Tier 1: Episodes                             |
|   Raw observations from every agent turn.                      |
+--------------------------------------------------------------+
```

---

## 3. PlaybookStore

The `PlaybookStore` manages named playbooks -- ordered sequences of steps that
describe a known-good approach to a task type.

### 3.1 Playbook Schema

```rust
pub struct Playbook {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub steps: Vec<PlaybookStep>,
    pub balance: f64,
    pub demurrage_paid: f64,
    pub success_count: u32,
    pub failure_count: u32,
}

pub struct PlaybookStep {
    pub index: u32,
    pub description: String,
    pub action_kind: String,
    pub expected_signals: Vec<String>,
}
```

### 3.2 Persistence

Each playbook is stored as a separate JSON file in `.roko/learn/playbooks/`,
keyed by playbook ID. The store uses per-ID async mutexes for concurrent
safety: multiple playbooks can be updated simultaneously, but updates to the
same playbook are serialized. Writes use the atomic tempfile+rename pattern.

```
.roko/learn/playbooks/
+-- pb-001.json    <- "Rust trait implementation" playbook
+-- pb-002.json    <- "Config schema extension" playbook
+-- pb-003.json    <- "Gate failure recovery" playbook
```

### 3.3 Operations

| Method | What it does |
|--------|-------------|
| `PlaybookStore::save(playbook)` | Persist a new or updated playbook |
| `PlaybookStore::load(id)` | Load a single playbook by ID |
| `PlaybookStore::load_all()` | Load all playbooks from the directory |
| `PlaybookStore::record_outcome(id, success)` | Increment success or failure counter |

---

## 4. PlaybookRules

The `PlaybookRules` module manages if-then rules with rich trigger matching and
bounded confidence dynamics. Rules are the actionable output of the learning
system -- they are injected into agent prompts to prevent known failure modes,
and they self-trim through demurrage.

### 4.1 Rule Schema

```rust
pub struct Rule {
    pub rule_id: String,
    pub title: String,         // <= 80 chars
    pub body: String,          // injected into prompt
    pub triggers: Triggers,
    pub balance: f64,          // freshness balance (demurrage-governed)
    pub demurrage_paid: f64,
    pub confidence: f64,       // bounded to [0.0, 0.95]
    pub validations: u32,
    pub contradictions: u32,
    pub last_applied: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub source_episodes: Vec<String>,
}
```

### 4.2 Trigger System

Rules fire when incoming context matches their `Triggers`:

```rust
pub struct Triggers {
    pub file_globs: Vec<String>,        // shell glob patterns
    pub tags: Vec<String>,              // case-insensitive overlap
    pub categories: Vec<String>,        // task categories
    pub error_signatures: Vec<String>,  // error signature strings
    pub roles: Vec<String>,             // agent roles
}
```

Matching uses **OR semantics** across the five trigger kinds: a rule fires if
ANY of its trigger lists intersects the incoming context. An all-empty
`Triggers` matches nothing -- it never fires, guarding against accidental
universal rules.

File glob matching uses the `globset` crate for shell-style pattern matching:

```toml
[[rule]]
rule_id = "rule-008"
title = "Auth module lifetime check"
body = "Check lifetime parameters on all auth types. Use get_symbol_context."
confidence = 0.92
validations = 12
contradictions = 1

[rule.triggers]
file_globs = ["src/auth/**/*.rs", "crates/roko-agent/src/auth/*"]
tags = ["lifetime", "borrow"]
categories = ["refactor", "bugfix"]
roles = ["Implementer"]
```

### 4.3 Matching Context

When composing a prompt, the system constructs a `MatchContext` from the
current task:

```rust
pub struct MatchContext {
    pub files: Vec<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub error_signature: Option<String>,
    pub role: Option<String>,
}
```

`PlaybookRules::select(context)` returns all rules whose triggers match the
context, sorted by confidence (highest first). The prompt composer injects the
top-N rules (typically 3-5) into the agent's system prompt as "lessons from
previous builds."

---

## 5. Confidence Dynamics

Confidence is update-driven, not time-based, and freshness is governed by
balance rather than a hard retention window:

| Trigger | Confidence change | Balance change |
|---------|------------------|----------------|
| Validation (rule predicted correctly) | +0.05 | Reinforcement bonus |
| Contradiction (rule predicted incorrectly) | -0.10 | Loss plus cooling |
| Successful reuse / citation | N/A | Reinforcement bonus |
| Demurrage tick | N/A | Holding cost |
| Prune threshold | N/A | Below floor: removed |

The asymmetric update rate (contradictions penalize 2x more than validations
reward) ensures that rules which stop being accurate are quickly demoted.
Demurrage makes demotion continuous instead of relying on periodic cleanup.

```
Confidence lifecycle:
    new rule -> 0.50 (default)
        |
        +-- validated -> 0.55 -> 0.60 -> ... -> 0.95 (ceiling)
        |
        +-- contradicted -> 0.40 -> 0.30 -> ... -> 0.0 (pruned)
```

### 5.1 Why 0.95 Ceiling?

The confidence ceiling prevents epistemic closure. A rule at 1.0 confidence
would never be questioned, even if the codebase changes in ways that invalidate
the rule's assumptions. The 0.95 ceiling means that every rule, no matter how
well-validated, retains a 5% "doubt margin" that allows contradictions to
eventually demote it.

---

## 6. Rule Lifecycle

```
Episode Stream
    |
    v
Pattern Discovery (trigram mining, HDC clustering)
    |
    v
Pattern extracted: "Auth module types have lifetime parameters"
    |
    +-- support_count < 5 -> stays as Pattern (Tier 2)
    |
    +-- support_count >= 5 -> promoted to Rule (Tier 3)
            |
            +-- Validated in subsequent builds -> confidence climbs
            |
            +-- Contradicted -> confidence drops
            |
            +-- confidence < min_confidence -> pruned (removed)
```

### 6.1 Demotion and Pruning

1. Each contradiction reduces confidence by 0.10 and cuts into balance.
2. Each demurrage tick reduces balance even when the rule is not contradicted.
3. Successful reuse or citation replenishes balance.
4. When confidence or balance drops below `min_confidence` / `min_balance`,
   the rule is pruned or moved to cold storage.
5. Pruned rules are removed from the TOML file on the next save.

This creates a self-cleaning knowledge base: rules that were valid for an older
version of the codebase but no longer apply are automatically removed as
contradictions accumulate and their balance drains.

---

## 7. GRASP-Style Regression-Gated Admission

> **Current state:** Playbook entries are admitted when a pattern has
> `support_count >= 5` and confidence exceeds `min_confidence`. There is no
> check for whether the new entry degrades performance on existing
> trajectories.

> **Target design:** Each candidate playbook entry is tested against a
> balanced held-out probe set under a hard regression budget before admission.
> Only admitted if net improvement is positive.

### 7.1 Rationale

GRASP (arXiv:2605.29668, May 2026) demonstrated that unrestricted grounding of
agent actions on retrieved knowledge causes silent degradation -- rules that
help on new cases can break established trajectories. On MedAgentBench, GRASP
improved task success from 40.6% to 88.8% (+48 points) by gating admission
through a regression test.

### 7.2 Admission Protocol (Target)

```
Candidate Rule R_new
    |
    v
1. Sample probe set P from recent successful episodes
   (stratified by role, complexity, crate)
    |
    v
2. Simulate: for each episode in P, would R_new have
   fired? If so, would its advice have changed the outcome?
    |
    v
3. Compute regression budget:
   improvements = count(P where R_new helps)
   regressions  = count(P where R_new hurts)
    |
    v
4. Gate: admit only if improvements > regressions + margin
   where margin = max(1, 0.1 * |P|)
    |
    v
5. If admitted, set initial confidence = 0.50
   If rejected, log reason to .roko/learn/rejected-rules.jsonl
```

### 7.3 Failed-Episode Augmentation (SiriuS)

Rather than discarding failed episodes outright, SiriuS (arXiv:2502.04780)
augments them with corrective annotations and reuses them as negative examples.
The target design preserves failed episodes with their error signatures and
uses them as part of the probe set -- a candidate rule that would have
*prevented* a known failure gets credit, while one that would have *caused* a
new failure in the probe set is penalized.

### 7.4 Unbounded Growth Prevention (SkillZip)

SkillZip (arXiv:2608.11079) compresses one skill's text by finding its shortest faithful structural explanation: a typed minimum-description-length (MDL) objective under a hard coverage constraint, run once or on every self-evolution patch (Zip-on-Write) (abstract, §V). It is evaluation-free and does not merge rules across a library. The target design borrows the MDL idea (Roko's own extension): when the playbook store exceeds a configured
capacity (default: 500 rules), the system compresses by merging rules with
overlapping triggers and high HDC similarity into a single generalized rule.
The MDL criterion ensures that generalization only happens when the merged rule
is shorter (in description length) than the two originals while preserving
predictive accuracy.

### 7.5 Complementary Work

ReSkill (arXiv:2606.01619) provides a complementary approach of skill
refinement through iterative self-correction, which could augment the GRASP
admission gate with post-admission refinement cycles.

---

## 8. Integration with Prompt Composition

When the prompt composer assembles a system prompt for an agent, it queries
the playbook rules:

```
Task spec (files, tags, category, role)
    |
    v
PlaybookRules::select(MatchContext)
    |
    v
Top-N matching rules (sorted by confidence)
    |
    v
Inject into system prompt as "Lessons from previous builds":
    "Note: past builds show that auth module types have
     lifetime parameters. Check actual signatures before
     using them. (confidence: 0.92, validated 12 times)"
```

The injected rules typically consume 50-100 tokens per rule. For a typical task
with 2-3 matching rules, this adds ~200 tokens -- a trivial cost that prevents
multi-thousand-token debugging loops.

---

## 9. Persistence Format

Playbook rules are stored in TOML for human readability:

```toml
min_confidence = 0.10
max_body_tokens = 200

[[rule]]
rule_id = "rule-001"
title = "Serde derive for config types"
body = "All types in roko-core::config that cross serialization boundaries need #[derive(Serialize, Deserialize)]."
confidence = 0.85
validations = 8
contradictions = 1
created_at = "2026-03-15T10:30:00Z"
source_episodes = ["ep-042", "ep-043", "ep-051", "ep-067", "ep-089"]

[rule.triggers]
file_globs = ["crates/roko-core/src/config/**/*.rs"]
tags = ["serde", "config"]
categories = ["bugfix"]
error_signatures = ["E0277.*Serialize"]
roles = ["Implementer"]
```

---

## 10. Cross-Project Transfer

Playbook rules are project-agnostic when their triggers use structural patterns
rather than project-specific identifiers. A rule triggered by
`error_signatures = ["E0277.*Serialize"]` applies to any Rust project.

Transfer workflow:

1. Export rules from project A.
2. Import into project B's `.roko/learn/playbook-rules.toml`.
3. Reset confidence to 0.50 and balance to a starter value (rules must
   re-earn both in the new context).
4. Rules that validate in project B climb; rules that contradict or go unused
   lose balance and are pruned.
