# M103 — Dream Loop Graph Structure

## Objective
Decompose the monolithic `DreamCycle` in `roko-dreams` into a Loop Graph with NREM, REM, and Integration as composable phase nodes. The dream cycle becomes a graph where each phase is a named stage with explicit input/output types, enabling phases to be replaced, extended, or run independently. The existing `DreamCycle::run` delegates to this graph structure.

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/cycle.rs` (monolithic DreamCycle), new file `crates/roko-dreams/src/dream_graph.rs`
- Phase ref: depth doc 11-memory/06-dream-cycle-as-loop.md
- Depth doc: `tmp/unified-depth/11-memory/06-dream-cycle-as-loop.md`

## Steps
1. Discover the current DreamCycle structure:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct\|impl.*DreamCycle' crates/roko-dreams/src/cycle.rs | head -25
   wc -l crates/roko-dreams/src/cycle.rs
   grep -n 'pub struct DreamCycle' crates/roko-dreams/src/cycle.rs
   grep -n 'pub struct DreamCycleReport' crates/roko-dreams/src/cycle.rs
   grep -n 'pub struct StagingBuffer' crates/roko-dreams/src/staging.rs | head -3
   grep -n 'pub enum DreamPhaseKind' crates/roko-dreams/src/phase2/sleep_time.rs | head -3
   ```

2. **Current DreamCycle** (in `crates/roko-dreams/src/cycle.rs`):
   ```rust
   pub struct DreamCycle {
       episode_store: Arc<EpisodeLogger>,
       knowledge_store: Arc<KnowledgeStore>,
       playbook_store: Arc<PlaybookStore>,
       dispatcher: Arc<dyn AgentDispatcher>,
       last_dream_at: Option<DateTime<Utc>>,
       threat_simulation: bool,
       threat_severity_floor: f64,
       staging_buffer: StagingBuffer,
       staging_path: Option<PathBuf>,
       phase_tracker: Option<DreamBudgetTracker>,
   }

   impl DreamCycle {
       pub async fn run(&mut self) -> Result<DreamCycleReport>;
       pub async fn run_budgeted(&mut self, budget: &mut Option<DreamBudget>) -> Result<DreamCycleReport>;
   }
   ```
   **Important**: `run` is **async** because it reads episodes from disk and may call LLM dispatch.

3. **Existing DreamPhaseKind** (in `crates/roko-dreams/src/phase2/sleep_time.rs`):
   ```rust
   pub enum DreamPhaseKind {
       Hypnagogia,   // creative onset
       Nrem,         // replay
       Rem,          // imagination
       Integration,  // pure computation
       Evolution,    // MAP-Elites
   }
   ```
   This enum already exists -- reuse it rather than creating a new `DreamPhase` enum.

4. Create `crates/roko-dreams/src/dream_graph.rs` with phase node definitions:
   ```rust
   use crate::cycle::DreamCycleReport;
   use crate::staging::StagingBuffer;
   use crate::phase2::sleep_time::DreamPhaseKind;
   use roko_learn::episode_logger::{Episode, EpisodeLogger};  // Episode is NOT re-exported from roko_learn root
   use roko_neuro::KnowledgeStore;
   use serde::{Deserialize, Serialize};

   /// Configuration for a single dream phase.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PhaseConfig {
       pub phase: DreamPhaseKind,
       pub budget_seconds: u64,
       pub enabled: bool,
   }

   /// The dream Loop Graph: phases in sequence with a feedback edge
   /// from Integration back to NREM for iterative refinement.
   pub struct DreamGraph {
       pub phases: Vec<PhaseConfig>,
       pub max_iterations: usize,
       pub convergence_threshold: f64,
   }
   ```

5. Implement each phase as a standalone function (sync, since they operate on in-memory data after episode loading):
   ```rust
   pub struct NremResult {
       pub patterns: Vec<ExtractedPattern>,
       pub episode_count: usize,
   }

   pub struct RemResult {
       pub counterfactuals_tested: usize,
       pub knowledge_gaps: Vec<String>,
   }

   pub struct IntegrationResult {
       pub promoted_count: usize,
       pub gc_removed: usize,
       pub converged: bool,
   }

   /// NREM phase: replay episodes, extract patterns.
   pub fn run_nrem(episodes: &[Episode], config: &PhaseConfig) -> NremResult { ... }

   /// REM phase: generate counterfactuals, test heuristics.
   pub fn run_rem(nrem_output: &NremResult, config: &PhaseConfig) -> RemResult { ... }

   /// Integration phase: stage promoted knowledge, verify, write to store.
   pub fn run_integration(
       rem_output: &RemResult,
       staging: &mut StagingBuffer,
       config: &PhaseConfig,
   ) -> IntegrationResult { ... }
   ```

6. Implement the graph executor:
   ```rust
   pub struct DreamGraphResult {
       pub iterations_run: usize,
       pub nrem: NremResult,
       pub rem: RemResult,
       pub integration: IntegrationResult,
   }

   impl DreamGraph {
       /// Execute the dream graph, running phases in sequence with optional iteration.
       pub fn execute(
           &self,
           episodes: &[Episode],
           staging: &mut StagingBuffer,
       ) -> DreamGraphResult {
           let mut last_nrem = None;
           let mut last_rem = None;
           let mut last_integration = None;
           for iteration in 0..self.max_iterations {
               let nrem_cfg = self.phases.iter().find(|p| p.phase == DreamPhaseKind::Nrem);
               let rem_cfg = self.phases.iter().find(|p| p.phase == DreamPhaseKind::Rem);
               let int_cfg = self.phases.iter().find(|p| p.phase == DreamPhaseKind::Integration);
               // Run enabled phases in sequence
               // Check convergence
           }
       }
   }
   ```

7. Register in `crates/roko-dreams/src/lib.rs`:
   ```rust
   pub mod dream_graph;
   ```

8. Write tests:
   - Graph with all phases enabled runs to completion
   - Disabling a phase skips it
   - Max iterations bound is respected
   - Convergence threshold triggers early exit

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- dream_graph
```

## What NOT to do
- Do NOT delete or gut `DreamCycle` -- delegate from it to the graph
- Do NOT implement Trigger scheduling -- that is M104
- Do NOT implement individual NREM/REM Cell details -- those are M105/M106
- Do NOT create a new DreamPhase enum -- reuse `DreamPhaseKind` from `phase2/sleep_time.rs`
- Do NOT modify runner.rs scheduling logic
