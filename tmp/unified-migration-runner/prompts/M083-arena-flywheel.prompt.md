# M083 — 7-Step Arena Flywheel

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/19-arenas/` depth docs. The depth docs specify: HDC fingerprint similarity thresholds, Bradley-Terry MLE implementation details, curriculum generation strategies, and pattern extraction with mandatory falsifiers.

## Objective
Implement the 7-step arena flywheel: TRACE (record execution) -> AUTO-GRADE (scoring function) -> PREFERENCE-MINE (extract pairwise preferences via Bradley-Terry MLE) -> FAILURE-CLUSTER (group failures by HDC similarity) -> CURRICULUM-GEN (generate training tasks from failure clusters) -> PATTERN-EXTRACT (extract Heuristic Signals with mandatory falsifiers) -> PREFERENCE-BOOTSTRAP (create training data from preferences). The flywheel is self-reinforcing and IS the arena.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/arena/flywheel.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Spec ref: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` SS4 (7-Step Flywheel)
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Read the arena types from M082:
   ```bash
   cat crates/roko-learn/src/arena/types.rs 2>/dev/null | head -40
   ```

2. Implement each step of the flywheel:
   ```rust
   pub struct Flywheel {
       arena: Arena,
       episodes: Vec<Episode>,
   }

   impl Flywheel {
       pub fn trace(&mut self, episode: Episode);
       pub fn auto_grade(&self, episode: &Episode) -> Verdict;
       pub fn preference_mine(&self) -> Vec<PairwisePreference>;
       pub fn failure_cluster(&self) -> Vec<FailureCluster>;
       pub fn curriculum_gen(&self, clusters: &[FailureCluster]) -> Vec<Task>;
       pub fn pattern_extract(&self) -> Vec<HeuristicSignal>;
       pub fn preference_bootstrap(&self, patterns: &[HeuristicSignal]) -> Vec<PairwisePreference>;
       pub fn run_cycle(&mut self) -> FlywheelOutput;
   }
   ```

3. Step 6 (PATTERN-EXTRACT) is load-bearing: extracted patterns MUST include a mandatory falsifier derived from failure clusters.

4. Write tests: feed 100 episodes into the flywheel, confirm each step produces output.

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- arena::flywheel
```

## What NOT to do
- Do NOT implement real Bradley-Terry MLE -- use a simplified version until depth docs specify
- Do NOT skip the mandatory falsifier on Heuristic Signals
- Do NOT proceed without depth docs
- Do NOT implement LLM-based grading -- ground truth is external only
