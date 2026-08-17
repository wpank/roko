# Prompt Assembly Subsystem Audit

9-layer SystemPromptBuilder, role templates, token budgeting, VCG auction — a sophisticated system that only 1 of 6+ entry points actually uses.

## The Problem

The prompt assembly system is well-architected: 9 layers, proper templates, token budgeting, section effectiveness learning. But most entry points bypass it entirely:
- `roko` / `roko chat` / `roko "prompt"`: **No system prompt at all**
- ACP runner: **Inline format strings**
- Review/scribe: **Template-only, no budgeting**
- Only main dispatch in orchestrate.rs (dead): **Full 9-layer builder**

---

## 1. The 9-Layer SystemPromptBuilder

`roko-compose/src/system_prompt_builder.rs` — emits 8 distinct layers (plus variants) in cache-aligned order:

| Layer | Content | Cache Tier | What |
|---|---|---|---|
| 1 | Role identity | System (stable) | Role name, responsibilities, constraints |
| 2 | Conventions | System (semi-stable) | Code style, naming, patterns |
| 3 | Domain context | Session (semi-stable) | Relevant code files, project context |
| 3c | Active signals | Session (semi-stable) | Pheromone warnings (dead) |
| 4 | Task context | Task (volatile) | Task description, acceptance criteria |
| 4b | Gate feedback | Dynamic | Prior gate failures, error details |
| 5 | Tool instructions | System (stable) | Available tools and usage |
| 6 | Relevant techniques | Task (volatile) | Playbooks + skills |
| 7 | Anti-patterns | Task (volatile) | Known failure patterns from neuro |
| 8 | Affect guidance | Dynamic | Daimon state (overengineered, see doc 14) |

**Build methods:**
- `.build()` — simple concatenation (no budgeting)
- `.build_sections()` → `Vec<PromptSection>` for composer
- `.compose_with_budget()` — applies token budgeting via PromptComposer
- `.compose_build_with_budget_and_section_effectiveness()` — full pipeline with learning

---

## 2. Role Templates

13 templates in `roko-compose/src/templates/`, each implementing `RolePromptTemplate`:

| Role | File | Used By |
|---|---|---|
| Strategist | `strategist.rs` | orchestrate.rs (dead) |
| Implementer | `implementer.rs` | orchestrate.rs (dead) |
| Auditor/Architect | `reviewer.rs` | orchestrate.rs review path |
| Scribe/Critic | `scribe.rs` | orchestrate.rs doc revision |
| Researcher | `researcher.rs` | orchestrate.rs research |
| QuickReviewer | `quick.rs` | orchestrate.rs |
| + 7 others | various | Built, unused |

**Template format:** Each takes a typed input struct (no filesystem I/O) and emits `Vec<PromptSection>`.

**NOT used by:** `roko run`, `roko chat`, `roko "prompt"`, ACP runner, dispatch_direct.

---

## 3. Token Budgeting

**4 strategies in `roko-compose/src/strategy.rs`:**

| Strategy | How | Actually Used? |
|---|---|---|
| DensityGreedy | Greedy knapsack, sort by value/cost | Yes (default) |
| WeightedSum | Linear optimization | Rarely |
| VCG | Welfare auction with fake payments | Built but rarely activates |
| Auto | VCG only when bidders have 50+ observations | Default setting |

**Reality:** DensityGreedy dominates. VCG warmup threshold (50+ observations per subsystem) is rarely reached in practice. VCG diagnostics are computed but don't influence actual section selection.

**VCG auction** (`auction.rs`): Fully implemented welfare-maximizing auction with:
- Bidder bids per section
- Payment computation (externality-based)
- Pareto optimality check
- Welfare loss estimation

**UNIFIED-IMPLEMENTATION-PLAN says:** Delete VCG payments computation. Keep greedy knapsack only.

---

## 4. Who Uses What

| Entry Point | Builder? | Templates? | Budgeting? | Effectiveness Learning? |
|---|---|---|---|---|
| `roko "prompt"` | No | No | No | No |
| `roko` (inline chat) | No | No | No | No |
| `roko chat` (REPL) | No | No | No | No |
| `roko run` | Yes (9-layer) | Yes | Yes | Yes |
| `roko plan run` (runner v2) | Partial | Partial | No | No |
| ACP runner | No (inline strings) | No | No | No |
| ACP bridge_events | No (inline strings) | No | No | No |
| orchestrate.rs main dispatch (dead) | Yes (full) | Yes | Yes (Composer) | Yes (SectionEffectiveness) |
| orchestrate.rs review (dead) | Template only | Yes | No | No |
| orchestrate.rs scribe (dead) | Template only | Yes | No | No |
| orchestrate.rs retry (dead) | No (inline strings) | No | No | No |

**3 out of 10 paths use the builder.** Only 1 (dead) uses the full pipeline with budgeting and learning.

---

## 5. Inline Prompt Strings (Anti-Pattern #2)

Hardcoded prompts found at:

| File | Lines | Purpose |
|---|---|---|
| `dispatch_helpers.rs` | 101-105 | Task context frame |
| `orchestrate.rs` | 9941 | Fallback task prompt |
| `orchestrate.rs` | 9960-9962 | Retry hint wrapper |
| `orchestrate.rs` | 11214-11216 | Gate failure replan |
| `orchestrate.rs` | 11280-11282 | Model escalation |
| `orchestrate.rs` | 11404-11408 | Architectural replan |
| `orchestrate.rs` | 18678-18680 | Enrichment intro |
| `roko-acp/runner.rs` | 114, 143, 247, 333 | ACP review/fix/commit roles |
| `roko-acp/runner.rs` | 405-421 | ACP review variants |
| `roko-acp/runner.rs` | 525-531 | ACP architect/auditor roles |

The ACP runner has the most egregious inline prompts — full role descriptions hardcoded in `format!()` strings that duplicate what the template system already provides.

---

## 6. Context Section Assembly

In `dispatch_agent_with()` (orchestrate.rs:14350-14750), 7 context sources are queried:

1. **Code context** → keyword extraction + index search (3000 token budget)
2. **Playbooks** → `playbook_query_context()` (Layer 6)
3. **Skills** → matched from skill library (Layer 6)
4. **Anti-patterns** → `query_anti_knowledge_patterns()` from neuro store (Layer 7)
5. **Pheromones** → ambient signals (Layer 3c) — dead, should be warnings
6. **Search context** → external research (2048 token cap)
7. **Section effectiveness** → learned priority adjustments per role

**Scoring:** Priority + learned adjustment (SectionEffectivenessRegistry) + daimon modulation.

**None of this runs from live paths.** `dispatch_direct.rs` sends bare prompts with zero context.

---

## 7. Knowledge & Playbook Injection

**Knowledge injection** (orchestrate.rs:14689-14695):
```rust
let anti_patterns = query_anti_knowledge_patterns(&self.knowledge_store, task, 5);
// → Layer 7 (anti-patterns)
```

**Playbook injection** (orchestrate.rs:14361-14370):
```rust
let relevant_playbooks = self.playbook.query(&playbook_query).await?;
// → Layer 6 (relevant techniques)
```

Both work correctly — in the dead code path. Live paths get neither.

---

## 8. Composition Metadata

Every composition emits a `CompositionManifest`:
- Selected strategy (DensityGreedy vs VCG)
- Included sections + bid values + VCG payments
- Excluded sections + exclusion reasons
- Total tokens used vs budget
- Pareto optimality flag
- Welfare loss estimate

Persisted in episodes for learning. Only generated from the dead path.

---

## 9. Anti-Patterns In This Subsystem

| Anti-Pattern | Where |
|---|---|
| **#2 Inline prompt strings** | ACP runner (10+ locations), orchestrate.rs retry paths |
| **#6 Feedback as afterthought** | Section effectiveness only tracked from dead path |
| **#5 Hardcoded role behavior** | ACP runner has if/else for role prompts instead of templates |

---

## 10. What PromptAssemblyService Should Do

Phase 0.2 of the unified plan: a single service that all callers use:

```rust
trait PromptAssemblyService {
    fn assemble(&self, req: PromptAssemblyRequest) -> AssembledPrompt;
}
```

Where `PromptAssemblyRequest` includes: role, task, context sections, budget, tools.

Every entry point — `roko "prompt"`, `roko chat`, `roko run`, ACP — goes through this service. The service handles: template selection, 9-layer assembly, token budgeting, section scoring, knowledge/playbook injection.

---

## 11. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-compose/src/system_prompt_builder.rs` | ~600 | Core — good design |
| `roko-compose/src/templates/mod.rs` | ~100 | 13 templates |
| `roko-compose/src/templates/*.rs` | ~2K | Template implementations |
| `roko-compose/src/auction.rs` | ~500 | VCG — overengineered |
| `roko-compose/src/strategy.rs` | ~300 | Composition strategies |
| `roko-compose/src/context_provider.rs` | ~200 | Context section builder |
| `roko-cli/src/dispatch_helpers.rs` | ~200 | Prompt helpers |
| `roko-cli/src/prompting.rs` | ~300 | Role prompt builder |
| `roko-acp/src/runner.rs` | ~800 | Inline prompts (anti-pattern) |
