# 08-learning/03 -- Skill Library (Voyager-Style)

> Monotonically growing capability accumulation inspired by Wang et al. (2023),
> with prompt-template skills, gate-pipeline validation, and HDC retrieval.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/skill_library.rs`

**Persistence:** `.roko/learn/skills.json`

**Academic basis:** Wang, G., Xie, Y., Jiang, Y., Mandlekar, A., Xiao, C.,
Zhu, Y., Fan, L., & Anandkumar, A. (2023). *Voyager: An Open-Ended Embodied
Agent with Large Language Models.* arXiv:2305.16291.

**Cross-references:** [episode-logger](episode-logger.md),
[playbook-store](playbook-store.md),
[pattern-discovery-trigram](pattern-discovery-trigram.md)

---

## 1. Purpose

The skill library implements a Voyager-style capability accumulation system
(Wang et al. 2023). Where playbook rules capture defensive knowledge ("watch
out for X"), skills capture offensive knowledge ("here is how to do X"). Each
skill is a named, reusable capability with a prompt template, tool
dependencies, example I/O pairs, and usage telemetry that tracks how often the
skill succeeds when injected into agent prompts.

The Voyager insight is that agent systems should monotonically accumulate skills
from successful executions, building a growing library that makes future tasks
cheaper and more reliable. A crate that has been successfully modified 50 times
has accumulated patterns for trait implementation, test scaffolding, config
extension, and error handling -- patterns that a new agent can inherit rather
than rediscovering through trial and error.

---

## 2. Skill Schema

```rust
pub struct Skill {
    // -- Core identity --
    pub name: String,              // snake_case, human-readable
    pub summary: String,           // one-line description
    pub prompt_template: String,   // injected when selected
    pub required_tools: Vec<String>,
    pub example_inputs: Vec<String>,
    pub example_outputs: Vec<String>,
    pub tags: Vec<String>,

    // -- Usage telemetry --
    pub success_rate: f64,         // smoothed, [0.0, 1.0]
    pub usage_count: u64,

    // -- Voyager-style extraction fields --
    pub description: String,       // 1-2 sentences
    pub plan_id: String,           // originating plan
    pub files: Vec<String>,        // files touched
    pub pattern: String,           // numbered-step recipe (<= 750 chars)
    pub score: f64,                // eval score, [0.0, 1.0]
    pub first_seen: Option<DateTime<Utc>>,
    pub last_matched: Option<DateTime<Utc>>,
    pub match_count: u32,
    pub validated_count: u32,
    pub task_category: String,
}
```

### 2.1 Deduplication

Skills sharing >= 70% of their tags AND the same `task_category` are considered
duplicates. When a duplicate is detected during registration, the library keeps
the skill with the higher `score` and merges the usage telemetry from the
lower-scoring duplicate.

---

## 3. Voyager Architecture

The Voyager paper (Wang et al. 2023) describes a three-component system for
open-ended skill acquisition:

1. **Automatic Curriculum** -- proposes tasks of increasing complexity.
2. **Skill Library** -- stores and retrieves reusable code/procedures.
3. **Iterative Prompting** -- refines skills through feedback loops.

Roko implements an adapted version:

| Voyager Component | Roko Equivalent |
|-------------------|-----------------|
| Automatic curriculum | Plan generator (`roko plan generate`) creates tasks from a prompt or spec |
| Skill library | `SkillLibrary` in `roko-learn` with JSON persistence |
| Iterative prompting | Gate pipeline validates; failed attempts retry with context |
| Environment feedback | Gate verdicts (compile, test, lint, diff) |
| Code verification | 19-gate pipeline in `roko-gate` |

The key difference from Voyager (which operates in Minecraft) is that Roko's
environment is a real codebase with deterministic verification: the gate
pipeline provides ground-truth feedback that the skill either works or does
not. This makes confidence tracking more reliable than in open-ended
environments where success criteria are ambiguous.

---

## 4. Skill Extraction Pipeline

Skills are extracted from successful episodes:

```
Successful Episode (gate pass)
    |
    v
Analyze execution trace:
    +-- What files were touched, in what order?
    +-- What tools were used?
    +-- What prompt sections were most relevant?
    +-- What was the numbered-step recipe?
    |
    v
Construct Skill:
    +-- name: derived from task category + file pattern
    +-- prompt_template: generalized version of the successful prompt
    +-- required_tools: tools actually used during the episode
    +-- pattern: numbered-step recipe (<= 750 chars)
    +-- files: files touched
    +-- score: episode eval score
    +-- tags: derived from file paths, task category, error types
    |
    v
SkillLibrary::register(skill)
    +-- Check for duplicates (>= 70% tag overlap + same category)
    +-- If duplicate: keep higher-score skill, merge telemetry
    +-- If new: add to library, persist to skills.json
```

---

## 5. Skill Retrieval and Injection

When composing a prompt for a new task, the skill library is queried:

```
Task spec (files, category, tags)
    |
    v
SkillLibrary::search_by_tag(tags)
SkillLibrary::search_by_files(files)
    |
    v
Filter: success_rate >= 0.5, usage_count >= 2
    |
    v
Rank by: score * success_rate * recency_bonus
    |
    v
Top-3 skills injected into prompt as "Recommended approach":
    "Skill: rust_trait_implementation (confidence: 0.87)
     1. Read the existing trait definition with get_symbol_context
     2. Create the impl block in the target file
     3. Add #[cfg(test)] mod tests with at least one smoke test
     4. Run cargo test --lib to verify
     5. Run cargo clippy to check for lint warnings"
```

### 5.1 Validation Tracking

| Outcome | Update |
|---------|--------|
| Gate pass | `validated_count += 1`, `match_count += 1` |
| Gate fail | `match_count += 1` only |

The validation rate (`validated_count / match_count`) provides a direct measure
of skill utility. Skills frequently matched but rarely validated are candidates
for revision or removal.

---

## 6. Persistence and Thread Safety

The `SkillLibrary` is an in-memory `BTreeMap<String, Skill>` guarded by a
`parking_lot::RwLock`. Read operations (search, retrieve) acquire a shared read
lock. Write operations (register, record_use) acquire an exclusive write lock.

Persistence uses `tokio::fs` with the atomic tempfile+rename pattern:

```
1. Serialize library to JSON
2. Write to temporary file (skills.json.tmp)
3. fsync the temporary file
4. Rename skills.json.tmp -> skills.json (atomic on POSIX)
```

On startup, `SkillLibrary::new(path)` loads existing `skills.json` if present.
If the file does not exist, the library starts empty. If the file is corrupt,
the library fails with `SkillLibraryError::Serde`.

---

## 7. Monotonic Growth Property

The skill library is designed to grow monotonically: skills are added but never
removed in normal operation. This mirrors the Voyager insight that accumulated
knowledge should only increase over time. The only mechanisms that reduce the
library are:

1. **Deduplication** -- when a new skill duplicates an existing one, the
   lower-scoring duplicate is discarded.
2. **Manual pruning** -- an operator can edit `skills.json`.

There is no automatic pruning based on low usage or low success rate. The
rationale: a skill that hasn't been used recently may still be valuable when a
matching task appears. The cost of storing unused skills is negligible (a few
KB each), while the cost of re-extracting a pruned skill is significant.

---

## 8. Cross-Project Transfer

Skills that use structural patterns (trait implementation, test scaffolding)
rather than project-specific identifiers are transferable across codebases.

```
Project A skills.json -> export -> Project B skills.json
    |
    +-- Reset usage_count to 0
    +-- Reset success_rate to 0.5 (neutral prior)
    +-- Keep pattern, prompt_template, required_tools
    +-- Skills must re-earn confidence in project B
```

---

## 9. Relationship to Other Frameworks

| Framework | Skill Representation | Retrieval | Validation |
|-----------|---------------------|-----------|------------|
| Voyager (Wang et al. 2023) | JavaScript functions | Embedding similarity | Environment feedback |
| ExpeL (Zhao et al. 2023) | Natural language insights | Task-type matching | Success/failure tracking |
| Roko SkillLibrary | Prompt templates + tool lists | Tag + file matching + HDC | Gate pipeline verdicts |

Key differences from Voyager:

- **Language-agnostic skills**: Roko skills are prompt templates, not code in a
  specific language. The agent interprets the template and generates code.
- **Deterministic validation**: Gate pipeline provides ground-truth
  success/failure, unlike Minecraft's ambiguous environment feedback.
- **Bounded confidence**: Skills can never reach 1.0 confidence, preventing
  epistemic closure.

See also: EvoSkills (Chen et al. 2023) for evolutionary skill optimization.
