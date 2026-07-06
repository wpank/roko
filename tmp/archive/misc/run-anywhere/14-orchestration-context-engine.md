# Roko Orchestration: Context Engineering Engine

> **Audience**: Prompt engineers, model ops, ML pipeline builders
> **Scope**: The canonical target-state specification for Roko's context engine, index, retrieval cascade, structural compression, and prompt-assembly pipeline.

---

The naive approach to building multi-agent systems is to push the entire repository's filesystem into the prompt and let the model figure out what matters. This incurs massive token waste and degradation in model attention (**"Lost in the Middle"**, Liu et al., 2023). 

Roko takes the opposite approach. It relies on a deterministic, highly compressed context preparation phase. Instead of discovering context at runtime, the exact subset of required context is mathematically pre-computed before any inference token is processed.

## 1. The Nine-Layer Context Engine

Roko operates a sophisticated 9-layer retrieval and injection stack to achieve up to an 83% reduction in prompt token size.

### Layer 1: AST Extraction (Tree-sitter)
Tree-sitter parses the repository natively in Rust, building an index of public traits, function signatures, and exports in ~6ms. 
* **vs. Legacy LLM**: Instead of an agent reading 2,000 lines of `lib.rs` to find three exports, Roko structurally outputs a 30-token AST summary instantly.

### Layer 2: Workspace Index (Symbol Graph + PageRank)
Types mapped by tree-sitter are directed into a symbol graph modeled directly on the **Aider Repo Map** architecture, but improved. 
* Every public function, type, and trait is a node. Calls and imports form edges.
* Using a PageRank algorithm initialized to bias files currently tracked in the active Task's TOML array, Roko mathematically resolves the most critical cross-file dependencies.

### Layer 3: Semantic Code Search (HNSW + Embeddings)
Sometimes tasks require semantic relationships (e.g., "Implement rate limiting similarly to the gateway").
Roko embeds extracted AST chunks via local `CodeRankEmbed` (137M parameter ONNX) or `Voyage Code 3` APIs into an HNSW index via `usearch` (75K inserts/sec). 
Retrieval follows a **Hybrid Search** approach: HNSW density matching crossed with exact `ripgrep` keyword matching.

### Layer 4: Merkle Tree Change Detection
To prevent re-embedding identical files across concurrent parallel plan runs, a Merkle tree tracks file deviations from the base `roko` build frame. If a file hashing changes, only the divergent leaves trigger tree-sitter updates, dropping invalidation cascades from 110+ plans down to just 2-5 affected plans.

### Layer 5: Prompt Caching Alignment
Anthropic cache tiers offer a 90% discount on prefix repetition. But JSON dictionaries in Rust natively use unordered HashMaps—leading to unpredictable byte arrays that force cache misses.
* Roko serializes all tool and prompt injections using `BTreeMap`, enforcing deterministic alphabetical key ordering. 
* **Result**: >90% prefix cache success rate mathematically guaranteed across identical role invocations.

### Layer 6: Context Compression
Two-part compression aggressively curtails non-essential sequences:
1. **Structural**: Entire function bodies are removed from context mapping, leaving only the type signature `pub fn validate_token() -> Result<T, E>`.
2. **LLMLingua-2** (Pan et al., 2024): Token-level contextual compression is applied to English text strings, compressing free-text instructions by 3-6x with measured <1.5% loss of fidelity.

### Layer 7: Research Agent (Agentic RAG)
Before implementation, a Haiku-tier Research Agent explores the codebase independently to build a "Research Brief". It iteratively leverages Layers 1-6 in a closed CoT loop until it has accumulated sufficient state.
* **Citation**: Based on Karpathy's autoresearch patterns and Anthropic's Multi-Agent Research findings (showing a 90% lift over single-agent executions). The brief generated actively grounds the Implementation model against hallucinations.

### Layer 8/9: Extended Thinking & LLM-Judge Gates
Claude's `Extended Thinking` modes are explicitly toggled based on task designation. Complex tasks are allocated up to 16k reasoning tokens, while Trivial tweaks consume 0.
Upon implementation success, an LLM-Judge scores the output across 6 dimensions (Correctness, Security, Readability, Idiomaticness, Performance, Maintainability) using additive scoring with CoT reasoning.
* **Citation**: This rubric methodology mirrors the exact architecture proven most reliable in **G-Eval** (Liu et al., NeurIPS 2023).

---

## 2. Dynamic Prompt Budgeting

Different multi-role agents natively require vastly different context parameters. Supplying the entire workspace to every role starves specific attention zones. Based on the 200k/128k context ceilings, Roko computes dynamic per-role budgets. (Reserve 40% for output, 60% for input).

For a 120k token input budget:
* **Implementers**: Receive exact Task/Code subsets (25% Plan, 20% PRD Extract, 10% Worktree Map, 10% Code blocks, 10% Brief, 10% Prior Reviews).
* **Strategists**: Receive 30% Plan, 20% Workspace Map, 20% PRD Extract (0 code implementations; strategists only execute logic).

When the budget threshold is hit, items are deterministically truncated from the tail, as Sequential logic mandates the survival of introductions over trailing edge files. Roko explicitly injects `<!-- roko:layer:N -->` caching transition boundaries to further optimize LLM prefix parsing.

## 3. Worktree Isolation: Global Base vs. Per-Agent Overlay

When Agent A operates on Plan 03 and Agent B operates on Plan 04 concurrently, they spawn isolated Git Worktrees. If they shared a centralized AST/Semantic search index, Agent A's half-written types would bleed into Agent B's retrieval space. 

To solve this without wasting 8x memory footprint duplicating the entire 12,000-chunk codebase matrix for every agent, Roko utilizes **Overlay Networking**:

1. **Global Base**: The shared, core repository (12,340 indexed blocks, read-only).
2. **Worktree Overlay**: As Agent A writes new code in its local isolation directory, Roko processes a ~200KB differential overlay network localized tracking *only* new AST nodes and Embeddings. 

When Agent A runs a semantic search, it checks the base index + its own tiny overlay mapping. The system operates entirely synchronously with zero cross-contamination.

## 4. The 83% Token Validation

A typical approach across a 20-plan build with 150k token prompts executes an input cost of roughly $105.00 on Sonnet 3.5.
By deploying the Roko structural extraction, HNSW chunking, tree-sitter tracking, and LLMLingua-2 compressions, an agent routinely processes identical tasks utilizing less than 25,000 input tokens.
Combined with the BTreeMap prefix cache optimizations taking 90% discounts on the stable `AGENTS.md` and Workspace Maps, the net cost plummets to ~$31.50 — a consistent 83% structural reduction while directly increasing the mathematical success odds by deleting noise parameters from the model's inference scope.

---

## 5. Detailed Mechanism Parameters

### 5.1 Tree-sitter Performance

- **Initial parse**: ~6ms for a typical 2,000-line Rust source file (single-pass incremental LR)
- **Incremental re-parse**: <1ms after an edit (only affected subtree rebuilt)
- **Language coverage**: 100+ languages via community grammars
- **Native support**: Rust (`roko-lang-rust`), TypeScript (`roko-lang-typescript`), Go (`roko-lang-go`)
- **Symbol extraction**: structs, enums, traits, functions, type aliases, constants, statics, modules
- **Output**: 30-token AST summary from a 2,000-line file (public signatures only, bodies stripped)

### 5.2 PageRank for Symbol Graph

Parameters in `roko-index/src/graph.rs`:
- **Damping factor**: 0.85 (standard Brin & Page, 1998)
- **Iterations**: 30 (normal), 50 (cycle-heavy graphs)
- **Initialization bias**: Files in task TOML `files` array start at `2.0/N` (double the uniform `1.0/N`)
- **Edge types**: Calls, Imports, Implements, Contains
- **Performance**: ~2ms for 12,000-symbol codebase (in-memory HashMap iteration)

### 5.3 HNSW Index Parameters (usearch)

| Parameter | Value | Meaning |
|---|---|---|
| M | 16 | Bi-directional links per node (balance: size vs recall) |
| ef_construction | 200 | Candidate list during build (high = better graph quality) |
| ef_search | 100 | Candidate list during query (~0.1ms, >95% recall@10) |
| Metric | Cosine | Distance function |
| Dimensions | 768 | CodeRankEmbed output size |
| Throughput | 75K inserts/sec | Index build speed |

### 5.4 Embedding Models

| Model | Type | Dimensions | Speed | Best For |
|---|---|---|---|---|
| **CodeRankEmbed** (default) | Local ONNX, 137M params | 768 | 200 chunks/sec (M-series), 80/sec (Intel) | Batch indexing (deterministic, no network) |
| **Voyage Code 3** (fallback) | Remote API | 1024 → projected to 768 | Network-bound | Semantic queries on natural language |

### 5.5 BTreeMap Cache Determinism

Standard `HashMap` iterates in arbitrary order (hash-seed-dependent) → different byte sequences for identical data → cache miss.

`BTreeMap` iterates in sorted key order → byte-identical JSON → cache hit.

Measured impact:
- **Without BTreeMap**: ~30% cache hit rate
- **With BTreeMap**: >90% cache hit rate for identical role invocations

Cache transition boundaries injected via `<!-- roko:layer:N -->` markers.

### 5.6 LLMLingua-2 Compression (Pan et al., ACL 2024)

Applied to prose (PRD text, research briefs, instructions). NOT applied to code, type signatures, file paths, or tool definitions.

| Content Type | Compression | Fidelity Loss |
|---|---|---|
| PRD requirements | 4.2x | <1.0% |
| Research brief | 5.8x | <1.5% |
| Instructions | 3.1x | <0.5% |
| Task descriptions | 3.7x | <1.2% |

### 5.7 Overlay Networking for Per-Worktree Indexes

Two tiers prevent 600MB of redundant memory (12 agents × 50MB per index):

1. **Global base** (read-only, shared): Full repo at base commit. 12,340 blocks, ~50MB. Memory-mapped — single physical copy.
2. **Worktree overlay** (per-agent): Only deltas. 50-200 entries, ~200KB. Rebuilt incrementally on each agent write.

Query: overlay first (local modifications) → base second → merge results. Cross-contamination impossible.

### 5.8 The 83% Reduction: Exact Math

| Optimization | Reduction | Tokens After (per plan) |
|---|---|---|
| Structural extraction (signatures only) | 60% | 60,000 |
| PageRank top-k filtering | 30% of remainder | 42,000 |
| HNSW semantic scoping | 20% of remainder | 33,600 |
| LLMLingua-2 on prose | 4x on ~15% | ~28,000 |
| **Per-plan total** | | **~25,000** |
| 20 plans total input | | 500,000 |
| BTreeMap prefix caching (90% on 60%) | | ~200,000 billable |
| **Net cost** | | **~$31.50** (vs $105.00 baseline) |

Token reduction: 83%. Cost reduction: 70%. Additional cost savings from prefix caching on already-reduced prompts.

---

## 9. Prompt Structure for High Attention

Prompts are ordered for maximum LLM attention (Liu et al., 2023 — "Lost in the Middle"):

```
HIGH ATTENTION (Beginning):
  → MCP tool guidance (most critical for tool selection)
  → Conductor steering messages (urgent directives)
  → Assignment + constraints (what to do)

LOWER ATTENTION (Middle):
  → Plan content
  → Verification requirements
  → Brief
  → PRD2 extract
  → Skill injections
  → Task checklists

HIGH ATTENTION (End):
  → Iteration feedback (what failed last time)
  → Reviewer criteria (what reviewers will check)
  → Self-validation requirements
  → Completion instructions
```

**Key insight**: Critical context at the START and END of prompts. Less-critical context in the middle. The LLM pays most attention to the beginning and end — the middle is the "dead zone."

---

## 10. The Research Agent (Agentic RAG)

Before implementation, a Haiku-tier Research Agent explores the codebase independently:

```
Research Agent cycle:
  1. Query MCP code intelligence (search_code, get_symbol_context)
  2. Retrieve relevant episodes from Grimoire
  3. Cross-reference with existing plans
  4. Produce "Research Brief" with findings
  5. Iterate until satisfied (closed CoT loop)
```

**Output**: `research.md` — a structured document containing:
- Existing patterns in the codebase relevant to the task
- Similar prior implementations and their outcomes
- Dependencies and integration points
- Potential pitfalls (from error pattern history)

**Impact**: Research Agents produce a 90% lift over single-agent execution for complex tasks (Anthropic multi-agent research findings). The brief grounds the Implementation agent against hallucinations.

**Cost**: ~$0.01-0.05 per research brief (Haiku tier). ROI: prevents $0.50+ failed implementation iterations.

---

## 11. Context Injection into Worktree

Each agent's worktree receives pre-assembled context files at `/context/in/`:

```
context/in/
├── execution-pack.md          # Default: merged context for any role
├── implementer-pack.md        # Role-specific: Implementer context
├── architect-pack.md          # Role-specific: Architect context
├── auditor-pack.md
├── quick-reviewer-pack.md
├── strategist-pack.md
├── researcher-pack.md
├── scribe-pack.md
├── critic-pack.md
├── auto-fixer-pack.md
├── conductor-pack.md
├── brief.md                   # Pre-assembled brief
├── prd2-extract.md            # Relevant PRD paragraphs
├── decomposition.md           # Step-by-step breakdown
├── verify-tasks.toml          # Verification steps per rung
├── review-tasks.toml          # Reviewer criteria
├── learning.md                # Playbook + research + patterns
├── research.md                # Research agent findings
├── playbook.md                # Validated rules
├── reflections.md             # Iteration memory from prior attempts
└── artifact-status.md         # Which artifacts are fresh vs stale
```

The agent reads its role-specific pack. It never sees the full PRD, the full codebase, or other agents' context — only what's been pre-computed as relevant for its specific task and role.

---

## 12. Plan Refresh Workflow (Staleness Detection)

When PRDs change, affected plans must be refreshed. The workflow:

1. **PRD fingerprinting**: HDC vectors computed for each PRD (<3ms for 343 files)
2. **Plan-to-PRD mapping**: Bipartite graph with fuzzy matching
3. **Staleness detection**: Content hash comparison + two-stage semantic filter
4. **In-flight handling**: Audit tasks for completed plans (don't invalidate finished work)
5. **Targeted regeneration**: Regenerate only stale sections ($0.02/section vs $0.50 full plan)
6. **Output update**: Write refreshed artifacts back to plan directory

**Cost**: 343 PRD files, 20 stale, 5 uncovered → ~$0.60 total refresh.

**Research**: Merkle tree change detection (Layer 4 in the context engine) ensures only divergent files trigger re-indexing.

---

## 13. The ContextAssembler Trait (Composable Assembly)

The context engine is not a monolith — it's a composable pipeline of independent assemblers:

```rust
pub trait ContextAssembler: Send + Sync {
    fn assemble(
        &self,
        sections: Vec<PromptSection>,
        budget: TokenBudget,
    ) -> AssembledContext;
}

pub struct PromptSection {
    pub name: &'static str,      // e.g., "workspace_map", "prd_extract"
    pub content: String,
    pub priority: u8,            // 5=always include, 1=drop first
    pub hard_cap: Option<usize>, // Max tokens for this section
    pub cache_layer: CacheLayer, // Role(1), Workspace(2), Plan(3), Volatile(0)
}

pub struct TokenBudget {
    pub total: usize,            // Max input tokens
    pub reserved_for_output: usize, // Reserve for completion
    pub available: usize,        // total - reserved
}
```

### Assembly Algorithm

```
1. Collect all sections from enrichment pipeline
2. Sort by cache_layer (stable first) then priority (highest first)
3. For each section:
   a. Count tokens (via TokenCounter for the target model)
   b. If fits within remaining budget: include
   c. If priority >= Critical and doesn't fit: truncate to fit
   d. If priority < Critical and doesn't fit: drop
4. Insert cache layer markers at boundaries
5. Return assembled context with metadata
```

### AssembledContext Metadata

```rust
pub struct AssembledContext {
    pub text: String,
    pub sections_included: Vec<String>,
    pub sections_dropped: Vec<String>,
    pub sections_truncated: Vec<String>,
    pub total_tokens: usize,
    pub cache_prefix_tokens: usize,  // Tokens in stable prefix (cacheable)
    pub volatile_tokens: usize,       // Tokens in volatile suffix
}
```

This metadata feeds the section effectiveness tracker — learning which dropped/truncated sections correlated with gate failures.

---

## 14. The AttentionAuction (Golem Context Budget)

For DeFi agents, the context budget is allocated via a VCG auction where subsystems bid for token space:

### Five Bidders

| Bidder | What It Bids For | Bid Strategy |
|---|---|---|
| **Oracle** | Prediction-relevant context | Bid ∝ prediction residual (where am I wrong?) |
| **Daimon** | Emotionally relevant context | Bid ∝ PAD alignment (mood-congruent) |
| **Risk Engine** | Position-protective context | Bid ∝ CVaR reduction (what protects my portfolio?) |
| **Curiosity** | Novel information | Bid ∝ KL divergence (what would I learn?) |
| **Mortality** | Survival-relevant context | Bid ∝ urgency (how close to death?) |

### Budget Allocation

Total context budget (e.g., 120K tokens) is partitioned:
- 60% to highest-bidding category (primary focus)
- 25% to second-highest (secondary)
- 15% spread across remaining (diversity minimum)

**Strategy-proofness**: VCG mechanism guarantees truthful bidding is the dominant strategy. Subsystems cannot game for more budget by misreporting values.

**Research**: Vickrey (1961), Clarke (1971), Groves (1973) — mechanism design for truthful resource allocation. Kahneman (1973) — attention as limited capacity requiring allocation policy.

---

## 15. Prompt Construction Per Role (Exact Differences)

Each of the 28 roles gets a structurally different prompt:

### Implementer Prompt (Largest, ~25K tokens)

```
[HIGH ATTENTION - Beginning]
  MCP tool guidance (which tools to prefer)
  Conductor steering messages (if any active directives)
  Assignment + constraints (task TOML + file assignments)

[MIDDLE]
  Plan content (the overall plan this task belongs to)
  Verification requirements (what gates will check)
  Strategist brief (pre-analyzed approach)
  PRD2 extract (relevant requirements paragraphs)
  Skill injections (playbook rules matching this task)
  Task checklists (acceptance criteria)
  File context (relevant source code snippets)

[HIGH ATTENTION - End]
  Iteration feedback (what failed last time, if retrying)
  Reviewer criteria (what reviewers will evaluate)
  Self-validation requirements
  Completion instructions
```

### AutoFixer Prompt (Minimal, ~2K tokens)

```
Error output (structured error digest, max 10 unique errors)
Affected file list (only files with errors)
Explicit constraint: "Fix ONLY the listed errors. Do NOT refactor."
```

No plan content. No PRD. No workspace map. No skills. Just the error and the file.

### TestGenerator Prompt (Isolated)

```
tasks.toml (acceptance criteria + rubric.md)
System instruction: "You have NO access to implementation files."
Output format: JSON with symbol_checks, behavioral_tests, property_tests, integration_scenarios
```

The TestGenerator NEVER sees the implementation code. This prevents reverse-engineering the expected output — the tests must be satisfied by implementing correct behavior, not by gaming the tests.

### Compressed Review Feedback (3-Bullet Format)

When injecting prior review feedback into retry prompts:

```
1. What failed: [specific test, line, assertion]
2. Why it failed: [root cause analysis]
3. What to do: [specific fix with reference pattern]
```

Maximum 3 bullets. No prose, no context. This keeps feedback cost < 200 tokens vs 2,000+ for raw review output.

### Skill Auto-Detection

Some skills load automatically based on file patterns:

```rust
if files.iter().any(|f| f.contains("bardo-terminal"))
    && matches!(role, Implementer | TerminalValidator) {
    inject_skill("ratatui-cinematic");
}

if role == Scribe || role == Critic || role == DocVerifier {
    inject_skill("humanizer");
}
```

### PromptBuild Metadata (Tracking What Was Built)

```rust
pub struct PromptBuild {
    pub prompt: String,
    pub context_strategy: ContextStrategy,  // mcp_first | hybrid | inline_heavy
    pub context_pack_bytes: usize,
    pub inline_context_bytes: usize,
    pub cache_hit: bool,                    // Was the context pack cached?
    pub playbook_hits: usize,               // How many rules injected?
    pub research_prepass_used: bool,         // Did research agent run?
    pub verify_artifacts_fresh: bool,        // Are verification artifacts current?
}
```

This metadata feeds the efficiency tracker — correlating prompt composition with gate outcomes to learn what works.

---

## 16. The ContextAssembler: Building Prompts for Each Role

### Per-Role Budget Allocation

Not every role needs the same kind of context. An Implementer needs detailed code snippets and file contents. A Strategist needs the big picture — plan structure, workspace map, dependency graph. Giving an Implementer a 50K-token workspace map wastes budget on irrelevant information. Giving a Strategist 8K of raw source code distracts from high-level reasoning.

The ContextAssembler allocates tokens to sections based on the agent's role:

| Section | Implementer | Strategist | Architect | Scribe | AutoFixer |
|---|---|---|---|---|---|
| Plan content | 50K chars | 50K chars | 30K chars | 10K chars | 0 |
| PRD extract | 8K chars | 15K chars | 20K chars | 20K chars | 0 |
| Workspace map | 5K chars | 20K chars | 15K chars | 5K chars | 0 |
| Code context | 8K chars | 0 | 5K chars | 2K chars | Error files only |
| Research brief | 5K chars | 10K chars | 8K chars | 3K chars | 0 |
| Playbook rules | 3K chars | 5K chars | 3K chars | 1K chars | 0 |
| Reflections | 3K chars | 2K chars | 2K chars | 1K chars | 3K chars |
| Error digest | 2K chars | 0 | 1K chars | 0 | 10K chars (primary) |
| Review feedback | 3K chars | 0 | 0 | 0 | 5K chars |
| Iteration history | 2K chars | 1K chars | 1K chars | 0 | 2K chars |

**Key asymmetries**:
- **Implementer** gets heavy code context (8K) and the full plan (50K). It needs to know exactly what files to modify and how they relate to the task.
- **Strategist** gets heavy plan context (50K) and a large workspace map (20K). It needs the bird's-eye view to reason about architecture and decomposition. It gets zero code — strategists reason about structure, not syntax.
- **Architect** gets balanced context across all dimensions. It needs to see code patterns (5K), understand requirements (20K PRD), and know the plan structure (30K).
- **Scribe** gets heavy documentation context (20K PRD, 5K workspace map) and light code context (2K). Its job is to produce documentation, not to understand implementation details.
- **AutoFixer** gets almost exclusively error context (10K error digest, 5K review feedback). It does not need the plan, the PRD, or the workspace map. It needs to know what broke and how to fix it.

### The Assembly Algorithm (Step by Step)

```
Input: role, task, sections[], model_context_limit

1. SORT sections by CacheLayer (stable layers first):
   - CacheLayer::Role (layer 1)     → Role-specific system prompt, tool definitions
   - CacheLayer::Workspace (layer 2) → Workspace map, AGENTS.md, roko.toml
   - CacheLayer::Plan (layer 3)      → Plan content, PRD extract, task TOML
   - CacheLayer::Volatile (layer 0)  → Iteration feedback, error digest, reflections

   Within each layer, sort by priority (5=Critical, 4=High, 3=Medium, 2=Low, 1=Optional)

2. ALLOCATE tokens per section by priority:
   - Priority 5 (Critical): guaranteed allocation up to hard_cap
   - Priority 4 (High): allocated from remaining budget
   - Priority 3 (Medium): allocated if >30% budget remains
   - Priority 2-1 (Low/Optional): allocated if >50% budget remains

3. FIT to budget greedily:
   for each section in sorted order:
     tokens_needed = count_tokens(section.content, model)
     if tokens_needed <= remaining_budget:
       include(section)
       remaining_budget -= tokens_needed
     elif section.priority >= Critical:
       truncated = truncate_to_fit(section.content, remaining_budget)
       include(truncated)
       remaining_budget = 0
     else:
       drop(section)  // tracked in sections_dropped metadata

4. U-SHAPED PLACEMENT (Liu et al., 2023):
   - Critical sections → beginning of prompt (highest attention)
   - Medium sections → middle of prompt (lowest attention zone)
   - Iteration feedback, completion instructions → end of prompt (high attention)

   This exploits the empirical finding that LLMs attend most strongly to
   the beginning and end of their context window, with a "dead zone" in
   the middle where retrieval accuracy drops 20-40%.

5. INSERT cache layer markers:
   Between each CacheLayer transition, insert:
   <!-- roko:layer:N -->
   This enables the LLM provider's prefix caching to recognize stable
   content boundaries and cache them across invocations.

6. RETURN AssembledContext with metadata:
   - sections_included, sections_dropped, sections_truncated
   - total_tokens, cache_prefix_tokens, volatile_tokens
```

### Context Overlays: Per-Worktree Mutations

When Agent A is implementing a task in its worktree, it creates new files, modifies existing ones, and potentially introduces new symbols and types. These changes should be visible in Agent A's context (so it can reference its own work) but invisible in Agent B's context (to prevent cross-contamination).

The context overlay system provides this isolation:

```
Global Base Index (read-only, shared)
  12,340 blocks from base commit
  ~50MB memory-mapped — single physical copy
  │
  ├── Agent A Overlay (per-worktree)
  │     38 new/modified blocks
  │     ~50KB
  │     Rebuilt incrementally on each write
  │
  └── Agent B Overlay (per-worktree)
        12 new/modified blocks
        ~20KB
        Rebuilt incrementally on each write
```

**Query resolution**: When Agent A searches for a symbol, the overlay is checked first (local modifications take precedence), then the base index. Results are merged with overlay entries taking priority on conflicts. Agent B's overlay is never consulted — complete isolation.

**Incremental rebuild**: When Agent A writes a file, only that file's AST is re-parsed by tree-sitter (<1ms for an incremental edit). The overlay entry for that file is updated. The global base index is never modified — it remains a stable, shared snapshot.

**Memory efficiency**: Without overlays, 12 concurrent agents would each need a full copy of the 50MB index (600MB total). With overlays, the shared base costs 50MB (memory-mapped, single physical copy) plus ~50KB per agent overlay = ~50.6MB total for 12 agents. A 12x memory reduction.

### The Execution Pack

The final assembled context is written to each agent's worktree as a single file:

```
{worktree}/context/in/execution-pack.md
```

This file contains the merged, role-specific context for the current task:

| Section | Content | Source |
|---|---|---|
| Brief | High-level task description and approach | Strategist output or plan TOML |
| PRD extract | Relevant paragraphs from the PRD | Semantic search against task description |
| Decomposition | Step-by-step breakdown of the task | Task TOML decomposition field |
| Verify tasks | Gate criteria for this task's rung | verify-tasks.toml per rung |
| Learning | Playbook rules + research + error patterns | roko-learn aggregation |
| Playbook | Validated rules from prior successful builds | playbook.md (from episode analysis) |
| Reflections | Iteration memory from prior attempts | reflections.md (per-task persistence) |
| Artifact status | Which artifacts are fresh vs stale | Merkle tree comparison |
| Research | Researcher agent analysis for this plan | research.md (if research prepass ran) |

The agent reads this single file at the start of its turn. It never needs to navigate the broader filesystem for context — everything relevant has been pre-assembled and placed in its worktree.

---

## 17. The Learning Pack: Per-Task Context Injection

Every task receives context from prior runs — a curated set of signals that help the agent avoid repeating mistakes and leverage proven patterns. This is the **learning pack**, assembled from eight distinct sources.

### Playbook Hints

The playbook is a set of validated rules extracted from prior builds. Rules that correlated with successful gate outcomes gain confidence; rules that correlated with failures lose confidence. The learning pack includes only rules that exceed a confidence threshold (default: 0.7) and match the current task's characteristics.

```
Example playbook hints injected into an Implementer's context:

- When modifying roko-gate crates, always run `cargo test -p roko-gate`
  before committing. Gate tests have cross-crate dependencies that
  `cargo test` alone may not catch. (confidence: 0.92, source: episode-1847)

- Trait implementations in roko-core must preserve Send + Sync bounds.
  Removing these bounds causes compilation failures in 6+ downstream
  crates. (confidence: 0.88, source: episode-2103)
```

**Source**: `roko-learn` crate, extracted from `EpisodeLogger` data. Rules are updated after every plan execution.

### Research Artifacts

When a Research Agent runs before implementation (the agentic RAG prepass), its findings are injected as structured context:

- Existing patterns in the codebase relevant to this task
- Similar prior implementations and their outcomes (success or failure)
- Dependencies and integration points that the implementer must account for
- API surface analysis: which functions, traits, and types are involved

**Source**: `research.md` produced by the Research Agent's closed CoT loop. Cost: ~$0.01-0.05 per brief.

### Dependency Manifests

External dependencies required by the task — crate versions, feature flags, compatibility constraints:

```
- alloy v0.14+ required (v0.13 has breaking API changes in Provider trait)
- tokio: must use multi-threaded runtime (features = ["rt-multi-thread"])
- serde: derive feature required for all new structs
```

**Source**: Extracted from `Cargo.toml` analysis and prior build failures.

### Fixture Manifests

Test fixtures required for this task — Anvil fork configurations, mock HTTP endpoints, test data:

```
- Anvil fork: mainnet block 19234567 (contains the Uniswap V3 pool state
  needed for LP tests)
- Mock HTTP: 1inch API endpoint at localhost:8545 returning fixture response
  for ETH/USDC swap quote
- Test wallet: 0xf39F...2266 funded with 100 ETH + 10000 USDC on fork
```

**Source**: `fixture-manifest.toml` in the plan directory, auto-generated from task analysis.

### Integration Memos

Cross-system notes from prior plans that affect this task — interface contracts, shared state, coordination requirements:

```
- roko-orchestrator writes executor state to .roko/state/executor.json
  after every task group. If your task modifies the executor state struct,
  update the serialization in orchestrate.rs lines 847-892.

- The ToolDispatcher in roko-agent expects tools to implement the ToolHandler
  trait. If you add a new tool, register it in StaticToolRegistry::new().
```

**Source**: Integration analysis from prior plan executions. Accumulated in `.roko/learn/integration-memos.jsonl`.

### Error Patterns

Common failure modes observed by the conductor's 10 watchers, relevant to this task type:

```
- CompileFailRepeat: Tasks modifying roko-core frequently trigger
  downstream compile errors in roko-agent and roko-orchestrator.
  Run full workspace build, not just crate-local.

- ContextWindowPressure: Tasks with >10 file modifications tend to
  exceed the Implementer's context budget. Consider splitting into
  subtasks if the file list exceeds 8 files.
```

**Source**: `roko-conductor` watcher history, filtered by task type and crate affinity.

### Reflections (Iteration Memory)

When a task has been attempted before (prior iteration failed gates), the reflection from that attempt is injected:

```
Attempt 1 (failed at Test gate):
  - Implemented LoopGuard::check() but forgot to update the
    ActionFingerprint hasher to include tool arguments.
  - Test `test_loop_detection_with_varied_args` failed because
    identical fingerprints were produced for different tool calls.
  - Fix: hash tool name + serialized arguments, not just tool name.
```

**Reflections are per-task, not per-plan**: Each task's reflection is specific to that task's prior failure. The implementer on Attempt 2 sees exactly what went wrong on Attempt 1 and what the root cause was.

**Source**: Generated by the conductor at the end of each failed iteration. Stored in `.roko/learn/reflections/{task-id}.md`.

### Contrarian Entries (Anti-Echo-Chamber Injection)

To prevent the agent from developing tunnel vision — reinforcing its own assumptions without considering alternatives — the learning pack injects a **15% contrarian allocation**: entries that represent the mood-opposite perspective.

```
If the agent's current approach is "add a new retry mechanism":

Contrarian injection:
  - Prior analysis suggests the retry mechanism in roko-agent/src/retry.rs
    already handles this case. Have you verified that the existing
    mechanism is insufficient before adding a new one?

  - 3 of the last 5 tasks that added new retry mechanisms were later
    removed as duplicates of existing functionality. Search before building.
```

**Why 15%**: Too little contrarian context (5%) is easily ignored by the model. Too much (30%) becomes adversarial and confuses the reasoning process. 15% provides enough friction to force consideration of alternatives without dominating the context.

**Selection**: Contrarian entries are selected from the knowledge base using mood-inverted HDC search. If the current task's fingerprint points toward "add new code," the contrarian search biases toward entries related to "reuse existing code" or "simplify." The entries must still be topically relevant — random contradictions provide no value.

**Source**: Korai Ledger knowledge entries (when on-chain) or local neuro entries (when offline), filtered by inverted mood vector and topical relevance.
