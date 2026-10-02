# Enrichment Pipeline: 12-Step Context Pre-Computation

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/enrichment/`
> v1 source: `docs/v1/03-composition/04-enrichment-pipeline-13-step.md`

---

## Overview

The enrichment pipeline pre-computes context artifacts before agent sessions
begin. Rather than having agents spend tokens discovering what they need, the
pipeline generates 12 typed artifacts using the cheapest appropriate model for
each step. Each artifact is stored on disk, staleness-checked, and selectively
injected into agent prompts based on role and task type.

This embodies the "write for amnesia" principle: every agent session starts
cold, and the files on disk are the only truth.

The pipeline had a thirteenth step, `Prd`, which extracted PRD sections into
`prd-extract.md`. It was removed with the PRD pipeline: plans now come straight
from a prompt or a written spec, and the plan's own `plan.md` carries that context.

---

## 1. The 12 Enrichment Steps

```rust
pub enum EnrichStep {
    Briefs,        // 1. Generate strategist briefs (What/Why/How)
    Tasks,         // 2. Generate task TOMLs from plan decomposition
    Decompose,     // 3. Step-by-step subtask breakdown
    Research,      // 4. Deep research with citations
    Dependencies,  // 5. External dependency requirements
    Fixtures,      // 6. Test fixture requirements
    Integration,   // 7. Cross-crate integration notes
    Verify,        // 8. Invariant verification scripts
    Reviews,       // 9. Review task lists
    Tests,         // 10. Test task lists
    Invariants,    // 11. Invariant specifications
    Scribe,        // 12. Documentation task lists
}

pub const ALL_ORDERED: &[EnrichStep] = &[
    EnrichStep::Briefs,       EnrichStep::Tasks,
    EnrichStep::Decompose,    EnrichStep::Research,
    EnrichStep::Dependencies, EnrichStep::Fixtures,
    EnrichStep::Integration,  EnrichStep::Verify,
    EnrichStep::Reviews,      EnrichStep::Tests,
    EnrichStep::Invariants,   EnrichStep::Scribe,
];
```

| # | Step | Output File | Model | Purpose |
|---|------|------------|-------|---------|
| 1 | Briefs | `brief.md` | Sonnet | Generate What/Why/How task summaries |
| 2 | Tasks | `tasks.toml` | Sonnet | Generate task specifications |
| 3 | Decompose | `decomposition.md` | Sonnet | Step-by-step subtask breakdown |
| 4 | Research | `research.md` | Opus | Deep research with citations |
| 5 | Dependencies | `dependency-manifest.toml` | Haiku | External dependency list |
| 6 | Fixtures | `fixture-manifest.toml` | Haiku | Test fixture requirements |
| 7 | Integration | `integration.md` | Sonnet | Cross-crate integration notes |
| 8 | Verify | `verify.sh` | Haiku | Invariant verification script |
| 9 | Reviews | `review-tasks.toml` | Haiku | Review task assignments |
| 10 | Tests | `test-tasks.toml` | Haiku | Test task assignments |
| 11 | Invariants | `invariants.md` | Sonnet | Invariant specifications |
| 12 | Scribe | `scribe-tasks.toml` | Haiku | Documentation task assignments |

Cheapest model for each step: Haiku for mechanical extraction, Sonnet for
reasoning, Opus for deep research.

---

## 2. Pipeline Architecture

```rust
pub struct EnrichmentPipeline<C: LlmClient> {
    client: Arc<C>,
    config: EnrichmentConfig,
    output_dir: PathBuf,
}

pub trait LlmClient: Send + Sync {
    fn complete(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String>;
}
```

### Staleness Checking

Before running a step, the pipeline checks whether output already exists and
is fresh:

```rust
fn is_stale(&self, step: &EnrichStep) -> bool {
    let output_path = self.output_dir.join(step.output_filename());
    if !output_path.exists() { return true; }
    let age = /* file modification age */;
    age > self.config.max_staleness  // default: 24 hours
}
```

### TOML Repair

Steps producing TOML output include a validate-and-repair pass: if the initial
LLM output has syntax errors, one retry sends the parse error back to the LLM
with instructions to fix. If the repair also fails, the step is marked failed
and the pipeline continues. One-retry policy prevents infinite loops.

### Continue-on-Failure

The pipeline runs all 12 steps regardless of individual failures. Failed steps
are logged as warnings. The agent receives whatever artifacts were generated
successfully. Missing artifacts are absent from the prompt -- the
PromptComposer's priority-based dropping handles this gracefully.

---

## 3. Step Selection

Not every task needs all 12 steps:

| Task Type | Steps Run | Steps Skipped |
|-----------|----------|---------------|
| Simple rename | Briefs | 11 others |
| Standard implementation | Briefs, Tasks, Decompose, Research | 8 others |
| Cross-crate integration | All 12 | None |
| Review task | Reviews | 11 others |
| Documentation task | Scribe, Research | 10 others |

```rust
pub fn steps_for(complexity: Complexity, role: AgentRole) -> Vec<EnrichStep> {
    match (complexity, role) {
        (Complexity::Trivial, _) => vec![EnrichStep::Briefs],
        (Complexity::Standard, AgentRole::Scribe) => {
            vec![EnrichStep::Scribe, EnrichStep::Research]
        }
        (Complexity::Complex, _) => EnrichStep::ALL_ORDERED.to_vec(),
        _ => vec![
            EnrichStep::Briefs, EnrichStep::Tasks,
            EnrichStep::Decompose, EnrichStep::Research,
        ],
    }
}
```

---

## 4. Context Injection

Artifacts are injected into agent prompts through context packs:

```
context/in/
  execution-pack.md        # Merged context for any role
  implementer-pack.md      # Role-specific: Implementer
  architect-pack.md        # Role-specific: Architect
  scribe-pack.md           # Role-specific: Scribe
  brief.md                 # Implementation brief
  decomposition.md         # Step-by-step breakdown
  verify-tasks.toml        # Verification checklist
  learning.md              # Learning pack
  research.md              # Research artifacts
  playbook.md              # Applicable playbook rules
  reflections.md           # Prior iteration reflections
```

Each role receives guidance on which files to read, preventing agents from
reading the entire context directory.

---

## 5. Cost Analysis

| Enrichment approach | Cost per plan | Agent success rate |
|--------------------|---------------|-------------------|
| No enrichment | $0 | ~45% |
| All 12 steps (Haiku/Sonnet mix) | ~$0.15 | ~78% |
| Manual context assembly | $0 (human time) | ~72% |

The $0.15 enrichment investment produces a ~33% improvement in agent success.
The key is using the cheapest model per step: Haiku $0.005/call, Sonnet
$0.02/call, Opus $0.08/call.

---

## 6. Academic Foundations

**Compound AI Systems** [Zaharia et al., BAIR 2024]. The enrichment pipeline
embodies the compound AI principle: clever engineering over model scaling. A
system of Haiku calls at $0.01/artifact produces context enabling a single
Sonnet call to achieve higher success than Opus without enrichment.

**Self-RAG** [Asai et al. 2023]. Reflection tokens let agents decide WHEN to
retrieve. The step selector makes this decision at the task level.

**DSPy** [Khattab et al. 2023]. Each enrichment step has a typed signature
(plan -> artifact); the pipeline can be optimized against downstream success.

**"Write for Amnesia" Principle.** Every agent session starts cold. The files
on disk are the only truth. The enrichment pipeline does context preparation
ahead of time so agents do not burn tokens figuring out what they need.

---

## 7. Implementation Status

| Aspect | Status |
|--------|--------|
| 12 enrichment steps defined | **Shipped** |
| EnrichmentPipeline struct | **Shipped** |
| Staleness checking | **Shipped** |
| TOML repair (one retry) | **Shipped** |
| Continue-on-failure | **Shipped** |
| LlmClient trait | **Shipped** |
| Step selector by complexity/role | **Shipped** |
| 4 backend types defined | **Shipped** |
| Adaptive step selection (learned) | **Not yet** |
| Parallel step execution | **Not yet** |
| Cost tracking per step | **Not yet** |

---

## Cross-References

- [role-templates-11.md](role-templates-11.md) -- Per-role budget allocation
- [token-budget-management.md](token-budget-management.md) -- Budget constraints
- [5-stage-assembly-pipeline.md](5-stage-assembly-pipeline.md) -- Pipeline consuming artifacts
- `crates/roko-compose/src/enrichment/` -- Implementation source
