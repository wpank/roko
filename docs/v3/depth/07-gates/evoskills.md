# EvoSkills -- Self-Evolving Verification Skills

> Depth file for [07-GATES.md](../../07-GATES.md) section 13.
> Source: `crates/roko-learn/src/skill_library.rs`,
> `crates/roko-learn/src/pattern_discovery.rs`

> **Citation**: Wang et al. "Voyager: An Open-Ended Embodied Agent with Large
> Language Models" (arXiv:2305.16291, 2023) -- open-ended skill library with
> verification-driven skill accumulation.

---

## 1. The Core Idea

EvoSkills accumulates reusable tool-use patterns from successful task
executions, validates them through adversarial testing, and injects the
survivors into future agent prompts. The skill library is the agent's
long-term procedural memory.

Reference results from the Voyager-inspired architecture:

| Condition | Success rate |
|---|---|
| Baseline (no skills) | 32% |
| With EvoSkills | 75% (+43pp) |
| Cross-model transfer | +35-44pp |

These numbers come from adversarial surrogate verification: skills that
survive adversarial testing are genuinely useful, not just incidental patterns
from lucky executions.

---

## 2. Three-Tier Learning Hierarchy

Skills emerge through a three-tier system:

### Tier 1: Episodes (Raw)

Every agent execution is recorded in `.roko/episodes.jsonl`. An episode
contains the task specification, ordered tool calls, gate verdicts, final
outcome, token counts, and timing data. Episodes are write-once, never
modified.

### Tier 2: Patterns (Extracted)

When 5 or more similar episodes show the same tool-use sequence leading to
gate passage, a pattern is extracted:

```
Pattern: "Rust Compile Fix -- Missing Import"
Precondition: compile gate fails with error[E0433] or error[E0425]
Procedure:
  1. Read the error message to identify the missing symbol
  2. Search for the symbol in the codebase (Grep)
  3. Identify the crate/module that exports it
  4. Add the use statement to the file
Postcondition: compile gate passes
```

Patterns are hypotheses. They suggest that a particular approach works for a
particular kind of task. They are not yet validated in production.

### Tier 3: Playbook (Validated)

When a pattern has been successfully applied 5 or more times in live
execution (not just extracted from historical data), it is promoted to a
playbook rule. Playbook rules are:

- Injected into agent context via the prompt's "skills" section
- Tracked with confidence scores and usage telemetry
- Queried at dispatch time by the runner and matched to the current task

> **Citation**: SAGE (arXiv:2512.17102) -- "Agents that accumulate reusable
> tool-use patterns across tasks use 26% fewer steps and 59% fewer tokens."

---

## 3. Adversarial Surrogate Verification

The validation step differentiates EvoSkills from naive pattern caching.
Rather than trusting that extracted patterns generalize, the system tests them
adversarially.

### 3.1 Surrogate Test Generation

For each candidate skill, generate adversarial test cases that specifically
try to break the skill:

- Edge cases the skill might not handle
- Input variations that test generalization
- Failure modes that reveal brittle patterns

### 3.2 Cross-Model Testing

Apply the skill with different models to verify transfer:

- If a skill works with Model A but fails with Model B, it may be an artifact
  of Model A's specific capabilities rather than genuine task-completion
  knowledge
- Skills validated across 3 or more models are robust

### 3.3 Confidence Scoring

Each skill maintains a confidence score:

```
confidence = (validations / (validations + failures)) * cross_model_factor
```

Where `cross_model_factor` is:

| Models validated on | Factor |
|---|---|
| 3 or more | 1.0 |
| 2 | 0.8 |
| 1 | 0.6 |

Skills below 0.5 confidence are demoted back to Tier 2 for re-extraction.

---

## 4. Skill Structure

```rust
pub struct Skill {
    pub id: String,
    pub name: String,
    pub precondition: String,         // When to apply
    pub procedure: String,            // What to do
    pub postcondition: String,        // Expected outcome
    pub confidence: f64,              // [0, 1]
    pub source_episodes: Vec<String>, // Where it came from
    pub validations: u64,             // Successful applications
    pub failures: u64,                // Failed applications
    pub task_categories: Vec<String>, // Applicable task types
    pub created_at: String,
    pub last_validated_at: Option<String>,
}
```

### Example

```
Skill: "Rust Compile Fix -- Missing Import"
Precondition:  compile gate fails with error[E0433] or error[E0425]
Procedure:     Read error -> Grep for symbol -> Identify export -> Add use
Postcondition: compile gate passes
Confidence:    0.92 (validated 46 times, failed 4 times, 3 models)
```

---

## 5. Skill Injection into Agent Context

The SystemPromptBuilder injects validated skills into the agent's prompt as a
dedicated section:

```
## Relevant Skills

Based on the current task (compile error fix, auth module), the following
verified skills may be applicable:

### Skill: Rust Compile Fix -- Missing Import (confidence: 0.92)
When: compile gate fails with error[E0433] or error[E0425]
Do: Read error -> Grep for symbol -> Identify export -> Add use statement
Expected: compile gate passes

### Skill: Auth Module Test Pattern (confidence: 0.78)
When: task involves auth module changes
Do: Read existing auth tests -> Modify in parallel -> Run TestGate
Expected: all auth tests pass
```

The playbook store's `query_top_matches()` function selects the most relevant
skills at dispatch time. The section effectiveness tracker monitors whether
skill injection improves gate pass rates. Skills whose injection does not help
get their priority reduced.

---

## 6. Cross-Model Transfer

The +35-44pp cross-model transfer improvement means skills extracted from
Model A's successes help Model B succeed:

```
Model A executes 100 tasks:
  -> 50 succeed (50% base rate)
  -> Extract 15 skills from successes

Model B executes 100 tasks (same distribution):
  -> Without skills: 32% success rate
  -> With Model A's skills: 67-76% (+35-44pp)
```

Transfer works because skills encode *task-completion knowledge* -- how to
fix compile errors, how to write tests, how to modify auth modules -- not
model-specific behaviors. "Read the error, search for the symbol, add the
import" works regardless of which model executes it.

Practical implication: **new models get instant expertise**. When a new model
enters the routing pool, it immediately benefits from the accumulated skill
library without any warm-up period.

---

## 7. Skill Evolution

Skills evolve through three mechanisms:

### 7.1 Refinement

When a skill fails, the failure is analyzed. Was the precondition too broad?
Narrow it. Was the procedure missing a step? Add it. Was the postcondition
wrong? Correct the expected outcome.

### 7.2 Specialization

A general skill that works for most cases but fails for a specific sub-case
spawns a specialized variant:

```
General:      "Fix compile error -- missing import"
Specialized:  "Fix compile error -- missing import from workspace crate"
  (adds: check Cargo.toml for workspace dependencies before adding use)
```

### 7.3 Retirement

Skills whose confidence drops below the threshold (due to codebase evolution
making old patterns obsolete) are retired from the active playbook, retained
in the archive for historical analysis, and may be re-extracted if conditions
change.

---

## 8. Skill Genome and MAP-Elites

For systematic evolution, skills are represented as genomes with evolvable
parameters:

```rust
pub struct SkillGenome {
    pub skill: Skill,
    pub prompt_template: String,          // How the skill is described
    pub tool_preferences: Vec<(String, f64)>, // Tool priority weights
    pub retry_config: RetryGenome,
    pub temperature: f64,                 // [0.0, 1.0]
    pub token_budget: usize,
    pub gate_weights: Vec<f64>,           // Per-rung fitness weights
    pub behavior: BehavioralDescriptor,   // Measured, not set
    pub fitness: f64,
}
```

MAP-Elites maintains a quality-diversity archive indexed by four behavioral
axes:

| Axis | What it measures |
|---|---|
| Completion rate | Fraction of tasks the skill completes |
| Gate score | Average gate score across all rungs |
| Token efficiency | Inverse of tokens per successful task |
| Generalization | Fraction of task categories covered |

The archive stores the highest-fitness genome for each behavioral cell. Two
metrics track archive health:

- **Coverage**: fraction of cells filled (more = more diverse)
- **QD-score**: sum of all fitness values (higher = better quality AND diversity)

> **Citation**: Mouret & Clune, "Illuminating Search Spaces by Mapping Elites"
> (arXiv:1504.04909, 2015).

---

## 9. Mutation and Crossover

Evolution operators produce offspring from archived genomes:

**Mutation**: Gaussian perturbation on continuous parameters (temperature,
token budget), categorical mutation on discrete parameters (prompt template,
tool preferences). Mutation rates are landscape-adaptive: rugged landscapes
get stronger mutations to escape local optima; flat landscapes get more
crossover to search broadly.

**Crossover**: Uniform crossover on discrete fields (each field taken from
one of two parents with 50% probability), intermediate crossover on continuous
fields (offspring value is a weighted average of parents).

**Fitness evaluation**: A skill genome is evaluated by running it on sampled
tasks and measuring gate outcomes. Fitness is a weighted combination:

```
fitness = 0.5 * gate_pass_rate
        + 0.3 * token_efficiency
        + 0.2 * generalization
```

---

## 10. Speciation

Skills using fundamentally different strategies (e.g., "fix by reading error
messages" vs. "fix by searching codebase for patterns") are grouped into
species using a compatibility distance metric. Speciation prevents strategies
from competing directly and preserves diversity.

The compatibility distance combines prompt similarity (n-gram Jaccard), tool
preference distance (cosine), and parameter distance (normalized L2). Genomes
within the threshold distance belong to the same species.

Species that stagnate for 15 generations without fitness improvement are
dissolved, reallocating their evaluation budget to more productive species.

> **Citation**: Stanley & Miikkulainen, "Evolving Neural Networks through
> Augmenting Topologies" (NEAT, Evolutionary Computation, 2002) -- speciation
> via compatibility distance.

---

## 11. Relationship to Verification

EvoSkills are deeply connected to the gate pipeline:

| Direction | Mechanism |
|---|---|
| Gates -> Skills | Skills extracted from episodes where ALL gates passed |
| Gates -> Validation | Skill validation = apply on new task, check gates pass |
| Skills -> Gates | Injected skills improve gate pass rates on future tasks |
| Gates -> Retirement | Persistent gate failures with a skill -> retire it |

The feedback loop: skills extracted from gate successes are injected into
prompts, leading to more gate successes, leading to more skill extraction.
Adversarial verification prevents noise accumulation in this positive loop.

---

## 12. Persistence

```
.roko/learn/
  skill-archive.json       # MAP-Elites archive with genomes
  species.json             # Species membership and stagnation
  landscape.json           # Fitness landscape analysis
  playbooks.json           # Active Tier 3 playbook rules
```

The archive persists across sessions. Evolution continues from the last
saved state, and skills survive process restarts.

---

## 13. Test criteria

| Test | Property |
|---|---|
| `pattern_extraction_requires_5_episodes` | Fewer than 5 similar episodes = no pattern |
| `skill_confidence_decreases_on_failure` | Failed application reduces confidence |
| `cross_model_factor_scales_confidence` | 1 model = 0.6x, 3 models = 1.0x |
| `retirement_below_threshold` | Confidence < 0.5 -> demoted to Tier 2 |
| `archive_coverage_increases` | Successive insertions fill more cells |
| `qd_score_never_decreases_on_replacement` | Better genome replaces worse, never the reverse |
| `mutation_respects_bounds` | Temperature stays in [0,1], budget stays positive |
| `speciation_groups_similar_genomes` | Distance < threshold -> same species |
| `stagnation_dissolves_species` | 15 gens without improvement -> species dissolved |
| `skill_injection_includes_precondition` | Prompt section contains when/do/expected |
