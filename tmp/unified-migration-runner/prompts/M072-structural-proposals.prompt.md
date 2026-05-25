# M072 — L4 Structural Change Proposals

## Objective
Define the StructuralProposal type and supporting types for Learning Loop 4 (Structural Self-Evolution). Proposals are Signals published on Bus that describe changes to system structure: modifying Graphs, adding/removing Cells, changing configuration, or updating the Verify pipeline. Each proposal includes evidence (references to Signals that motivated it) and requires human approval before application.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/structural.rs` (new), `crates/roko-learn/src/lib.rs`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.3
- Spec ref: `tmp/unified/10-LEARNING-LOOPS.md` SS5 (Loop 4)

## Steps
1. Check for existing structural or proposal types:
   ```bash
   grep -rn 'Structural\|structural\|Proposal\|proposal\|ProposalKind' crates/roko-learn/src/ --include='*.rs' | head -10
   ```

2. Define the proposal types in `crates/roko-learn/src/structural.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct StructuralProposal {
       pub id: String,
       pub kind: ProposalKind,
       pub description: String,
       pub diff: ProposalDiff,
       pub evidence: Vec<SignalRef>,
       pub author: String,  // CellId or agent that generated the proposal
       pub confidence: f64,
       pub created_at: DateTime<Utc>,
       pub status: ProposalStatus,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ProposalKind {
       ModifyGraph { graph_id: String },
       AddCell { cell_manifest: Value },
       RemoveCell { cell_id: String },
       ChangeConfig { config_path: String },
       UpdateVerifyPipeline { changes: Vec<VerifyChange> },
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum ProposalStatus {
       Pending,
       Approved { by: String, at: DateTime<Utc> },
       Rejected { by: String, reason: String, at: DateTime<Utc> },
       Applied { at: DateTime<Utc> },
       Archived,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ProposalDiff {
       pub before: Option<String>,
       pub after: String,
       pub file_path: Option<String>,
   }

   pub type SignalRef = String;  // Reference to a Signal by hash
   ```

3. Implement a ProposalStore:
   ```rust
   pub struct ProposalStore {
       proposals: Vec<StructuralProposal>,
       persistence_path: PathBuf,
   }

   impl ProposalStore {
       pub fn new(path: PathBuf) -> Self;
       pub fn submit(&mut self, proposal: StructuralProposal) -> Result<()>;
       pub fn pending(&self) -> Vec<&StructuralProposal>;
       pub fn approve(&mut self, id: &str, by: &str) -> Result<()>;
       pub fn reject(&mut self, id: &str, by: &str, reason: &str) -> Result<()>;
       pub fn apply(&mut self, id: &str) -> Result<&StructuralProposal>;
       pub fn save(&self) -> Result<()>;
       pub fn load(path: &Path) -> Result<Self>;
   }
   ```

4. Proposals are published as Signals on Bus topic `structural.proposal.submitted`.

5. Persist proposals to `.roko/learn/structural-proposals.json`.

6. Write tests:
   - StructuralProposal serializes/deserializes correctly
   - Submit -> Approve -> Apply lifecycle works
   - Submit -> Reject -> Archive lifecycle works
   - Pending filter returns only Pending proposals

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- structural
```

## What NOT to do
- Do NOT implement automatic approval -- all L4 proposals require human approval
- Do NOT implement the proposal generation logic -- that is M074 (dream integration)
- Do NOT apply proposals without approval -- the store enforces the lifecycle
- Do NOT implement the Inbox integration -- that is M073
