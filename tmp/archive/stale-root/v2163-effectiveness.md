# v2.1.63 vs v2.1.96: Effectiveness Comparison

Data from a token-tracking gateway between Claude Code and the Anthropic API. Both versions running Opus 4.6 on 1M context, same work, same repo, same parallel terminal setup, continuous 11-hour window on Apr 8. Only variable is the CLI version.

---

## Per-request efficiency

These metrics control for activity level. Regardless of how many terminals are open or how fast the operator is working, each individual request tells the story.

| Metric | v2.1.96 (478 Opus reqs) | v2.1.63 (2,492 Opus reqs) | Delta |
|---|---|---|---|
| Opus cost/request | $0.299 | $0.097 | **-68%** |
| Opus output tokens per dollar | 1,369 | 2,987 | **2.2x** |
| Opus output per 1K context tokens | 1.46 | 4.23 | **2.9x** |
| Avg context per request | 279,939 | 68,364 | -76% |

Each dollar spent on v2.1.63 produces 2.2x more output tokens. Each unit of context consumed produces 2.9x more output. The model is doing more useful work per API call because it's not dragging 280K of accumulated context through every request.

---

## Code quality: functional code vs scaffolds

The same kind of work was being done on both versions: wiring subsystems into an orchestration loop for a Rust workspace.

### v2.1.96 produced (5.2 hours, $152 in API spend):

- **6 commits**, 334 files changed, 17,152 insertions
- Created `roko-golem` crate with 6 modules (507 lines): every module is a scaffold returning static strings. `MortalityEngine::pulse()` returns `"roko-golem scaffold: mortality"`. `DaimonEngine::evaluate()` returns `"roko-golem scaffold: daimon"`. Six identical copy-paste patterns.
- Created TUI dashboard, efficiency page, operations page: all scaffolds with `todo!()` or placeholder text
- **15 new files flagged as scaffold/placeholder** by grep
- The `roko-golem` crate is not imported or used by any other crate in the workspace. Dead code.

Example of what v2.1.96 produced:
```rust
// 44 lines to return a static string
pub struct MortalityEngine;
impl MortalityEngine {
    pub const MARKER: &'static str = "roko-golem scaffold: mortality";
    pub const fn pulse(self) -> &'static str { Self::MARKER }
}
```

### v2.1.63 produced (6.0 hours, $255 in API spend):

- **1 large commit**, 23 files changed, 5,821 insertions
- `context_provider.rs` (1,122 lines): demand-driven, tier-aware context assembly with three tiers (Surgical/Focused/Full), token budgets, and model-specific behavior for local vs cloud models
- `symbol_resolver.rs` (616 lines): grep-based symbol resolution that finds struct/fn/trait/enum definitions and extracts signatures for agent context
- `task_brief.rs` (365 lines): generates task-scoped briefs from plan artifacts with dependency graph excerpts
- `orchestrate.rs` grew by 2,833 lines: wired real imports from 15+ crates, connected learning runtime, process supervisor, MCP config, gate dispatch, worktree management, and observability sinks
- All new files are imported and used by `orchestrate.rs` and `lib.rs`. Zero dead code.

Example of what v2.1.63 produced:
```rust
// Derives context tier from task complexity and model backend.
// Local models always get Surgical regardless of task tier.
pub fn from_task_and_model(task_tier: &str, model_slug: &str) -> Self {
    if is_local_model(model_slug) {
        return Self::Surgical;
    }
    match task_tier {
        "mechanical" => Self::Surgical,
        "architectural" => Self::Full,
        _ => Self::Focused,
    }
}
```

---

## Context efficiency

| Metric | v2.1.96 | v2.1.63 | Delta |
|---|---|---|---|
| Avg Opus context per request | 279,939 | 68,364 | **-76%** |
| Max context observed | 427,747 | 126,236 | -70% |
| Avg cache_create per request | 23,804 | ~8,000 | -66% |
| Context at 30 min session age | 156,000 | 76,000 | -51% |

v2.1.96 carries 4x more context per request. That context doesn't translate to better output; it mostly consists of accumulated conversation history from retry loops and tool result accumulation.

---

## Failure rates

| Metric | v2.1.96 | v2.1.63 |
|---|---|---|
| Empty/error responses (<20 tokens) | 0 (0.0%) | 13 (0.5%) |
| Substantial output (>200 tokens) | 191 (40.0%) | 936 (37.6%) |
| Deep responses (>1000 tokens) | 34 (7.1%) | 82 (3.3%) |

v2.1.96 produces slightly more deep responses per request (7.1% vs 3.3%), but v2.1.63 produces 2.4x more deep responses in absolute terms (82 vs 34) because it makes far more requests. v2.1.63 has a 0.5% empty response rate, likely from tool-only interactions.

---

## Code output quality

Raw line counts don't control for activity, but the *character* of the output does.

| Metric | v2.1.96 | v2.1.63 |
|---|---|---|
| Scaffold/placeholder files created | 15 | 0 |
| Dead code modules | 1 crate (roko-golem, 507 lines) | 0 |
| New files imported by production code | Partial (golem not wired) | All (context_provider, symbol_resolver, task_brief all used) |

v2.1.96 produced more raw volume but a significant fraction was scaffold code that is never imported by anything. v2.1.63 produced less volume but every file is functional and wired into the production code path.

---

## Summary

On a per-request basis, v2.1.63 is measurably more effective:

- **2.2x more output per dollar** spent
- **2.9x more output per unit of context** consumed
- **68% cheaper per request** ($0.097 vs $0.299)
- Produces **functional, integrated code** vs scaffolds and dead modules
- Keeps context at **68K avg** vs 280K, meaning each request is cheaper and the model has less noise to reason through

The qualitative difference matters more than the quantitative. v2.1.96 spent $152 and produced 17,000 lines where 15 files are placeholder scaffolds and an entire crate is dead code. v2.1.63 spent $255 and produced 5,800 lines of integrated, working code where every file is imported and used. Less volume, all of it real.
