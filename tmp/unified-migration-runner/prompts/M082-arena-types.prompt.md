# M082 — Define Arena Types

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/19-arenas/` depth docs being written first. The depth docs specify scoring function interfaces, ground truth source bindings, and leaderboard algorithms.

## Objective
Define the Arena type system: Arena struct (id, name, task source, scoring function, leaderboard config, ground truth source), 8 concrete arena kinds (Coding, Trading, Prediction, Research, Security Audit, Optimization, Agentic, MetaArena), and the Arena as a universal measurement surface. Arenas connect Agent behavior to ground truth.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/arena/mod.rs` (new directory), `crates/roko-learn/src/arena/types.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Spec ref: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` SS2-3
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Check for existing arena code:
   ```bash
   grep -rn 'arena\|Arena' crates/roko-learn/src/ --include='*.rs' | head -10
   ```

2. Define arena types in `crates/roko-learn/src/arena/types.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Arena {
       pub id: String,
       pub name: String,
       pub kind: ArenaKind,
       pub task_source: TaskSource,
       pub scoring_fn: ScoringFunction,
       pub leaderboard_config: LeaderboardConfig,
       pub ground_truth: GroundTruthSource,
       pub created_at: DateTime<Utc>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ArenaKind {
       Coding,
       Trading,
       Prediction,
       Research,
       SecurityAudit,
       Optimization,
       Agentic,
       MetaArena,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum TaskSource {
       TestSuite { path: PathBuf },
       Oracle { endpoint: String },
       HumanReview,
       ChainState { contract: String },
       Benchmark { dataset: String },
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ScoringFunction {
       Binary,                         // pass/fail
       Continuous { min: f64, max: f64 }, // numeric score
       Conjunctive { criteria: Vec<String> }, // all must pass
       Pareto { dimensions: Vec<String> },   // multi-objective
   }
   ```

3. Write tests: arena types compile and serialize correctly.

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- arena::types
```

## What NOT to do
- Do NOT implement the flywheel -- that is M083
- Do NOT implement eval protocol -- that is M084
- Do NOT implement bounty system -- that is M085
- Do NOT proceed without depth docs for scoring function details
