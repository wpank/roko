# Roko Development Anti-Patterns

Hard-won lessons from building 3+ parallel runtimes that don't share anything. Read this before writing code.

## The Cardinal Rule

> **One feature, one implementation, one code path.**
>
> If you're about to write something that already exists elsewhere in the codebase in a different form, STOP. Find the existing version and either use it, improve it, or replace it -- but never duplicate it.

---

## Anti-Pattern 1: "Just Shell Out To Claude"

**What it looks like:**
```rust
Command::new("claude")
    .arg("--print")
    .arg("--dangerously-skip-permissions")
    .arg(prompt)
    .output()
```

**Why it's wrong:** This bypasses the entire agent provider system (`roko-agent`), model routing (`CascadeRouter`), prompt assembly (`roko-compose`), safety layer, feedback recording, and observability. You get none of the system's capabilities. There are already 4 different "spawn claude" paths in the codebase and they all do it differently.

**What to do instead:** Use (or build toward) a single `ModelCallService` that all callers go through. If that doesn't exist yet, at minimum use `roko-agent`'s provider system (`create_agent_for_model()` or `spawn_agent_scoped()`).

**Real example:** `roko-acp/src/runner.rs:run_claude_cli()` uses bare `claude --print` with no model selection, no system prompt, no streaming. Meanwhile `roko-acp/src/bridge_events.rs:run_claude_cognitive_task()` uses `claude --print --output-format stream-json --model <model> --system-prompt <prompt>`. Meanwhile `runner/agent_stream.rs:spawn_agent()` uses `CliProviderConfig::build_invocation()`. Three different claude spawns in one codebase.

**Status (2026-04-28):** Resolved. `ModelCallService` (P1A, `roko-agent/src/model_call_service.rs`) provides the single dispatch point. `WorkflowEngine` (P2D) routes all callers through it. The legacy `Command::new("claude")` in `bridge_events.rs` was removed in batch P4B.

---

## Anti-Pattern 2: "Inline Prompt Strings"

**What it looks like:**
```rust
let prompt = format!(
    "You are the **Architect Reviewer**. Focus on:\n\
     1. Architecture and design pattern adherence\n\
     2. API contract correctness\n..."
);
```

**Why it's wrong:** Prompts should be assembled by the prompt system (`roko-compose::SystemPromptBuilder`), not hardcoded in the caller. Inline prompts can't benefit from knowledge injection, playbook inclusion, section effectiveness tuning, token budget enforcement, or any other prompt quality feature. They also can't be A/B tested or improved without code changes.

**What to do instead:** Use role templates from `roko-compose/src/templates/` and the `SystemPromptBuilder`'s 9-layer assembly. Role definitions belong in `core_roles.toml`, not in inline format strings.

**Real example:** `roko-acp/src/runner.rs` has inline prompts for strategist, implementer, auto-fixer, and reviewer roles. Meanwhile `roko-compose/src/templates/` has proper template modules for all these roles that nobody in the ACP pipeline calls.

**Status (2026-04-28):** Resolved. `PromptAssemblyService` (P1B, `roko-compose/src/prompt_assembly_service.rs`) wraps `SystemPromptBuilder` with role-based assembly. All workflow callers go through the service instead of building inline format strings. Role definitions sourced from templates.

---

## Anti-Pattern 3: "Build Another Runtime"

**What it looks like:** Adding plan execution, multi-task DAG walking, or agent orchestration features directly into a specific entry point (ACP, CLI command, HTTP route) instead of a shared engine.

**Why it's wrong:** This is how we got 3 runtimes:
1. `orchestrate.rs` (dead, 21K lines) -- the original, full-featured but never called
2. `runner/event_loop.rs` (live, 3K lines) -- the replacement, missing half the features
3. `roko-acp/runner.rs` (live, 800 lines) -- the ACP version, missing 90% of features

Each one duplicates: state management, agent dispatch, gate running, retry logic, persistence. None of them share code with the others.

**What to do instead:** Build shared services (`ModelCallService`, `PromptAssemblyService`, `FeedbackService`, `GateService`) that any entry point can compose. Then build ONE execution engine (`WorkflowEngine`) that all entry points call. See `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`.

**Real example:** The other Claude session was about to wire `roko-orchestrator` into the ACP pipeline runner, creating runtime #4 -- a hybrid of the ACP state machine with roko-orchestrator's DAG executor, still using bare `claude --print` for agent dispatch.

**Status (2026-04-28):** Resolved. `WorkflowEngine` (P2D, `roko-orchestrator/src/workflow_engine.rs`) is the single execution engine. It composes `TaskScheduler` (P2B) for DAG ordering, `EffectDriver` (P2C) for side effects, and `PipelineState` (P2A) for the state machine. The three legacy runtimes (`orchestrate.rs`, `event_loop.rs`, `roko-acp/runner.rs`) are superseded.

---

## Anti-Pattern 4: "Add Features to the Wrong Layer"

**What it looks like:** Adding multi-role review logic, retry policies, or gate failure classification directly into an effect driver / side-effect executor.

**Why it's wrong:** The ACP pipeline's best design decision was separating the pure state machine (`pipeline.rs`) from the side-effect driver (`runner.rs`). Decisions (what to do next) belong in the state machine. Effects (how to do it) belong in the driver. When you add decision logic to the driver, you lose testability and the state machine becomes incomplete.

**What to do instead:**
- **State machine** decides: which role reviews, how many reviewers, what happens on failure, when to retry, when to halt
- **Effect driver** executes: spawn the agent, parse the output, run the gate, make the commit
- The state machine is pure (no I/O, no async), fully unit-testable with no mocks

**Real example:** The multi-role review was added to `runner.rs` (the driver) as `run_multi_role_review()`. The pipeline state machine doesn't know about it -- it still thinks there's one reviewer. The decision "use two reviewers for thorough mode" should be in the state machine, emitting `SpawnArchitectReviewer` and `SpawnAuditorReviewer` actions that the driver executes.

**Status (2026-04-28):** Resolved. `PipelineState` (P2A, `roko-orchestrator/src/pipeline_state.rs`) is the pure state machine that emits `Action` variants. `EffectDriver` (P2C, `roko-orchestrator/src/effect_driver.rs`) executes those actions with no decision logic. Review role selection is now a state machine concern.

---

## Anti-Pattern 5: "Hardcoded Role Behavior"

**What it looks like:**
```rust
if config.review_strictness == "thorough" {
    // hardcoded architect + auditor
} else {
    // hardcoded single reviewer
}
```

**Why it's wrong:** Role definitions, which roles participate in which workflow, and what each role focuses on are all configuration concerns. They should come from role manifests (`core_roles.toml`) and workflow templates, not from if/else branches in the runner.

**What to do instead:** Define roles declaratively. `WorkflowTemplate::Full` specifies `review_roles: [Architect, Auditor]`. `WorkflowTemplate::Standard` specifies `review_roles: [QuickReviewer]`. The execution engine iterates the role list generically. Adding a new reviewer role means adding a config entry, not changing code.

---

## Anti-Pattern 6: "Feedback/Learning as Afterthought"

**What it looks like:** Running an agent, getting the output, and not recording anything about what happened -- no episode, no cost tracking, no routing observation.

**Why it's wrong:** The whole point of the system is that it learns from its runs. If you bypass the feedback loop, the CascadeRouter never learns which models work, the knowledge store never accumulates, and the playbook store stays empty. The system can't improve.

**What to do instead:** Every model call goes through `ModelCallService`, which automatically emits events. `FeedbackService` consumes those events and fans out to all learning sinks. No caller needs to manually record feedback.

**Real example:** The ACP pipeline runner spawns agents and runs gates but records zero feedback. `WorkflowRun.total_cost_usd` and `total_tokens` are initialized to 0 and never updated. No episodes are written.

**Status (2026-04-28):** Resolved. `FeedbackService` (P1C, `roko-learn/src/feedback_service.rs`) subscribes to `RuntimeEvent`s emitted by `ModelCallService` and `GateService`. All learning sinks (CascadeRouter, EpisodeLogger, efficiency tracker, playbook store) are fed automatically. No caller needs manual feedback recording.

---

## Anti-Pattern 7: "Copy-Paste Between Runtimes"

**What it looks like:** Taking a feature from `orchestrate.rs` (like gate failure classification) and reimplementing it in `runner/event_loop.rs` or `roko-acp/runner.rs` with slight variations.

**Why it's wrong:** Now you have 2-3 versions of the same logic that will diverge over time. Bug fixes only apply to one copy. Improvements don't propagate.

**What to do instead:** Extract the shared logic into a service or utility in the appropriate crate:
- Gate failure classification → `roko-gate`
- Retry/replan decisions → `roko-orchestrator` or a new `RepairPolicyEngine`
- Prompt assembly → `roko-compose`
- Model routing → `roko-learn::CascadeRouter`

**Status (2026-04-28):** Resolved. Foundation services (P1A-P1D) are the single implementations. `ModelCallService`, `PromptAssemblyService`, `FeedbackService`, and `GateService` each live in their owning crate and are composed by `WorkflowEngine`. No runtime-specific copies remain.

---

## Anti-Pattern 8: "Prefixing Unused Parameters with `_`"

**What it looks like:**
```rust
fn run_multi_role_review(
    _config: &PipelineConfig,  // was config, added _ to suppress warning
```

**Why it's wrong (in context):** If you just added a parameter and immediately had to prefix it with `_`, that's a code smell. Either the function should use the parameter (it was added for a reason), or the function signature is wrong. In this specific case, `config.review_strictness` was hardcoded as "thorough" in the caller, so the function should have used `config` -- or the function shouldn't exist and the behavior should be in the state machine.

---

## Anti-Pattern 9: "Bolting Multi-Task onto Single-Task"

**What it looks like:** Taking a pipeline designed for single prompts (ACP's `PipelineState`) and trying to add DAG-based multi-task execution to it by wiring in `roko-orchestrator`.

**Why it's wrong:** The single-prompt pipeline and the multi-task plan executor have fundamentally different state machines. A single prompt goes: implement → gate → review → commit. A multi-task plan goes: [for each task in DAG order: implement → gate] → merge. Bolting the second onto the first creates a Frankenstein that doesn't handle either case well.

**What to do instead:** Design the state machine from scratch to handle both cases via `WorkflowTemplate` variants:
- `Express` / `Standard` / `Full`: single-prompt workflows
- `PlanExecution`: multi-task DAG workflow

Both share the same effect driver and services, but the state machine phase transitions are different.

---

## Anti-Pattern 10: "God Files"

**What it looks like:**
- `orchestrate.rs`: 21,577 lines, owns dispatch + prompts + routing + feedback + gates + replan + safety + affect + pheromones + ...
- `event_loop.rs`: 3,036 lines, owns dispatch + events + gates + feedback + merge + persistence + ...
- `bridge_events.rs`: 1,856 lines, owns wire format + error types + event mapping + two LLM backends + 40+ slash commands + ...

**Why it's wrong:** When one file owns 10+ responsibilities, every change risks breaking unrelated functionality. It's impossible to understand, test, or refactor safely.

**What to do instead:** Each file/module owns ONE responsibility. The execution engine is a thin orchestrator that delegates to focused services. See Phase 0 of the unified plan: `ModelCallService`, `PromptAssemblyService`, `FeedbackService`, `PersistenceService`, `GateService` -- each owned by one module.

**Status (2026-04-28):** Resolved. The 14 new modules created across P0-P3 each own a single responsibility and range from 1-3K lines. `WorkflowEngine` is a thin coordinator (~500 lines) that delegates to `PipelineState`, `TaskScheduler`, `EffectDriver`, and the four foundation services. The 21K `orchestrate.rs` and 68K+40K cognitive layer files are superseded by focused service modules.

---

## Quick Decision Checklist

Before writing code, check:

- [ ] Am I spawning a process or making an HTTP call to an LLM? → Use `ModelCallService`
- [ ] Am I building a prompt? → Use `PromptAssemblyService` / `SystemPromptBuilder`
- [ ] Am I deciding what happens next in a workflow? → Put it in the state machine, not the driver
- [ ] Am I recording what happened? → Ensure `FeedbackService` sees it
- [ ] Does similar code already exist in another runtime? → Use it, don't duplicate it
- [ ] Am I adding a feature to one entry point? → Add it to a shared service instead
- [ ] Is my file getting above 500 lines? → Extract a service
