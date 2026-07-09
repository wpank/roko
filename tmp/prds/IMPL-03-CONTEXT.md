# IMPL-03: Context engineering

Implements PRD-04. Target crates: `roko-compose`, `roko-learn`.

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. It builds
agents that build themselves: read PRDs, generate plans, execute tasks via Claude agents,
validate with gates, and persist results.

This plan adds a learnable context assembly system. Today, prompt assembly is static --
sections are included based on fixed priorities. After this work, sections compete for token
budget via a VCG auction, their effectiveness is tracked per-role, allocation evolves over
time through Bayesian feedback, and the assembly output is cached for repeated access.

The result: agents get better context over time because the system learns which sections
help which roles, and stops wasting tokens on sections that do not contribute.

### Key existing code

| Component | File | What exists |
|-----------|------|-------------|
| VCG auction | `crates/roko-compose/src/auction.rs` | `vcg_allocate()`, `LearningBidder`, `VcgBid`, `AuctionDiagnostics`, `FairnessConfig`. 570+ lines. |
| Attention bidders | `crates/roko-compose/src/attention.rs` | `PositionAttentionModel` (U-shaped), `dynamic_placement()`, `ModelAttentionCurves`. |
| Prompt sections | `crates/roko-compose/src/prompt.rs` | `PromptSection`, `SectionPriority` (Low/Normal/High/Critical), `CacheLayer` (Role/Workspace/Plan/Volatile), `Placement` (Start/Middle/End), `AttentionBidder` enum (8 variants), `PromptComposer`. |
| Section effects | `crates/roko-learn/src/section_effect.rs` | `SectionEffect` (inclusion/exclusion tracking), `SectionEffectivenessRegistry`, lift computation, priority change recommendations. |
| C-factor | `crates/roko-learn/src/cfactor.rs` | `CFactor`, `CFactorComponents`, `AgentDispatchBias`, `CollectivePathology`. |
| System prompt builder | `crates/roko-compose/src/system_prompt_builder.rs` | 9-layer `SystemPromptBuilder` with section-effectiveness integration. |
| Budget predictor | `crates/roko-compose/src/budget_predictor.rs` | `BudgetPredictor` (EMA per feature key), `SectionInfluence` (leave-one-out). |
| Subsystem IDs | `crates/roko-compose/src/auction.rs` line 11 | `SubsystemId` = `AttentionBidder` (Neuro, Daimon, IterationMemory, CodeIntelligence, PlaybookRules, Research, TaskContext, Oracles). |

### Existing `AttentionBidder` variants

From `crates/roko-compose/src/prompt.rs` lines 80-98:

| Variant | What it represents |
|---------|-------------------|
| `Neuro` | Durable knowledge from the neuro store |
| `Daimon` | Affect/somatic guidance |
| `IterationMemory` | Recent turns, retries, prior outputs |
| `CodeIntelligence` | Symbols, files, workspace structure |
| `PlaybookRules` | Skills, playbooks, distilled rules |
| `Research` | Research memos, external domain context |
| `TaskContext` | Task brief, plan brief, PRD slices |
| `Oracles` | Predictions, warnings, forecasts |

### Existing `LearningBidder`

From `crates/roko-compose/src/auction.rs` lines 30-83:

The `LearningBidder` already implements Thompson-style Beta-posterior bidding per section.
It has `bid()` and `update()` methods. Each bidder has a `subsystem_id: SubsystemId` and
a `section_betas: HashMap<String, (f64, f64)>` (alpha/beta parameters).

---

## Phase 1: `CognitiveWorkspace` type

**Goal**: Define the central data structure that represents a fully assembled context window.
Everything downstream (auction, policy, cache) operates on this type.

### Task 1.1: Define `ContextCategory` enum

**File**: `crates/roko-compose/src/workspace.rs` (new file)

**Read first**:
- `crates/roko-compose/src/prompt.rs` lines 78-98 (`AttentionBidder` enum)
- `crates/roko-compose/src/prompt.rs` lines 26-75 (`SectionPriority`, `CacheLayer`, `Placement`)

**Do**:
1. Create `crates/roko-compose/src/workspace.rs`
2. Define `ContextCategory` with 18+ variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCategory {
    /// Role identity and constraints (layer 1).
    RoleIdentity,
    /// Project coding conventions (layer 2).
    Conventions,
    /// Domain-specific knowledge (layer 3).
    DomainKnowledge,
    /// Active pheromone/stigmergic signals (layer 3c).
    Pheromones,
    /// Current task brief and acceptance criteria (layer 4).
    TaskBrief,
    /// Plan-level context and task dependencies.
    PlanContext,
    /// Tool definitions and usage instructions (layer 5).
    ToolInstructions,
    /// Learned playbooks and skills (layer 6).
    Playbooks,
    /// Tool usage hints from profiles (layer 6b).
    ToolHints,
    /// Anti-patterns and prohibitions (layer 7).
    AntiPatterns,
    /// Affect/emotional guidance (layer 8).
    AffectGuidance,
    /// Durable knowledge entries from Neuro.
    Knowledge,
    /// Recent iteration memory (turns, retries).
    IterationMemory,
    /// Code intelligence (symbols, files, structure).
    CodeIntelligence,
    /// Research memos and citations.
    Research,
    /// Oracle predictions and forecasts.
    Predictions,
    /// Gate feedback from prior attempts.
    GateFeedback,
    /// PRD slices relevant to the current task.
    PrdSlice,
}
```

3. Implement `ContextCategory::default_priority(&self) -> SectionPriority`:
   - `RoleIdentity`, `AntiPatterns` -> `Critical`
   - `TaskBrief`, `GateFeedback` -> `High`
   - `Conventions`, `ToolInstructions`, `Playbooks` -> `Normal`
   - Everything else -> `Low`

4. Implement `ContextCategory::default_cache_layer(&self) -> CacheLayer`:
   - `RoleIdentity`, `Conventions`, `ToolInstructions` -> `CacheLayer::Role`
   - `DomainKnowledge`, `Pheromones`, `PlanContext` -> `CacheLayer::Workspace`
   - `TaskBrief`, `Playbooks`, `AntiPatterns` -> `CacheLayer::Plan`
   - Everything else -> `CacheLayer::Volatile`

5. Implement `ContextCategory::default_placement(&self) -> Placement`:
   - `RoleIdentity`, `Conventions`, `ToolInstructions` -> `Placement::Start`
   - `TaskBrief`, `GateFeedback`, `AffectGuidance` -> `Placement::End`
   - Everything else -> `Placement::Middle`

6. Register the module in `crates/roko-compose/src/lib.rs`:
   - Add `pub mod workspace;`

**Test**: Unit test verifying:
- All 18 variants have non-panicking defaults
- `RoleIdentity` is `Critical` / `Role` / `Start`
- `TaskBrief` is `High` / `Plan` / `End`
- `Research` is `Low` / `Volatile` / `Middle`

- [ ] `ContextCategory` enum with 18 variants
- [ ] `default_priority()` maps each variant
- [ ] `default_cache_layer()` maps each variant
- [ ] `default_placement()` maps each variant
- [ ] Module registered in `lib.rs`
- [ ] Unit tests pass

---

### Task 1.2: Define `ContextSection` and `SectionSource`

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- Task 1.1 output
- `crates/roko-compose/src/prompt.rs` lines 100-136 (`PromptSection` struct)
- `crates/roko-compose/src/prompt.rs` lines 78-98 (`AttentionBidder`)

**Do**:
1. Define `SectionSource`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionSource {
    /// Static configuration (roko.toml, role templates).
    Config,
    /// Neuro durable knowledge store.
    Neuro,
    /// Daimon affect engine.
    Daimon,
    /// Code intelligence MCP.
    CodeIntel,
    /// Research agent output.
    Research,
    /// Playbook / skill library.
    PlaybookLibrary,
    /// Orchestrator (plan brief, task brief, gate feedback).
    Orchestrator,
    /// Oracle / prediction subsystem.
    Oracle,
    /// User-provided (manual context injection).
    User,
}
```

2. Define `ContextSection`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    /// Which category this section belongs to.
    pub category: ContextCategory,
    /// Which subsystem contributed this content.
    pub source: SectionSource,
    /// Priority for budget-pressure dropping.
    pub priority: SectionPriority,
    /// Where in the final prompt to place this section.
    pub placement: Placement,
    /// Cache layer for prefix-cache optimization.
    pub cache_layer: CacheLayer,
    /// The section's text content.
    pub content: String,
    /// Estimated token count (computed on construction).
    pub tokens: usize,
    /// Token allocation from the auction (0 if not yet allocated).
    pub allocation: usize,
    /// Human-readable label.
    pub label: String,
    /// Arbitrary metadata (role, domain, task_id, etc.).
    pub metadata: HashMap<String, String>,
}
```

3. Implement `ContextSection::new(category, source, label, content) -> Self`:
   - Auto-compute `tokens` using `estimate_tokens()` from `crates/roko-compose/src/prompt.rs` line 21
   - Set defaults from `category.default_priority()`, `category.default_cache_layer()`, `category.default_placement()`
   - Initialize `allocation = 0`, empty metadata

4. Implement `ContextSection::to_prompt_section(&self) -> PromptSection`:
   - Convert to the existing `PromptSection` type for compatibility with `PromptComposer`
   - Map `category` to `AttentionBidder`:
     - `Knowledge` -> `AttentionBidder::Neuro`
     - `AffectGuidance` -> `AttentionBidder::Daimon`
     - `IterationMemory` -> `AttentionBidder::IterationMemory`
     - `CodeIntelligence` -> `AttentionBidder::CodeIntelligence`
     - `Playbooks`, `ToolHints` -> `AttentionBidder::PlaybookRules`
     - `Research` -> `AttentionBidder::Research`
     - `TaskBrief`, `PlanContext`, `PrdSlice`, `GateFeedback` -> `AttentionBidder::TaskContext`
     - `Predictions` -> `AttentionBidder::Oracles`
     - Everything else -> `AttentionBidder::TaskContext`

**Test**:
- Construct a `ContextSection` for each category, verify token estimation
- Round-trip through `to_prompt_section()` and verify fields map correctly

- [ ] `SectionSource` enum with 9 variants
- [ ] `ContextSection` struct with all fields
- [ ] `new()` auto-computes tokens and sets defaults from category
- [ ] `to_prompt_section()` converts to the legacy type
- [ ] Unit tests pass

---

### Task 1.3: Define `CognitiveWorkspace`

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- Tasks 1.1, 1.2 output
- `crates/roko-primitives/src/tier.rs` lines 22-31 (`InferenceTier`)

**Do**:
1. Define `CognitiveWorkspace`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveWorkspace {
    /// Current inference tier driving this assembly.
    pub tier: InferenceTier,
    /// All sections in the workspace, in assembly order.
    pub sections: Vec<ContextSection>,
    /// Total token budget for this assembly.
    pub budget: usize,
    /// Tokens actually used after assembly.
    pub used_tokens: usize,
    /// Assembly log entries for debugging.
    pub assembly_log: Vec<AssemblyLogEntry>,
    /// Cache key for this workspace state (Blake3 hash).
    pub cache_key: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyLogEntry {
    /// What happened.
    pub action: String,
    /// Which section was affected.
    pub section_label: Option<String>,
    /// Token delta (positive = added, negative = removed).
    pub token_delta: i64,
    /// Reason for the action.
    pub reason: String,
}
```

2. Implement `CognitiveWorkspace::new(tier: InferenceTier, budget: usize) -> Self`

3. Implement `CognitiveWorkspace::add_section(&mut self, section: ContextSection)`:
   - Push to sections
   - Update `used_tokens`
   - Log the addition

4. Implement `CognitiveWorkspace::drop_lowest(&mut self)`:
   - Remove the lowest-priority, lowest-allocation section
   - Update `used_tokens`
   - Log the drop

5. Implement `CognitiveWorkspace::to_prompt_sections(&self) -> Vec<PromptSection>`:
   - Convert all sections via `to_prompt_section()`
   - Sort by `(cache_layer, placement, priority desc)`

6. Implement `CognitiveWorkspace::compute_cache_key(&mut self)`:
   - Hash the sorted `(category, content)` pairs using Blake3
   - Store in `self.cache_key`

**Dependency**: Add `blake3` to `crates/roko-compose/Cargo.toml`. Check if already present:
`grep blake3 /Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml`

**Test**:
- Create workspace, add 5 sections, verify `used_tokens` matches sum
- Drop lowest, verify it removes the right section
- `to_prompt_sections()` returns them in cache-layer order
- Cache key is deterministic: same sections in any order produce the same key

- [ ] `CognitiveWorkspace` struct defined
- [ ] `add_section()` tracks tokens and logs
- [ ] `drop_lowest()` removes the least valuable section
- [ ] `to_prompt_sections()` sorts by cache layer, placement, priority
- [ ] `compute_cache_key()` produces deterministic Blake3 hash
- [ ] `blake3` dependency added
- [ ] Unit tests pass

---

## Phase 2: VCG auction wiring

**Goal**: Wire the existing `vcg_allocate()` function to produce `CognitiveWorkspace`
instances. Create 8 bidder implementations (one per `AttentionBidder` variant) that wrap
the existing `LearningBidder`.

### Task 2.1: Define `ContextBidder` trait

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-compose/src/auction.rs` lines 30-83 (`LearningBidder`)
- `crates/roko-compose/src/auction.rs` lines 200-298 (`VcgBid` struct, pre-`vcg_allocate` types)
- Task 1.2 output (`ContextSection`)

**Do**:
1. Define the trait:

```rust
pub trait ContextBidder: Send + Sync {
    /// Return the subsystem ID this bidder represents.
    fn subsystem_id(&self) -> SubsystemId;

    /// Generate bids for sections this subsystem wants to include.
    /// Each bid is a (section, relevance_score) pair.
    fn bid(&self, task_context: &TaskBidContext) -> Vec<(ContextSection, f64)>;

    /// Update the bidder after an episode completes.
    fn on_episode(&mut self, sections_included: &[ContextCategory], gate_passed: bool);
}
```

2. Define `TaskBidContext`:

```rust
#[derive(Debug, Clone)]
pub struct TaskBidContext {
    /// Role of the agent being assembled.
    pub role: String,
    /// Task description.
    pub task_description: String,
    /// Domain of the current task.
    pub domain: String,
    /// Available token budget.
    pub budget: usize,
    /// Current inference tier.
    pub tier: InferenceTier,
    /// Current affect state (pleasure, arousal, dominance).
    pub affect: Option<(f64, f64, f64)>,
}
```

**Test**: Verify trait is object-safe:
```rust
let _: Box<dyn ContextBidder> = ...; // must compile
```

- [ ] `ContextBidder` trait defined with 3 methods
- [ ] `TaskBidContext` struct provides bidding context
- [ ] Trait is object-safe
- [ ] Unit test verifies object safety

---

### Task 2.2: Implement 8 bidder wrappers

**File**: `crates/roko-compose/src/bidders.rs` (new file)

**Read first**:
- Task 2.1 output (`ContextBidder` trait)
- `crates/roko-compose/src/auction.rs` lines 30-83 (`LearningBidder`)
- `crates/roko-compose/src/prompt.rs` lines 78-98 (`AttentionBidder` variants)

**Do**:
1. Create `crates/roko-compose/src/bidders.rs`
2. For each of the 8 `AttentionBidder` variants, create a struct that wraps `LearningBidder`
   and implements `ContextBidder`. The 8 bidders:

**NeuroBidder** (wraps `AttentionBidder::Neuro`):
- `bid()`: generates sections with `ContextCategory::Knowledge`
- Relevance = learned Beta posterior from `LearningBidder`
- Priority: adjusts based on task domain match

**DaimonBidder** (wraps `AttentionBidder::Daimon`):
- `bid()`: generates sections with `ContextCategory::AffectGuidance`
- Relevance = higher when affect state is far from neutral
- Only bids when `task_context.affect` is `Some`

**IterationMemoryBidder** (wraps `AttentionBidder::IterationMemory`):
- `bid()`: generates sections with `ContextCategory::IterationMemory`
- Relevance = increases with retry count

**CodeIntelBidder** (wraps `AttentionBidder::CodeIntelligence`):
- `bid()`: generates sections with `ContextCategory::CodeIntelligence`
- Relevance = higher for code-related tasks

**PlaybookBidder** (wraps `AttentionBidder::PlaybookRules`):
- `bid()`: generates sections with `ContextCategory::Playbooks` and `ContextCategory::ToolHints`
- Relevance = learned from past effectiveness

**ResearchBidder** (wraps `AttentionBidder::Research`):
- `bid()`: generates sections with `ContextCategory::Research`
- Relevance = higher for research domain tasks

**TaskContextBidder** (wraps `AttentionBidder::TaskContext`):
- `bid()`: generates sections with `ContextCategory::TaskBrief`, `PlanContext`, `PrdSlice`, `GateFeedback`
- TaskBrief always bids high (it is the task itself)

**OracleBidder** (wraps `AttentionBidder::Oracles`):
- `bid()`: generates sections with `ContextCategory::Predictions`
- Relevance = learned from past prediction accuracy

3. Each bidder constructor takes a `LearningBidder` and any subsystem-specific data
   (knowledge entries, affect state, playbooks, etc.)

4. Register in `crates/roko-compose/src/lib.rs`: `pub mod bidders;`

**Test**: For each bidder:
- Construct with mock data
- Call `bid()` with a `TaskBidContext`
- Verify it returns sections of the correct category
- Verify relevance scores are in [0.0, 1.0]

- [ ] 8 bidder structs created (Neuro, Daimon, IterationMemory, CodeIntel, Playbook, Research, TaskContext, Oracle)
- [ ] Each wraps `LearningBidder` and implements `ContextBidder`
- [ ] Each generates sections of the correct `ContextCategory`
- [ ] Module registered in `lib.rs`
- [ ] Unit tests for each bidder

---

### Task 2.3: Wire bidders into `vcg_allocate` and produce `CognitiveWorkspace`

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-compose/src/auction.rs` lines 290-350 (`vcg_allocate` function signature and logic)
- Tasks 2.1, 2.2 output
- Task 1.3 output (`CognitiveWorkspace`)

**Do**:
1. Implement `assemble_workspace`:

```rust
pub fn assemble_workspace(
    bidders: &[&dyn ContextBidder],
    task_context: &TaskBidContext,
    affect_modulation: &AffectModulation,
) -> CognitiveWorkspace
```

Logic:
1. Collect bids from all bidders: `bidder.bid(task_context)` -> `Vec<(ContextSection, f64)>`
2. Convert each `(section, relevance)` to a `VcgBid`:
   - `bidder` = `bidder.subsystem_id()`
   - `section_name` = `section.label.clone()`
   - `tokens` = `section.tokens`
   - `value` = `relevance * section.priority as f64` (priority multiplier)
   - `valence` = 0.0 (default, affect modulation handles this)
3. Call `vcg_allocate(bids, task_context.budget, affect_modulation)`
4. Build `CognitiveWorkspace`:
   - For each winner in the allocation, find the original `ContextSection`
   - Set `section.allocation = winner.allocated_tokens`
   - Add to workspace
5. Sort workspace sections by `(cache_layer, placement, priority desc)`
6. Compute cache key
7. Log assembly stats

2. Add critical-section bypass:
   - Before auction, extract all `Critical` priority sections
   - Subtract their tokens from the budget
   - Add them directly to the workspace (they skip the auction)
   - Remaining sections compete for the remaining budget

**Test**:
- 3 bidders submit 6 total bids, budget = 1000 tokens
- All Critical sections are included regardless of auction
- VCG allocation respects budget: `used_tokens <= budget`
- Higher-value bids win over lower-value bids
- Assembly log records the decisions

- [ ] `assemble_workspace` collects bids, runs VCG, builds workspace
- [ ] Critical sections bypass the auction
- [ ] Budget is respected
- [ ] Assembly log captures decisions
- [ ] Unit tests pass

---

### Task 2.4: Integration test for auction-driven assembly

**File**: `crates/roko-compose/tests/workspace_integration.rs` (new file)

**Read first**:
- Tasks 2.1, 2.2, 2.3 output

**Do**:
1. Create integration test file
2. Scenarios:

**Scenario A: Budget pressure**
- 5 bidders, total demand 2000 tokens, budget 800
- Assert: only highest-value sections included
- Assert: `used_tokens <= 800`

**Scenario B: All sections fit**
- 3 bidders, total demand 500 tokens, budget 2000
- Assert: all sections included
- Assert: no sections dropped

**Scenario C: Critical section guarantee**
- 1 Critical section (200 tokens) + 4 Normal sections (250 tokens each)
- Budget = 600
- Assert: Critical section always included
- Assert: only 1-2 Normal sections included (400 remaining budget)

3. Run: `cargo test -p roko-compose --test workspace_integration`

- [ ] Scenario A: budget pressure drops low-value sections
- [ ] Scenario B: all sections fit when budget allows
- [ ] Scenario C: Critical sections always survive
- [ ] Integration tests pass

---

## Phase 3: `ContextPolicy` (learnable allocation)

**Goal**: Build a policy that evolves section allocations over time using Bayesian feedback
from episode outcomes. Sections that correlate with gate passes get more budget. Sections
that correlate with failures get less.

### Task 3.1: Define `ContextPolicy`

**File**: `crates/roko-compose/src/context_policy.rs` (new file)

**Read first**:
- `crates/roko-compose/src/auction.rs` lines 30-83 (`LearningBidder` for Beta-posterior pattern)
- `crates/roko-learn/src/section_effect.rs` lines 29-110 (`SectionEffect` for lift measurement)
- Task 1.1 output (`ContextCategory` enum)

**Do**:
1. Create `crates/roko-compose/src/context_policy.rs`
2. Define `ContextPolicy`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicy {
    /// Policy revision number. Incremented on each evolution step.
    pub revision: u64,
    /// Base allocation fraction per category. Sums to 1.0.
    pub allocations: HashMap<ContextCategory, f64>,
    /// Beta-distribution feedback per category: (alpha, beta).
    /// Alpha tracks successes, beta tracks failures.
    pub feedback: HashMap<ContextCategory, (f64, f64)>,
    /// Number of episodes observed since last evolution.
    pub episodes_since_evolve: u64,
    /// Evolution interval (default: 50 episodes).
    pub evolve_interval: u64,
}
```

3. Implement `ContextPolicy::default_allocations() -> HashMap<ContextCategory, f64>`:
   - `TaskBrief`: 0.20
   - `RoleIdentity`: 0.08
   - `Conventions`: 0.05
   - `Knowledge`: 0.12
   - `CodeIntelligence`: 0.10
   - `Playbooks`: 0.08
   - `IterationMemory`: 0.10
   - `Research`: 0.07
   - `GateFeedback`: 0.06
   - `AffectGuidance`: 0.03
   - `Predictions`: 0.03
   - Remaining categories share the remaining 0.08

4. Implement Loop 1: `ContextPolicy::record_episode(&mut self, sections_used: &[ContextCategory], gate_passed: bool)`:
   - For each used category: if passed, increment alpha; if failed, increment beta
   - Increment `episodes_since_evolve`
   - If `episodes_since_evolve >= evolve_interval`, call `evolve()`

5. Implement Loop 2: `ContextPolicy::evolve(&mut self)`:
   - For each category, compute posterior mean: `alpha / (alpha + beta)`
   - Adjust allocation: `new_alloc = base_alloc * posterior_mean / mean_of_all_posteriors`
   - Normalize allocations to sum to 1.0
   - Reset `episodes_since_evolve = 0`
   - Increment `revision`

6. Implement `ContextPolicy::allocation_for(&self, category: ContextCategory, total_budget: usize) -> usize`:
   - Return `(self.allocations[category] * total_budget as f64) as usize`

7. Implement persistence: `save(&self, path: &Path)` and `load(path: &Path)`:
   - JSON to `.roko/learn/context-policy.json`

8. Register in `crates/roko-compose/src/lib.rs`: `pub mod context_policy;`

**Test**:
- Initial allocations sum to 1.0
- After 50 episodes where `Knowledge` always passes and `Predictions` always fails:
  - `Knowledge` allocation increases
  - `Predictions` allocation decreases
- After 100 episodes, the shift is more pronounced
- Persistence round-trip preserves allocations and feedback

- [ ] `ContextPolicy` struct with allocations and Beta feedback
- [ ] Default allocations sum to 1.0 and weight TaskBrief highest
- [ ] Loop 1: episode outcomes update Beta posteriors
- [ ] Loop 2: evolution adjusts allocations from posteriors every 50 episodes
- [ ] `allocation_for()` returns correct token count
- [ ] Persistence round-trip works
- [ ] Module registered in `lib.rs`
- [ ] Unit tests pass

---

### Task 3.2: Wire policy into workspace assembly

**File**: `crates/roko-compose/src/workspace.rs` (modify `assemble_workspace`)

**Read first**:
- Task 3.1 output
- Task 2.3 output (`assemble_workspace`)

**Do**:
1. Add `policy: Option<&ContextPolicy>` parameter to `assemble_workspace`
2. When a policy is present:
   - Before VCG auction, compute per-category budget caps from `policy.allocation_for()`
   - Pass these caps as hard ceilings on per-section `VcgBid.tokens`
   - Sections that exceed their category cap are truncated (head-preserve)
3. After assembly, record which categories were included in the workspace
   - Return this info so the caller can feed it to `policy.record_episode()`

**Test**:
- Policy allocates 30% to TaskBrief, 10% to Research, budget = 1000
- TaskBrief sections get up to 300 tokens, Research up to 100
- Sections exceeding their cap are truncated

- [ ] `assemble_workspace` accepts optional `ContextPolicy`
- [ ] Policy caps are enforced as per-category ceilings
- [ ] Over-cap sections are truncated with head preservation
- [ ] Categories included are reported for feedback loop
- [ ] Unit tests pass

---

### Task 3.3: Integration test for policy evolution

**File**: `crates/roko-compose/tests/workspace_integration.rs` (append)

**Read first**:
- Tasks 3.1, 3.2 output

**Do**:
1. Scenario D: Policy evolution over 100 episodes
   - Create a `ContextPolicy` with default allocations
   - Simulate 100 episodes:
     - Episodes with `Knowledge` sections pass 90% of the time
     - Episodes with `Predictions` sections pass 30% of the time
     - Episodes with `Research` sections pass 60% of the time
   - Call `record_episode()` for each
   - Assert: after evolution, `Knowledge` allocation > initial
   - Assert: after evolution, `Predictions` allocation < initial
   - Assert: allocations still sum to 1.0

2. Scenario E: Cold start (no history)
   - Policy with default allocations
   - Assert: `allocation_for()` returns default proportions

3. Run: `cargo test -p roko-compose --test workspace_integration`

- [ ] Scenario D: policy shifts allocations toward high-success categories
- [ ] Scenario E: cold start returns defaults
- [ ] Integration tests pass

---

## Phase 4: Section effect tracking

**Goal**: Wire the existing `SectionEffectivenessRegistry` into the `CognitiveWorkspace`
so that per-section lift is tracked and influences future assemblies.

### Task 4.1: Wire section effects into workspace

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-learn/src/section_effect.rs` lines 112-200 (`SectionEffectivenessRegistry`)
- `crates/roko-learn/src/section_effect.rs` lines 72-110 (`lift()`, `lift_weight()`, `recommend_priority_change()`)
- Task 2.3 output (`assemble_workspace`)

**Do**:
1. Add `section_registry: Option<&SectionEffectivenessRegistry>` parameter to `assemble_workspace`
2. Before VCG auction, adjust bid values using lift weights:
   - For each bid, look up `registry.get(section_label, role)`
   - Multiply the bid value by `effect.lift_weight()` (range 0.5-1.5)
   - This boosts sections with proven positive lift and penalizes sections with negative lift
3. After assembly, record outcomes:
   - Return a list of `(section_label, included: bool)` so the caller can call
     `registry.record_outcome(label, role, included, gate_passed)` after the gate runs

**Test**:
- Section with lift_weight 1.5 wins over section with lift_weight 0.7 (same base value)
- Section with insufficient data (lift_weight 1.0) competes at base value
- Priority change recommendation flows through correctly

- [ ] `assemble_workspace` accepts optional `SectionEffectivenessRegistry`
- [ ] Bid values adjusted by lift weights
- [ ] Included/excluded sections reported for feedback
- [ ] Unit tests pass

---

### Task 4.2: Track lift per category per domain

**File**: `crates/roko-learn/src/section_effect.rs` (modify)

**Read first**:
- `crates/roko-learn/src/section_effect.rs` (full file)
- Task 1.1 output (`ContextCategory`)

**Do**:
1. Add domain scoping to `SectionEffectivenessRegistry`:
   - Current key: `(section_name, role)`
   - New key: `(section_name, role, domain)`
   - Implement `record_outcome_with_domain(&mut self, section, role, domain, included, passed)`
   - Implement `get_with_domain(&self, section, role, domain) -> Option<&SectionEffect>`
2. Keep backwards compatibility: `record_outcome()` calls `record_outcome_with_domain()` with `domain = "default"`
3. Add `lift_for_domain(&self, section, role, domain) -> f64`:
   - Returns lift for the specific domain, or falls back to the all-domain lift
4. Update `save()` / `load_or_new()` for the new key structure:
   - Serialize domain as the third field in the compound key

**Test**:
- Record outcomes for "workspace_map" in "coding" domain -> high lift
- Record outcomes for "workspace_map" in "research" domain -> low lift
- `lift_for_domain("workspace_map", "implementer", "coding")` returns high lift
- `lift_for_domain("workspace_map", "implementer", "research")` returns low lift
- Backward compatibility: old-style `record_outcome()` still works

- [ ] Domain scoping added to `SectionEffectivenessRegistry`
- [ ] `record_outcome_with_domain()` tracks per-domain effects
- [ ] `lift_for_domain()` returns domain-specific lift with fallback
- [ ] Backward compatibility preserved
- [ ] Persistence updated for new key structure
- [ ] Unit tests pass

---

### Task 4.3: Integration test for section effects

**File**: `crates/roko-compose/tests/workspace_integration.rs` (append)

**Read first**:
- Tasks 4.1, 4.2 output

**Do**:
1. Scenario F: Lift-weighted assembly
   - Registry where "knowledge" has lift 0.3 and "research" has lift -0.1
   - Budget allows only one of the two
   - Assert: "knowledge" wins because lift_weight (1.3) > research lift_weight (0.9)

2. Scenario G: Domain-specific lift
   - "workspace_map" has positive lift in "coding" but negative lift in "research"
   - For a coding task: "workspace_map" included
   - For a research task: "workspace_map" excluded (or lower priority)

3. Run: `cargo test -p roko-compose --test workspace_integration`

- [ ] Scenario F: lift weights influence auction outcome
- [ ] Scenario G: domain-specific lift produces different assembly per domain
- [ ] Integration tests pass

---

## Phase 5: Cache architecture

**Goal**: Add multi-tier caching so that repeated assemblies with the same inputs are fast.

### Task 5.1: Implement L0-L3 cache tiers

**File**: `crates/roko-compose/src/workspace_cache.rs` (new file)

**Read first**:
- Task 1.3 output (`CognitiveWorkspace`, `compute_cache_key`)
- `crates/roko-compose/src/prompt.rs` lines 42-58 (`CacheLayer` enum)

**Do**:
1. Create `crates/roko-compose/src/workspace_cache.rs`
2. Define cache tiers:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    /// L0: In-memory, per-session. Evicts on session end.
    L0Session,
    /// L1: In-memory, cross-session within a plan run. Evicts on plan completion.
    L1Plan,
    /// L2: On-disk, persisted. Evicts by LRU after capacity.
    L2Disk,
    /// L3: Derived from workspace state hash. Immutable reference.
    L3Immutable,
}
```

3. Define `WorkspaceCache`:

```rust
#[derive(Debug)]
pub struct WorkspaceCache {
    /// L0: session-local cache. Key = cache_key, Value = assembled workspace.
    l0: HashMap<[u8; 32], CognitiveWorkspace>,
    /// L1: plan-scoped cache.
    l1: HashMap<[u8; 32], CognitiveWorkspace>,
    /// L2: disk-backed LRU cache.
    l2_dir: PathBuf,
    /// L2 capacity (number of entries).
    l2_capacity: usize,
    /// Cache hit/miss statistics.
    pub stats: CacheStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub l0_hits: u64,
    pub l0_misses: u64,
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
}
```

4. Implement `WorkspaceCache::get(&mut self, key: &[u8; 32]) -> Option<CognitiveWorkspace>`:
   - Check L0, then L1, then L2
   - On L2 hit, promote to L0
   - Track stats

5. Implement `WorkspaceCache::put(&mut self, workspace: CognitiveWorkspace)`:
   - Requires `workspace.cache_key` to be set
   - Insert into L0
   - If the workspace's cache layer is `Role` or `Workspace`, also insert into L1

6. Implement `WorkspaceCache::flush_l0(&mut self)`:
   - Clear L0 (called at session end)
   - Promote L0 entries to L1 if they had stable cache layers

7. Implement L2 disk persistence:
   - `save_to_l2(&self, key: &[u8; 32], workspace: &CognitiveWorkspace)` -> write JSON to `l2_dir/{hex_key}.json`
   - `load_from_l2(&self, key: &[u8; 32])` -> read JSON from `l2_dir/{hex_key}.json`
   - LRU eviction: when L2 entries exceed `l2_capacity`, delete oldest by mtime

8. Register in `crates/roko-compose/src/lib.rs`: `pub mod workspace_cache;`

**Test**:
- Put a workspace, get it back from L0 -> hit
- Flush L0, get it from L1 -> hit
- Completely new key -> miss at all levels
- L2 round-trip: put, flush L0+L1, get from L2
- Cache stats track correctly

- [ ] `WorkspaceCache` with L0/L1/L2 tiers
- [ ] `get()` checks tiers in order with promotion
- [ ] `put()` inserts into appropriate tiers
- [ ] `flush_l0()` promotes stable entries to L1
- [ ] L2 disk persistence works
- [ ] LRU eviction enforces capacity
- [ ] Cache stats tracked
- [ ] Module registered in `lib.rs`
- [ ] Unit tests pass

---

### Task 5.2: Deterministic cache key computation

**File**: `crates/roko-compose/src/workspace.rs` (modify `compute_cache_key`)

**Read first**:
- Task 1.3 output (`compute_cache_key`)
- Task 5.1 output (cache tiers)

**Do**:
1. Ensure deterministic key computation:
   - Sort sections by `(category, label)` before hashing
   - Use `BTreeMap` for any metadata included in the hash
   - Hash the tier, budget, and sorted section `(category, content_hash)` tuples
   - Use Blake3 for the hash

2. Add `content_hash` field to `ContextSection`:
   - Computed on construction: `Blake3::hash(content.as_bytes())`
   - Avoids rehashing large content strings during cache key computation

3. Verify: two workspaces with the same sections in different insertion order produce
   the same cache key.

**Test**:
- Same sections, different order -> same cache key
- Same sections, different content -> different cache key
- Same sections, different budget -> different cache key
- Same sections, different tier -> different cache key

- [ ] Cache keys are deterministic regardless of insertion order
- [ ] `content_hash` field added to `ContextSection`
- [ ] Tier and budget included in cache key
- [ ] Unit tests verify all determinism invariants

---

### Task 5.3: Integration test for caching

**File**: `crates/roko-compose/tests/workspace_integration.rs` (append)

**Read first**:
- Tasks 5.1, 5.2 output

**Do**:
1. Scenario H: Cache hit reduces assembly time
   - Assemble a workspace (measure wall time)
   - Put it in cache
   - Request the same workspace (cache hit)
   - Assert: second request is at least 2x faster (no auction, no bidding)

2. Scenario I: Cache invalidation on section change
   - Assemble workspace A, cache it
   - Change one section's content
   - Assert: new cache key differs, cache miss

3. Scenario J: L2 disk round-trip
   - Create workspace, put in cache
   - Flush L0 and L1
   - Get from L2
   - Assert: workspace matches original

4. Run: `cargo test -p roko-compose --test workspace_integration`

- [ ] Scenario H: cache hit is faster than fresh assembly
- [ ] Scenario I: content change invalidates cache
- [ ] Scenario J: L2 disk round-trip works
- [ ] Integration tests pass

---

## Phase 6: Advanced features

**Goal**: Add the remaining advanced context engineering features that improve assembly
quality beyond the base auction+policy system.

### Task 6.1: U-shaped placement

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-compose/src/attention.rs` lines 14-55 (`PositionAttentionModel`, U-shaped curve)
- `crates/roko-compose/src/attention.rs` lines 84-124 (`placement_adjusted_score`, `dynamic_placement`)
- Task 1.3 output (`CognitiveWorkspace`)

**Do**:
1. Implement `CognitiveWorkspace::apply_u_placement(&mut self)`:
   - Rank sections by effective value (auction value * lift weight)
   - Assign top 1/3 to `Placement::Start`
   - Assign bottom 1/3 to `Placement::End`
   - Middle 1/3 stays at `Placement::Middle`
   - Critical sections keep their placement unchanged

2. Integrate with `assemble_workspace`: call `apply_u_placement()` after auction
   and before sorting.

**Test**:
- Highest-value section placed at Start
- Second-highest placed at End
- Middle-value section placed at Middle
- Critical section retains its original placement

- [ ] `apply_u_placement()` assigns high-value sections to attention peaks
- [ ] Critical sections are not moved
- [ ] Unit tests pass

---

### Task 6.2: Complexity-based token scaling

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-compose/src/budget_predictor.rs` lines 88-173 (`BudgetPredictor`)
- Task 1.3 output (`CognitiveWorkspace`)

**Do**:
1. Implement `fn complexity_scaled_budget(base_budget: usize, complexity: TaskComplexity) -> usize`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    Trivial,    // 4K context
    Standard,   // 16K context
    Complex,    // 40K context
    Frontier,   // 100K+ context
}
```

Mapping:
- `Trivial` -> `base_budget.min(4_000)`
- `Standard` -> `base_budget.min(16_000)`
- `Complex` -> `base_budget.min(40_000)`
- `Frontier` -> `base_budget` (no cap)

2. Integrate into `assemble_workspace`: if `task_context` includes complexity, apply scaling
   before the auction.

**Test**:
- Trivial task with 100K budget -> capped at 4K
- Standard task with 100K budget -> capped at 16K
- Complex task with 20K budget -> gets full 20K (under cap)
- Frontier task -> no cap

- [ ] `TaskComplexity` enum defined
- [ ] `complexity_scaled_budget()` caps budget per complexity tier
- [ ] Integration with `assemble_workspace`
- [ ] Unit tests pass

---

### Task 6.3: Affect-modulated allocation

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-compose/src/auction.rs` lines 200-260 (`AffectModulation` struct)
- `crates/roko-daimon/src/lib.rs` lines 310-340 (`AffectState`)
- Task 2.3 output (`assemble_workspace`)

**Do**:
1. Implement `fn affect_modulation_from_state(affect: &(f64, f64, f64)) -> AffectModulation`:
   - Convert PAD tuple to `AffectModulation`:
   - High arousal (> 0.5) + low pleasure (< -0.3) = "stressed":
     - Boost `AntiPatterns`, `GateFeedback` categories by 1.3x
     - Reduce `Research`, `Predictions` by 0.7x
   - High pleasure (> 0.3) + high dominance (> 0.3) = "confident":
     - Boost `Research`, `Predictions` by 1.2x
     - Reduce `AntiPatterns` by 0.8x
   - Neutral: no modulation

2. Wire into `assemble_workspace`: compute `AffectModulation` from `task_context.affect`
   and pass to `vcg_allocate`.

**Test**:
- Stressed state -> AntiPatterns gets more budget, Research gets less
- Confident state -> Research gets more budget, AntiPatterns gets less
- Neutral state -> no modulation

- [ ] `affect_modulation_from_state()` converts PAD to modulation
- [ ] Stressed state boosts safety-related categories
- [ ] Confident state boosts exploration-related categories
- [ ] Unit tests pass

---

### Task 6.4: HDC-based retrieval for knowledge categories

**File**: `crates/roko-compose/src/workspace.rs` (append)

**Read first**:
- `crates/roko-primitives/src/hdc.rs` lines 83-150 (HDC vector operations: random, bind, bundle, Hamming)
- Task 2.2 output (`NeuroBidder`)

**Do**:
1. Implement `fn hdc_relevance(task_fingerprint: &HdcVector, entry_fingerprint: &HdcVector) -> f64`:
   - Compute Hamming similarity: `1.0 - (hamming_distance / HDC_BITS as f64)`
   - This gives a [0, 1] relevance score

2. Wire into `NeuroBidder::bid()`:
   - If knowledge entries have HDC fingerprints, use `hdc_relevance` to rank them
   - Use the relevance score as the bid's relevance parameter
   - Entries with similarity > 0.6 get a boost

3. Wire into `PlaybookBidder::bid()`:
   - Same pattern: playbooks with HDC fingerprints are ranked by similarity to the task

**Dependency**: `roko-primitives` should already be a dependency of `roko-compose`. Verify:
`grep roko-primitives /Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml`

**Test**:
- Same HDC vector -> similarity 1.0
- Random HDC vectors -> similarity ~0.5
- Orthogonal vectors -> similarity ~0.5
- Bound vectors -> lower similarity than originals

- [ ] `hdc_relevance()` computes Hamming similarity
- [ ] `NeuroBidder` uses HDC relevance for knowledge ranking
- [ ] `PlaybookBidder` uses HDC relevance for playbook ranking
- [ ] Unit tests verify similarity ranges

---

### Task 6.5: Final integration test

**File**: `crates/roko-compose/tests/workspace_integration.rs` (append)

**Read first**:
- All phase outputs

**Do**:
1. Scenario K: End-to-end learnable context
   - Create all 8 bidders with mock data
   - Create a `ContextPolicy` with default allocations
   - Create a `SectionEffectivenessRegistry` with some pre-existing lift data
   - Assemble 50 workspaces with evolving policy:
     - After each assembly, simulate a gate outcome
     - Feed outcome to policy and registry
   - Assert: final allocations differ from initial (learning happened)
   - Assert: sections with positive lift appear more often
   - Assert: all workspaces respect budget constraints

2. Scenario L: Lift comparison (static vs learnable)
   - Assemble 100 workspaces with static allocation (no policy)
   - Assemble 100 workspaces with learnable policy
   - Simulate gate outcomes where Knowledge sections help 80% of the time
   - Assert: learnable policy allocates more to Knowledge by episode 100
   - Assert: difference is >= 5% (measurable lift)

3. Run: `cargo test -p roko-compose --test workspace_integration`

- [ ] Scenario K: end-to-end assembly with all components
- [ ] Scenario L: learnable context shows measurable lift vs static
- [ ] Integration tests pass

---

## Phase 7: InsightStore integration

**Goal**: Wire the context assembly system to query the Korai InsightStore (on-chain knowledge) via the HTC precompile, adding chain-sourced knowledge entries to agent prompts when the task domain warrants it.

### Task 7.1: Wire NeuroContextBidder to query Korai InsightStore via HTC precompile

**File to modify:** `crates/roko-compose/src/bidders.rs`

**Read first:**
- `crates/roko-compose/src/bidders.rs` -- existing `NeuroContextBidder` (or equivalent knowledge bidder from Phase 1)
- `crates/roko-chain/src/insight_store.rs` -- `InsightStore` trait, `InsightEntry`, `InsightQuery` (from IMPL-07 Task 4.4)
- `crates/roko-chain/src/precompiles/htc.rs` -- HTC precompile for similarity search (from IMPL-07 Task 3.6)
- `crates/roko-neuro/src/knowledge_store.rs` -- local `KnowledgeStore` for comparison

**What to do:**

1. Extend the `NeuroContextBidder` (or create `InsightStoreBidder` if it does not exist) to accept an optional `InsightStore` client:

```rust
pub struct InsightStoreBidder {
    local_store: KnowledgeStore,
    chain_store: Option<Box<dyn InsightStore>>,
    chain_weight: f64,  // default 0.8 -- chain entries are slightly less trusted
}
```

2. In the `bid()` method, query both stores:

```rust
impl ContextBidder for InsightStoreBidder {
    fn bid(&self, task: &TaskContext, budget: &TokenBudget) -> Vec<ContextSection> {
        let mut sections = Vec::new();

        // Local knowledge
        let local = self.local_store.query(&task.title, 3).unwrap_or_default();
        for entry in local {
            sections.push(ContextSection::new(entry.content.clone())
                .with_source(format!("neuro:{}", entry.id))
                .with_priority(SectionPriority::Knowledge)
                .with_bid_value(entry.confidence));
        }

        // Chain-sourced knowledge (InsightStore via HTC precompile)
        if let Some(ref store) = self.chain_store {
            let fingerprint = text_fingerprint(&task.title);
            if let Ok(chain_entries) = tokio::runtime::Handle::current()
                .block_on(store.query_similar(&fingerprint, 3))
            {
                for entry in chain_entries {
                    let bid = entry.similarity as f64 * self.chain_weight;
                    sections.push(ContextSection::new(format!(
                        "[chain:{}@block {}] {}",
                        entry.source_chain_id, entry.block_number,
                        hex::encode(&entry.content_hash[..8])
                    ))
                    .with_source(format!("insight_store:{}", hex::encode(&entry.content_hash)))
                    .with_priority(SectionPriority::Knowledge)
                    .with_bid_value(bid));
                }
            }
        }

        sections
    }
}
```

3. Graceful degradation: if `chain_store` is `None` or the query fails, return local-only results without error.

**Files to modify:**
- `crates/roko-compose/src/bidders.rs`
- `crates/roko-compose/Cargo.toml` (add optional `roko-chain` dependency behind `chain` feature)

**Test:**
- With mock InsightStore returning 2 entries: assert 2 chain-sourced sections in output with correct attribution.
- With `chain_store = None`: assert only local sections returned, no error.
- With InsightStore returning error: assert graceful fallback to local-only.

- [ ] `InsightStoreBidder` queries both local and chain knowledge stores
- [ ] Chain entries attributed with `insight_store:` source prefix
- [ ] Graceful degradation when chain is unavailable
- [ ] Feature-gated `roko-chain` dependency

---

### Task 7.2: Implement reputation-weighted scoring for chain-sourced entries

**File to modify:** `crates/roko-compose/src/bidders.rs`

**Read first:**
- Task 7.1 output
- `crates/roko-chain/src/reputation_registry.rs` -- 7-domain EMA scoring
- `crates/roko-chain/src/insight_store/mod.rs` -- `InsightEntry::quality_score()` (from IMPL-07 Task 4.3)

**What to do:**

1. Extend `InsightStoreBidder` with a reputation query:

```rust
pub struct InsightStoreBidder {
    local_store: KnowledgeStore,
    chain_store: Option<Box<dyn InsightStore>>,
    reputation: Option<Box<dyn ReputationQuery>>,
    chain_weight: f64,
}

#[async_trait]
pub trait ReputationQuery: Send + Sync {
    async fn agent_reputation(&self, address: &str, domain: &str) -> Result<f64>;
}
```

2. When computing bid values for chain-sourced entries, scale by contributor reputation:

```rust
let rep = self.reputation.as_ref()
    .and_then(|r| tokio::runtime::Handle::current()
        .block_on(r.agent_reputation(&entry.contributor, &task.domain))
        .ok())
    .unwrap_or(0.5);  // default to neutral if no reputation data

let bid = entry.similarity as f64 * self.chain_weight * rep;
```

3. Low-reputation entries (rep < 0.3) are excluded entirely to prevent knowledge pollution.

**Test:**
- Entry from high-reputation agent (0.9): bid value ~0.72 (0.9 * 0.8 * 1.0 similarity).
- Entry from low-reputation agent (0.2): excluded (rep < 0.3 threshold).
- No reputation available: default to 0.5 weight.

- [ ] Bid values scaled by contributor reputation
- [ ] Low-reputation entries excluded (rep < 0.3)
- [ ] Default to neutral reputation when unavailable

---

### Task 7.3: Implement CausalLink composition

**File to create:** `crates/roko-compose/src/causal_link.rs` (new file)

**Read first:**
- `crates/roko-chain/src/insight_store/mod.rs` -- `InsightType::CausalLink`
- `crates/roko-compose/src/bidders.rs` -- bidder output format
- `crates/roko-neuro/src/knowledge_store.rs` -- `KnowledgeEntry` relationships

**What to do:**

1. Create `crates/roko-compose/src/causal_link.rs`.
2. Define causal link composition:

```rust
/// A directed causal relationship between two knowledge entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    pub cause: String,       // content hash of cause entry
    pub effect: String,      // content hash of effect entry
    pub confidence: f64,     // 0.0 to 1.0
    pub evidence_count: u32, // number of independent observations
}

/// Compose causal chains: if A->B and B->C exist, infer A->C.
pub fn compose_causal_links(links: &[CausalLink]) -> Vec<CausalLink> {
    let mut composed = Vec::new();
    let by_cause: HashMap<&str, Vec<&CausalLink>> = links.iter()
        .fold(HashMap::new(), |mut acc, link| {
            acc.entry(link.cause.as_str()).or_default().push(link);
            acc
        });

    for link_ab in links {
        if let Some(bc_links) = by_cause.get(link_ab.effect.as_str()) {
            for link_bc in bc_links {
                // A->B + B->C = A->C
                let composed_confidence = link_ab.confidence * link_bc.confidence;
                if composed_confidence > 0.3 {  // threshold: only strong chains
                    composed.push(CausalLink {
                        cause: link_ab.cause.clone(),
                        effect: link_bc.effect.clone(),
                        confidence: composed_confidence,
                        evidence_count: link_ab.evidence_count.min(link_bc.evidence_count),
                    });
                }
            }
        }
    }

    composed
}
```

3. Wire into context assembly: when the `InsightStoreBidder` retrieves `CausalLink`-type entries, compose transitive chains and include the composed links as additional context sections.

4. Register in `crates/roko-compose/src/lib.rs`: `pub mod causal_link;`

**Test:**
- A->B (0.9) + B->C (0.8) composes to A->C (0.72).
- A->B (0.9) + B->C (0.3) composes to A->C (0.27) -> excluded (below 0.3 threshold).
- No matching B intermediate -> no composition.
- Cyclic links (A->B->A) do not produce infinite loops.

- [ ] `CausalLink` struct defined
- [ ] `compose_causal_links()` infers transitive A->C from A->B + B->C
- [ ] Confidence threshold filters weak chains
- [ ] Composed links appear as context sections
- [ ] Module registered in `lib.rs`

---

### Task 7.4: Integration test for chain-sourced context

**File to create:** `crates/roko-compose/tests/insight_store_integration.rs` (new file)

**Read first:**
- Tasks 7.1 through 7.3

**Do:**

1. **Scenario A: ISFR-relevant task gets chain context**
   - Create a task with domain "blockchain" and title "Monitor ISFR divergence"
   - Populate mock InsightStore with 5 blockchain-domain entries
   - Run the `InsightStoreBidder`
   - Assert: chain-sourced sections appear in output
   - Assert: sections have `insight_store:` source attribution

2. **Scenario B: Coding task does not query chain**
   - Create a task with domain "coding" and title "Fix authentication bug"
   - Assert: only local knowledge sections returned
   - Assert: no chain store queries made (verify via mock call count)

3. **Scenario C: CausalLink composition in context**
   - Create 3 `CausalLink` entries: A->B, B->C, C->D
   - Compose and verify A->C and B->D appear
   - Verify A->D appears (3-hop composition via two compose rounds)

4. Run: `cargo test -p roko-compose --test insight_store_integration`

- [ ] Blockchain tasks include chain-sourced context
- [ ] Non-blockchain tasks skip chain queries
- [ ] CausalLink composition produces transitive chains
- [ ] All integration tests pass

---

## Phase 8: WorldGraph context injection

**Goal**: Add a WorldGraph context bidder that contributes entity-level context from the dynamic worldview built by the foraging model (IMPL-09 Phase 8).

### Task 8.1: Define WorldGraphBidder

**File to create:** `crates/roko-compose/src/worldgraph_bidder.rs` (new file)

**Read first:**
- `crates/roko-compose/src/bidders.rs` -- existing bidder pattern
- `crates/roko-compose/src/prompt.rs` -- `AttentionBidder` enum, `PromptSection`
- IMPL-09 Phase 8 (WorldGraph crate structure)

**What to do:**

1. Create `crates/roko-compose/src/worldgraph_bidder.rs`.
2. Define the bidder:

```rust
/// Context bidder that contributes entity-level knowledge from the WorldGraph.
///
/// The WorldGraph accumulates entities and relationships discovered by
/// multi-chain ingestion and contract discovery. This bidder queries it
/// for entities relevant to the current task.
pub struct WorldGraphBidder {
    /// Query interface to the WorldGraph.
    graph: Arc<dyn WorldGraphQuery>,
    /// Maximum entities to include per bid.
    max_entities: usize,
    /// Minimum relevance score to include an entity.
    min_relevance: f64,
}

#[async_trait]
pub trait WorldGraphQuery: Send + Sync {
    /// Find entities relevant to the given query string.
    fn query_entities(&self, query: &str, limit: usize) -> Result<Vec<WorldEntity>>;
    /// Find relationships between two entities.
    fn query_relationships(&self, entity_a: &str, entity_b: &str) -> Result<Vec<WorldRelationship>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub properties: HashMap<String, String>,
    pub relevance_score: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldRelationship {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub confidence: f64,
}
```

3. Implement `ContextBidder for WorldGraphBidder`:

```rust
impl ContextBidder for WorldGraphBidder {
    fn bid(&self, task: &TaskContext, budget: &TokenBudget) -> Vec<ContextSection> {
        let entities = self.graph.query_entities(&task.title, self.max_entities)
            .unwrap_or_default();

        entities.iter()
            .filter(|e| e.relevance_score >= self.min_relevance)
            .map(|entity| {
                let content = format!(
                    "[WorldGraph: {} ({})]\n{}",
                    entity.name,
                    entity.entity_type,
                    entity.properties.iter()
                        .map(|(k, v)| format!("  {}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                ContextSection::new(content)
                    .with_source(format!("worldgraph:{}", entity.id))
                    .with_priority(SectionPriority::Normal)
                    .with_bid_value(entity.relevance_score)
            })
            .collect()
    }
}
```

4. Register in `lib.rs`: `pub mod worldgraph_bidder;`

**Test:**
- Mock WorldGraph with 5 entities. Query with matching title -> 5 sections returned.
- Entities below `min_relevance` are excluded.
- Empty WorldGraph -> no sections, no error.

- [ ] `WorldGraphBidder` defined with `WorldGraphQuery` trait
- [ ] Bidder produces sections from graph entities with relevance-scored bids
- [ ] Entities below threshold excluded
- [ ] Module registered in `lib.rs`

---

### Task 8.2: Wire WorldGraphBidder as 9th bidder in VCG auction

**File to modify:** `crates/roko-compose/src/bidders.rs` or `crates/roko-cli/src/orchestrate.rs`

**Read first:**
- `crates/roko-compose/src/prompt.rs` -- `AttentionBidder` enum (currently 8 variants)
- `crates/roko-compose/src/auction.rs` -- `vcg_allocate()` accepts a Vec of bidders
- Task 8.1 output

**What to do:**

1. Add `WorldGraph` variant to the `AttentionBidder` enum:

```rust
pub enum AttentionBidder {
    // ... existing 8 variants ...
    WorldGraph(WorldGraphBidder),
}
```

2. In the orchestrator's context assembly phase, construct and register the WorldGraph bidder when a WorldGraph is available:

```rust
if let Some(ref worldgraph) = self.worldgraph {
    bidders.push(AttentionBidder::WorldGraph(
        WorldGraphBidder::new(worldgraph.clone(), 5, 0.3)
    ));
}
```

3. The WorldGraph bidder participates in the VCG auction alongside all other bidders. It competes for token budget on merit -- no special treatment.

**Files to modify:**
- `crates/roko-compose/src/prompt.rs` (add enum variant)
- `crates/roko-cli/src/orchestrate.rs` (wire bidder registration)

**Test:**
- With WorldGraph available: assert 9 bidders participate in VCG auction.
- Without WorldGraph: assert 8 bidders (no error, no crash).
- WorldGraph sections appear in assembled prompt when they win auction slots.

- [ ] `WorldGraph` variant added to `AttentionBidder` enum
- [ ] Bidder registered in orchestrator when WorldGraph is available
- [ ] Participates in VCG auction with no special treatment
- [ ] Graceful absence when WorldGraph is not configured

---

### Task 8.3: Integration test for WorldGraph context injection

**File to create:** `crates/roko-compose/tests/worldgraph_integration.rs` (new file)

**Read first:**
- Tasks 8.1, 8.2

**Do:**

1. **Scenario A: Blockchain task with WorldGraph**
   - Create a task: "Analyze Uniswap V3 pool fee distribution"
   - Populate mock WorldGraph with entities: "Uniswap V3 Pool", "Fee Accumulator", "Tick Manager"
   - Run full context assembly with 9 bidders
   - Assert: WorldGraph entities appear in the assembled prompt
   - Assert: entities ranked by relevance score

2. **Scenario B: WorldGraph entities compete fairly**
   - Create a task where both neuro store and WorldGraph have relevant entries
   - Run VCG auction
   - Assert: highest-value entries win regardless of source
   - Assert: total token budget not exceeded

3. Run: `cargo test -p roko-compose --test worldgraph_integration`

- [ ] WorldGraph entities appear in assembled prompt for relevant tasks
- [ ] WorldGraph competes fairly with other bidders in VCG auction
- [ ] Token budget constraints respected
- [ ] All integration tests pass

---

## Acceptance criteria

- [ ] `CognitiveWorkspace` assembles correctly from 8 bidder outputs
- [ ] VCG auction respects budget constraints
- [ ] Critical sections bypass the auction and are always included
- [ ] `ContextPolicy` evolves: high-success categories get more budget after 100 episodes
- [ ] Section effect tracking measures lift correctly
- [ ] Domain-specific lift produces different assembly per domain
- [ ] Cache hits reduce assembly time by >50%
- [ ] U-shaped placement puts high-value sections at attention peaks
- [ ] Complexity scaling caps budget for trivial tasks
- [ ] Affect modulation adjusts allocation under stress vs confidence
- [ ] HDC similarity ranks knowledge entries by task relevance
- [ ] End-to-end: agent with learnable context shows measurable lift vs static context
- [ ] All structs are `Serialize + Deserialize` for persistence
- [ ] All new code has doc comments
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test -p roko-compose` passes

## Files created or modified

| File | Action |
|------|--------|
| `crates/roko-compose/src/workspace.rs` | **New**. CognitiveWorkspace, ContextCategory, ContextSection, assembly, U-placement, complexity scaling, affect modulation, HDC retrieval. |
| `crates/roko-compose/src/bidders.rs` | **New**. 8 ContextBidder implementations wrapping LearningBidder. |
| `crates/roko-compose/src/context_policy.rs` | **New**. Learnable allocation policy with Beta-posterior feedback. |
| `crates/roko-compose/src/workspace_cache.rs` | **New**. L0-L3 cache tiers with disk persistence and LRU eviction. |
| `crates/roko-compose/src/lib.rs` | **Modified**. Register workspace, bidders, context_policy, workspace_cache modules. |
| `crates/roko-compose/Cargo.toml` | **Modified**. Add `blake3` dependency. |
| `crates/roko-learn/src/section_effect.rs` | **Modified**. Add domain scoping to registry. |
| `crates/roko-compose/tests/workspace_integration.rs` | **New**. Integration tests for all phases. |

## Build and test commands

```bash
# Build
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-compose

# Unit tests
cargo test -p roko-compose

# Integration tests
cargo test -p roko-compose --test workspace_integration

# Section effect tests (roko-learn)
cargo test -p roko-learn -- section_effect

# Full workspace check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```
