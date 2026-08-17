# PAF_03: Wire section effectiveness scoring from prompt assembly outcomes

## Task
Track which prompt sections (from the 9-layer SystemPromptBuilder) correlate with gate success, enabling future VCG auction and section pruning.

## Runner Context
Runner PAF (Knowledge Lifecycle), batch 3 of 3. Depends on PAF_02.

## Problem
KL-3 anti-pattern: "Assemble prompt, never evaluate sections." The SystemPromptBuilder produces 9 prompt layers, each with a cost estimate. But there's no feedback on which sections contributed to task success. Without this data, the VCG auction (which theoretically optimizes section allocation) has no training signal.

## Current Code

**PromptSection** — `crates/roko-compose/src/system_prompt_builder.rs`:
Each section has a name, content, and token estimate.

**VCG auction** — `crates/roko-compose/` (built but greedy path dominates):
Would use per-section effectiveness if available.

**ExperimentStore** — `crates/roko-learn/src/experiments.rs`:
Can record prompt experiment outcomes. Could be extended for section effectiveness.

## Exact Changes

### Step 1: Record section manifest per task

When the SystemPromptBuilder produces a prompt, capture the section manifest:

```rust
struct SectionManifest {
    task_id: String,
    sections: Vec<SectionRecord>,
    total_tokens: u64,
    assembled_at: u64,
}

struct SectionRecord {
    name: String,
    token_count: u64,
    included: bool,  // was it included or pruned by budget?
}
```

### Step 2: After gate outcome, score sections

```rust
// After gate pass/fail:
if let Some(manifest) = &task_section_manifest {
    let outcome = if gate_passed { 1.0 } else { 0.0 };
    for section in &manifest.sections {
        if section.included {
            // Record: this section was present when gate passed/failed
            section_effectiveness.record(
                &section.name,
                outcome,
                section.token_count,
            );
        }
    }
}
```

### Step 3: Persist section effectiveness

```rust
struct SectionEffectiveness {
    scores: HashMap<String, SectionScore>,
}

struct SectionScore {
    total_observations: u64,
    success_count: u64,
    total_tokens_spent: u64,
    effectiveness: f64,  // EMA of success rate when included
}

impl SectionEffectiveness {
    fn record(&mut self, section_name: &str, outcome: f64, tokens: u64) {
        let score = self.scores.entry(section_name.to_string())
            .or_insert(SectionScore::default());
        score.total_observations += 1;
        if outcome > 0.5 { score.success_count += 1; }
        score.total_tokens_spent += tokens;
        // EMA with alpha=0.1
        score.effectiveness = 0.9 * score.effectiveness + 0.1 * outcome;
    }

    fn save(&self, path: &Path) -> Result<()> { /* JSON */ }
    fn load(path: &Path) -> Result<Self> { /* JSON */ }
}
```

Persist to `.roko/learn/section-effectiveness.json`.

### Step 4: Make effectiveness data available to future prompt assembly

The VCG auction or greedy allocator can consult effectiveness:

```rust
// In SystemPromptBuilder, when budgeting:
if let Some(eff) = section_effectiveness.get(&section.name) {
    section.priority_boost = eff.effectiveness;  // higher effectiveness → higher priority
}
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (capture manifest, score after gates)
- `crates/roko-learn/src/section_effectiveness.rs` (new file — SectionEffectiveness store)

## Read-Only Context
- `crates/roko-compose/src/system_prompt_builder.rs` (section structure)
- `crates/roko-learn/src/experiments.rs` (ExperimentStore pattern reference)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Section manifest captured per task at prompt assembly time
- Gate outcome feeds into per-section effectiveness score
- Effectiveness persisted to `.roko/learn/section-effectiveness.json`
- EMA smoothing prevents single-task noise from dominating
- Data available for future VCG auction consultation

## Do NOT
- Change the SystemPromptBuilder
- Activate VCG auction (that's separate — this just provides the training data)
- Prune sections based on effectiveness in this prompt (observation only)
