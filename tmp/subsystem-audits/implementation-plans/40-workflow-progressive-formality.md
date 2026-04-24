# 40 — Progressive-Formality Workflow (5 Verbs)

This plan implements the workflow redesign suggested in
`tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md`. It
collapses 35+ subcommands into 5 verbs (`do`, `think`, `show`, `tune`,
`undo`) with progressive formality detection.

**This is a forward-looking plan.** It is **not** a bug fix. Land the
engine fixes (plans 10-34) first; this is a UX layer on top.

Source: doc 42 (workflow redesign suggestion).

---

## Why This Plan Is Last

Implementing this on a broken engine just adds another surface to the
problems. With:

- T2 (dead code) deleted
- T3 (security) hardened
- T4 (feedback) closed
- T5-35 (orchestrate.rs) extracted
- Plan 22 (dispatch) consolidated
- Plan 24 (run ledger) primary truth source

…this work becomes a thin UX surface. Without those, it's another
shadow runtime.

---

## Today's State

35+ subcommands. ~85 serve routes. 59 chat slash commands. 10 TUI tabs.
Each with its own state and discoverability story.

---

## Anti-Patterns Specific To This Plan

1. **No new internal state machine.** The 5 verbs are dispatched on top
   of existing engine pieces.
2. **No new dispatch path.** `do` is a thin dispatcher into the
   existing planner / runner / dispatch_agent_with chain.
3. **No "v2 schema" for work items that breaks existing PRD/plan
   files.** Work items wrap existing artifacts.
4. **No removing the old commands during this PR.** Add the new verbs
   alongside; deprecate later.
5. **No LLM call for intent classification on the hot path** unless
   bench shows < 200 ms p99. Heuristic first, LLM if ambiguous.

---

## Plan

### [ ] WF-1: Add `roko do "..."` command (alias of `roko run`)

The minimum viable change: rename `run` to `do`, keep `run` as a
deprecated alias.

**File**: `crates/roko-cli/src/main.rs` and the clap definitions.

```rust
#[derive(clap::Subcommand)]
enum Command {
    /// Execute work. Adapts: instant → managed.
    Do {
        prompt: String,
        #[clap(long)]
        plan: bool,
        #[clap(long)]
        review: bool,
        #[clap(long)]
        just_do_it: bool,
    },

    /// (deprecated, use `do`) Run a single prompt.
    #[clap(hide = true)]
    Run { prompt: String },
}
```

`run` warns "use `do` instead" and forwards.

### [ ] WF-2: Add `roko show` command (alias of `status`)

```rust
Show {
    target: Option<String>,    // work item ID or "learning" / "config"
    #[clap(long)]
    live: bool,                // open as TUI dashboard
}
```

Dispatches to `status` (no args), `dashboard` (`--live`), or
`learning show` (`learning`).

### [ ] WF-3: Add `roko think "..."` command (alias of research)

Calls the existing research path, returns analysis without execution.

### [ ] WF-4: Add `roko tune` and `roko undo`

`tune` aliases `config`. `undo` calls `git revert HEAD` or cancels the
last running work item.

### [ ] WF-5: Add intent classifier (heuristic)

**File**: `crates/roko-cli/src/intent.rs` (new)

```rust
pub fn classify(prompt: &str, workspace: &Workspace) -> Formality {
    // Heuristic-first: file count, keyword matching, prompt length
    let words = prompt.split_whitespace().count();
    let mentions_arch = ARCH_KEYWORDS.iter().any(|k| prompt.to_lowercase().contains(k));
    let mentions_typo = TYPO_KEYWORDS.iter().any(|k| prompt.to_lowercase().contains(k));

    match (words, mentions_arch, mentions_typo) {
        (_, _, true) if words < 30 => Formality::Trivial,
        (_, false, false) if words < 50 => Formality::Small,
        (_, false, false) => Formality::Medium,
        (_, true, _) => Formality::Large,
        _ => Formality::Medium,
    }
}

pub enum Formality {
    Trivial,    // direct exec, no plan, no approval
    Small,      // auto-plan, auto-execute, ask to commit
    Medium,     // show plan, wait for approval
    Large,      // create named work item, multi-agent, approval gates
}
```

For ambiguous cases (or when `--review` is set), call an LLM
classifier; cache the result.

### [ ] WF-6: Add `WorkItem` wrapper

**File**: `crates/roko-core/src/work_item.rs` (new)

```rust
pub struct WorkItem {
    pub id: String,
    pub status: WorkStatus,
    pub created: chrono::DateTime<chrono::Utc>,
    pub prompt: String,
    pub formality: Formality,

    pub prd: Option<PrdRef>,
    pub plan: Option<PlanRef>,
    pub tasks: Vec<TaskRef>,
    pub episodes: Vec<EpisodeRef>,
    pub git_branch: Option<String>,
    pub cost: CostSummary,
}

pub enum WorkStatus { Running, Paused, Done, Failed }
```

Persist under `.roko/work/<id>/` (one directory per work item).

### [ ] WF-7: Push-based progress in CLI

When `roko do "..."` runs interactively, render `RunnerEvent` /
`DashboardEvent` updates inline:

```
⟳ Planning... (opus, 2.1s)
✓ Plan: 4 tasks
  1. Extract auth types
  2. Implement JWT middleware  ← running (sonnet)
  3. Wire into pipeline
  4. Update tests
⟳ Task 2: editing auth/middleware.rs...
✓ Task 2: done (gate: compile ✓ test ✓ clippy ✓)
```

Subscribe to the event channel; format inline. Reuse the TUI's
formatter where possible.

### [ ] WF-8: HTTP API mirrors the 5 verbs

Add high-level routes in `roko-serve`:

```
POST /api/do           → start work
POST /api/think        → research / analyze
GET  /api/show         → list work items, sessions, learning state
GET  /api/show/:id     → detail of work item
POST /api/tune         → update config / thresholds / routing
POST /api/undo         → revert / cancel / pause
GET  /api/stream/:id   → SSE event stream
```

These routes wrap the existing 85 routes; they don't replace them.

---

## Phasing

This plan is **best landed in phases**, each shippable independently:

| Phase | Items | User-visible improvement |
|---|---|---|
| 0 | WF-1, WF-2, WF-3 | New verbs available; old commands deprecated |
| 1 | WF-7 | CLI shows live progress instead of being silent |
| 2 | WF-5 (heuristic only) | `do` adapts behavior to prompt |
| 3 | WF-6 | Work items in `.roko/work/`; `roko show` lists |
| 4 | WF-8 | HTTP API mirrors |
| 5 | WF-5 (LLM classifier) | Better ambiguous-case handling |

Each phase = ~2 sessions. Total: ~12 sessions for the full vision.

---

## Status

- [ ] Phase 0 — `do` / `show` / `think` aliases
- [ ] Phase 1 — Push-based progress
- [ ] Phase 2 — Heuristic intent classifier
- [ ] Phase 3 — WorkItem persistence
- [ ] Phase 4 — HTTP API mirror
- [ ] Phase 5 — LLM classifier fallback

**Don't start until plans 10-34 are mostly done.** UX layer on broken
engine is wasted work.
