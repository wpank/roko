# Wave C: CLI Redesign (5 Verbs)

## Root Cause

35+ subcommands at 3-4 levels organized by internal architecture. The user's mental model
is "I want to do something" — not "create a PRD, draft it, generate a plan, run the plan."

The fix: collapse to 5 verbs matching user intent: do, think, show, tune, undo.

---

## Critical Discovery: WorkflowEngine ALREADY EXISTS

**Audit finding**: `crates/roko-runtime/src/workflow_engine.rs` (1876 lines) already
implements `WorkflowEngine` with a `run()` method. It's used by `shared_runs.rs`.

The original C1 task proposed building a NEW WorkflowEngine facade. **This is wrong.**
The real work is:
1. Expose the existing WorkflowEngine via `roko do`
2. Add a ScopeResolver (extending existing `PlanComplexity`)
3. Wire the CLI command

This collapses C1+C2+C3 into a single coherent task.

---

## Critical Prerequisite: Fix Default Agent

**Root cause**: The default agent in roko.toml is `cat` (echoes input, reports success).
Any execution will appear to succeed while doing nothing.

**Fix**: WorkflowEngine initialization checks for `cat` agent and overrides with the
actual dispatch backend (Claude CLI, API, etc.).

---

## Task C1: `roko do` Command (Uses Existing WorkflowEngine)

**Current branch status - 2026-05-05**: `roko do` is wired on `wp-arch2` as a
WorkflowEngine template selector with heuristic classification in
`crates/roko-cli/src/scope_resolver.rs`. It does not yet implement LLM-first classification,
full PRD/plan generation for medium/complex work, side-by-side compare execution, or complete
work-item resume semantics. See taskrunner task 057 for exact gaps.

**Root cause**: The user's primary action requires 4 separate commands today.

**What already exists** (from audit):
- `WorkflowEngine` in roko-runtime (1876 lines) with `run()` method
- `PlanComplexity` enum in roko-gate: Trivial/Simple/Standard/Complex
- `RunConfig` (28 fields) already holds `Arc<RokoConfig>`
- `WorkflowEngine` is already used by `shared_runs.rs` for HTTP-triggered runs

**What to build** (~150-200 new lines):

1. **ScopeResolver** — extends PlanComplexity, determines execution strategy:
   ```rust
   // crates/roko-cli/src/scope_resolver.rs
   pub struct ScopeResolver;

   impl ScopeResolver {
       /// LLM-first classification (haiku ~$0.001, 500ms) with heuristic fallback
       pub async fn resolve(prompt: &str, config: &RokoConfig) -> PlanComplexity {
           // Try fast LLM classification first
           if let Ok(complexity) = Self::llm_classify(prompt, config).await {
               return complexity;
           }
           // Heuristic fallback
           Self::heuristic_classify(prompt)
       }

       fn heuristic_classify(prompt: &str) -> PlanComplexity {
           let words = prompt.split_whitespace().count();
           let lower = prompt.to_lowercase();

           // Trivial: short prompt, obvious action
           if words < 15 && ["fix typo", "rename", "update comment"].iter()
               .any(|k| lower.contains(k)) {
               return PlanComplexity::Trivial;
           }
           // Complex: long prompt, architectural keywords
           if words > 50 || ["refactor", "redesign", "architecture"].iter()
               .any(|k| lower.contains(k)) {
               return PlanComplexity::Complex;
           }
           // Standard: multi-file indicators
           if ["add feature", "implement", "new endpoint"].iter()
               .any(|k| lower.contains(k)) {
               return PlanComplexity::Standard;
           }
           PlanComplexity::Simple
       }
   }
   ```

2. **`roko do` CLI command**:
   ```
   roko do "<prompt>"             # auto-classify, execute
   roko do --plan "<prompt>"      # force plan generation
   roko do --yes "<prompt>"       # skip approval for standard/complex
   roko do --ghost "<prompt>"     # dry-run: show plan + cost estimate
   roko do --compare "<prompt>"   # run naive vs cascade side-by-side
   roko do --continue [work-id]   # resume interrupted work
   roko do --no-cascade           # disable cascade routing (for demo comparison)
   roko do                        # no args: offer to resume in-progress work
   ```

3. **Implementation**: Add `Do` variant to CLI Command enum. Handler calls existing
   `WorkflowEngine::run()` with appropriate RunConfig built from args + ScopeResolver.

**Backwards compat**: `roko run "<prompt>"` becomes alias for `roko do "<prompt>"`.

**Verification**:
```bash
cargo run -p roko-cli -- do "add a function that checks if a number is prime"
# Should: classify, plan, execute, stream inline output, gate, report
```

---

## Task C2: `roko show` Command

**Root cause**: State inspection scattered across 6+ commands. Should be one.

**Design** (independent — no engine dependency, can start immediately):
```
roko show                  # overview: work items, agents, costs, learning
roko show costs            # cost breakdown by model, task, time
roko show agents           # active agents with roles and status
roko show knowledge        # what the system has learned
roko show plans            # plans in progress
roko show learning         # routing confidence, model performance, gate drift
roko show history          # recent work items chronologically
roko show <work-id>        # detail on specific work item
roko show --live           # alias for dashboard (TUI)
```

**Output format** (using InlineTerminal primitives from B2):
```
◆ roko · workspace: /path/to/project

│ WORK ITEMS
│   auth-redesign    Running   3/7 tasks   $0.42
│   fix-login-bug    Done      1/1 tasks   $0.03
│
│ AGENTS
│   implementer    claude-sonnet    active
│   reviewer       gpt-4o-mini      idle
│
│ LEARNING
│   routing confidence: 0.78 (↑ from 0.62)
│   31 episodes today · $4.20 total spend
│
└ Try: roko show costs · roko show learning · roko show <id>
```

**Verification**: `cargo run -p roko-cli -- show` produces formatted output with real
data from `.roko/`. No "Coming soon" placeholders.

---

## Task C3: `roko think` and `roko tune`

**`roko think`** — research without action:
```
roko think "how does auth work in this codebase?"
roko think "what are best practices for rate limiting?"
```
Wraps: `research topic`, `research search`, `knowledge query`, `explain`.
Returns analysis, no code changes.

**`roko tune`** — adjust behavior:
```
roko tune routing         # model routing preferences
roko tune gates           # validation strictness
roko tune budget          # cost limits
roko tune model sonnet    # default model
```
**Critical**: These commands MUST actually write config. No confirmation theater.

**Verification**:
```bash
cargo run -p roko-cli -- tune model haiku
grep model roko.toml  # should show haiku
```

---

## Task C4: Work Items as First-Class Objects

**Root cause**: Users interact with opaque file paths (`.roko/state/executor.json`)
and must pass `--resume` with those paths.

**Design**:
```rust
pub struct WorkItem {
    pub id: String,           // "auth-redesign" (auto-generated or user-named)
    pub status: WorkStatus,   // Running | Paused | Done | Failed
    pub prompt: String,       // Original user intent
    pub complexity: PlanComplexity,
    pub cost: CostSummary,
    pub created: DateTime<Utc>,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub commits: Vec<String>,  // Git commit hashes (for undo)
    // Internal refs:
    plan: Option<PlanRef>,
    executor_snapshot: Option<PathBuf>,
}
```

Stored in `.roko/work/` as JSON. `roko do --continue` lists them.
`roko show` displays them. User never sees executor.json.

**Audit note**: This is 80% UX sugar over existing state persistence. The executor
snapshot already exists — WorkItem is a user-friendly wrapper around it.

**Verification**: Run `roko do "..."`, interrupt it, run `roko do` again.
Should offer to resume by name.

---

## Task C5: `roko undo` Command

**Root cause**: No safe reversal mechanism. If `roko do` produces bad code, the user
must manually `git checkout` or `git stash`.

**Design**:
```
roko undo                     # undo last work item's changes
roko undo <work-id>           # undo specific work item
roko undo --soft              # stage reversal but don't commit
roko undo --dry-run           # show what would be reverted
```

**Implementation**: Each WorkItem (from C4) records git commits it produced.
`roko undo` generates a revert.

**Verification**: Run `roko do "add function"`, then `roko undo`. Changes reverted.

---

## Task C6: `POST /api/do` HTTP Route

**Root cause**: The demo app needs a universal intent endpoint. Currently it triggers
plan execution via `POST /api/plans/:id/execute` which requires a pre-existing plan.

**Design**:
```
POST /api/do
Content-Type: application/json

{
  "prompt": "add a health check endpoint",
  "complexity": null,    // auto-classify
  "flags": { "yes": true, "no_cascade": false }
}
```

Returns: `202 Accepted` with `{ "work_item_id": "...", "stream_url": "/api/events/stream?filter=..." }`

Internally calls existing `WorkflowEngine::run()`.

**Verification**: `curl -X POST http://localhost:6677/api/do -d '{"prompt":"..."}'`
returns work item ID, and SSE stream shows events.

---

## Dependency Graph

```
C1 (roko do + ScopeResolver) → C4 (work items) → C5 (roko undo)
                              → C6 (POST /api/do)

C2 (roko show) ─── independent (reads .roko/ state, no engine dependency)
C3 (think/tune) ── independent (wraps existing commands)
```

C2 and C3 can start immediately. C1 is the prerequisite for C4, C5, C6.

**Cross-wave note**: C1's inline output comes from B2 (InlineTerminal wiring).
C1 depends on B2 for pretty output, but can work with basic stderr fallback first.

---

## Backwards Compatibility

All existing commands remain as aliases:
```
hint: try 'roko do "fix the bug"' (roko run still works)
```
No command is removed in this wave.
