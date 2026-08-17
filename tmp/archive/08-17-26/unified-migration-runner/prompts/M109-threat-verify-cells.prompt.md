# M109 — Threat Simulation Verify Cells

## Objective
Implement threat simulation as Verify Cells that stress-test the knowledge Store: FMEA (Failure Mode and Effects Analysis) as a Score Cell that rates threats by severity and probability, FTA (Fault Tree Analysis) as structural graph traversal, and a Nightmare Detection pipeline that identifies catastrophic knowledge gaps. These cells integrate into the dream cycle's Integration phase.

## Scope
- Crates: `roko-dreams`
- Files: `crates/roko-dreams/src/threat.rs` (existing), new file `crates/roko-dreams/src/threat_cells.rs`
- Phase ref: depth doc 11-memory/10-threat-simulation-and-nightmares.md
- Depth doc: `tmp/unified-depth/11-memory/10-threat-simulation-and-nightmares.md`

## Steps
1. Discover existing threat simulation code and types:
   ```bash
   grep -n 'pub fn\|pub async fn\|pub struct\|pub enum' crates/roko-dreams/src/threat.rs | head -20
   wc -l crates/roko-dreams/src/threat.rs
   grep -n 'pub fn\|pub struct' crates/roko-dreams/src/phase2/threat.rs 2>/dev/null | head -10
   grep 'roko-neuro\|roko-learn' crates/roko-dreams/Cargo.toml
   ```
   **Existing types** (in `crates/roko-dreams/src/threat.rs`):
   - `ThreatScenario` -- struct with severity, description, related episodes
   - `enumerate_threats(episodes: &[Episode]) -> Vec<ThreatScenario>` -- discover threats
   - `threat_warning_entries(...)` -- generate Warning KnowledgeEntries from threats
   - `threat_warning_entries_with_floor(...)` -- with severity floor
   - `ThreatScenario::severity() -> f64`

2. Create `crates/roko-dreams/src/threat_cells.rs`:
   ```rust
   /// A threat identified by the simulation.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ThreatEntry {
       pub id: String,
       pub description: String,
       pub severity: f64,      // 0.0-1.0
       pub probability: f64,   // 0.0-1.0
       pub risk_score: f64,    // severity * probability
       pub related_knowledge: Vec<String>,  // entry IDs
       pub mitigation_status: MitigationStatus,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum MitigationStatus {
       /// No mitigation exists in the knowledge store
       Unmitigated,
       /// Partial mitigation found
       Partial { coverage: f64 },
       /// Fully mitigated by existing knowledge
       Mitigated { by: Vec<String> },
   }
   ```

3. Implement FMEA Score Cell:
   ```rust
   /// FMEA Score Cell: rates potential failure modes by severity and probability.
   /// Scans the knowledge store for heuristics and checks if their failure
   /// conditions are covered by AntiKnowledge or other defensive entries.
   pub struct FmeaScoreCell {
       /// Minimum risk score to report (filter low-risk threats)
       pub risk_threshold: f64,
   }

   impl FmeaScoreCell {
       pub fn analyze(
           &self,
           heuristics: &[KnowledgeEntry],
           store: &KnowledgeStore,
       ) -> Vec<ThreatEntry> { ... }
   }
   ```

4. Implement FTA traversal:
   ```rust
   /// FTA (Fault Tree Analysis): given a top-level failure,
   /// traverse the knowledge graph to find root causes.
   pub struct FtaTraversalCell {
       /// Maximum depth to traverse
       pub max_depth: usize,
   }

   impl FtaTraversalCell {
       pub fn trace(
           &self,
           threat: &ThreatEntry,
           store: &KnowledgeStore,
       ) -> FaultTree { ... }
   }

   #[derive(Debug, Clone)]
   pub struct FaultTree {
       pub root: ThreatEntry,
       pub causes: Vec<FaultNode>,
   }

   #[derive(Debug, Clone)]
   pub struct FaultNode {
       pub cause: String,
       pub depth: usize,
       pub knowledge_coverage: f64,
       pub children: Vec<FaultNode>,
   }
   ```

5. Implement Nightmare Detection pipeline:
   ```rust
   /// Nightmare Detection: identifies catastrophic knowledge gaps where
   /// high-severity threats have no mitigation and no AntiKnowledge coverage.
   pub struct NightmareDetector {
       /// Minimum severity to be considered a "nightmare"
       pub severity_threshold: f64,
       /// Maximum acceptable unmitigated risk
       pub max_unmitigated_risk: f64,
   }

   impl NightmareDetector {
       pub fn detect(
           &self,
           threats: &[ThreatEntry],
           fault_trees: &[FaultTree],
       ) -> Vec<NightmareReport> { ... }
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct NightmareReport {
       pub threat: ThreatEntry,
       pub root_causes: Vec<String>,
       pub recommended_actions: Vec<String>,
       pub urgency: f64,
   }
   ```

6. Compose into a threat pipeline:
   ```rust
   pub struct ThreatPipeline {
       pub fmea: FmeaScoreCell,
       pub fta: FtaTraversalCell,
       pub nightmare: NightmareDetector,
   }

   impl ThreatPipeline {
       pub fn run(&self, store: &KnowledgeStore) -> ThreatReport { ... }
   }
   ```

7. Register in `crates/roko-dreams/src/lib.rs`:
   ```rust
   pub mod threat_cells;
   ```

8. Write tests:
   - FMEA identifies threats from undefended heuristics
   - FTA traces causes to correct depth
   - Nightmare detector flags high-severity unmitigated threats
   - Mitigated threats are not flagged as nightmares
   - Empty store produces no threats

## Verification
```bash
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams -- threat_cells
```

## What NOT to do
- Do NOT modify existing `threat.rs` -- add the new cells alongside it
- Do NOT implement actual LLM-based threat analysis -- use structural/HDC methods
- Do NOT implement nightmare containment (quarantine + Pulse emission) -- that is a React concern
- Do NOT add external risk assessment libraries
