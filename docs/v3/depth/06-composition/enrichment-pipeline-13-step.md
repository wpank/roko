# Enrichment Pipeline: 13-Step Context Pre-Computation

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/enrichment/`
> v1 source: `docs/v1/03-composition/04-enrichment-pipeline-13-step.md`

---

## Overview

The enrichment pipeline pre-computes context artifacts before agent sessions
begin. Rather than having agents spend tokens discovering what they need, the
pipeline generates 13 typed artifacts using the cheapest appropriate model for
each step. Each artifact is stored on disk, staleness-checked, and selectively
injected into agent prompts based on role and task type.

This embodies the "write for amnesia" principle: every agent session starts
cold, and the files on disk are the only truth.

---

## 1. The 13 Enrichment Steps

```rust
pub enum EnrichStep {
    Prd,           // 1. Extract PRD sections relevant to this plan
    Briefs,        // 2. Generate strategist briefs (What/Why/How)
    Tasks,         // 3. Generate task TOMLs from plan decomposition
    Decompose,     // 4. Step-by-step subtask breakdown
    Research,      // 5. Deep research with citations
    Dependencies,  // 6. External dependency requirements
    Fixtures,      // 7. Test fixture requirements
    Integration,   // 8. Cross-crate integration notes
    Verify,        // 9. Invariant verification scripts
    Reviews,       // 10. Review task lists
    Tests,         // 11. Test task lists
    Invariants,    // 12. Invariant specifications
    Scribe,        // 13. Documentation task lists
}

pub const ALL_ORDERED: &[EnrichStep] = &[
    EnrichStep::Prd,        EnrichStep::Briefs,
    EnrichStep::Tasks,      EnrichStep::Decompose,
    EnrichStep::Research,   EnrichStep::Dependencies,
    EnrichStep::Fixtures,   EnrichStep::Integration,
    EnrichStep::Verify,     EnrichStep::Reviews,
    EnrichStep::Tests,      EnrichStep::Invariants,
    EnrichStep::Scribe,
];
```

| # | Step | Output File | Model | Purpose |
|---|------|------------|-------|---------|
| 1 | Prd | `prd-extract.md` | Haiku | Extract plan-relevant PRD sections |
| 2 | Briefs | `brief.md` | Sonnet | Generate What/Why/How task summaries |
| 3 | Tasks | `tasks.toml` | Sonnet | Generate task specifications |
| 4 | Decompose | `decomposition.md` | Sonnet | Step-by-step subtask breakdown |
| 5 | Research | `research.md` | Opus | Deep research with citations |
| 6 | Dependencies | `dependency-manifest.toml` | Haiku | External dependency list |
| 7 | Fixtures | `fixture-manifest.toml` | Haiku | Test fixture requirements |
| 8 | Integration | `integration.md` | Sonnet | Cross-crate integration notes |
| 9 | Verify | `verify.sh` | Haiku | Invariant verification script |
| 10 | Reviews | `review-tasks.toml` | Haiku | Review task assignments |
| 11 | Tests | `test-tasks.toml` | Haiku | Test task assignments |
| 12 | Invariants | `invariants.md` | Sonnet | Invariant specifications |
| 13 | Scribe | `scribe-tasks.toml` | Haiku | Documentation task assignments |

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

The pipeline runs all 13 steps regardless of individual failures. Failed steps
are logged as warnings. The agent receives whatever artifacts were generated
successfully. Missing artifacts are absent from the prompt -- the
PromptComposer's priority-based dropping handles this gracefully.

---

## 3. Step Selection

Not every task needs all 13 steps:

| Task Type | Steps Run | Steps Skipped |
|-----------|----------|---------------|
| Simple rename | Prd, Briefs | 11 others |
| Standard implementation | Prd, Briefs, Tasks, Decompose, Research | 8 others |
| Cross-crate integration | All 13 | None |
| Review task | Prd, Reviews | 11 others |
| Documentation task | Prd, Scribe, Research | 10 others |

```rust
pub fn steps_for(complexity: Complexity, role: AgentRole) -> Vec<EnrichStep> {
    match (complexity, role) {
        (Complexity::Trivial, _) => vec![EnrichStep::Prd, EnrichStep::Briefs],
        (Complexity::Standard, AgentRole::Scribe) => {
            vec![EnrichStep::Prd, EnrichStep::Scribe, EnrichStep::Research]
        }
        (Complexity::Complex, _) => EnrichStep::ALL_ORDERED.to_vec(),
        _ => vec![
            EnrichStep::Prd, EnrichStep::Briefs, EnrichStep::Tasks,
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
  prd2-extract.md          # Relevant PRD sections
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
| All 13 steps (Haiku/Sonnet mix) | ~$0.15 | ~78% |
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
| 13 enrichment steps defined | **Shipped** |
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
