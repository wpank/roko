# Prompt for Redirecting the Other Session

Copy everything below the line and paste it into the other Claude session.

---

STOP. Do not continue adding features to the ACP pipeline runner. What you just did (multi-role reviews in runner.rs, and the planned roko-orchestrator wiring) is the exact anti-pattern we're trying to fix.

## The Problem

There are already 3 runtimes in this codebase that don't share code:

1. `orchestrate.rs` (21K lines, dead -- never called from any CLI path)
2. `runner/event_loop.rs` (3K lines, live -- used by `roko plan run`)
3. `roko-acp/src/runner.rs` (800 lines, live -- used by `roko acp` workflow mode)

You are about to create runtime #4 by bolting roko-orchestrator's DAG executor onto the ACP runner, which still uses bare `claude --print --dangerously-skip-permissions` with no model selection, no system prompts, no provider system, no feedback recording, and inline prompt strings.

The multi-role review you just added has these specific problems:
- It's in the driver (`runner.rs`), not the state machine (`pipeline.rs`) -- the pipeline doesn't know about multiple reviewers
- It uses inline prompt strings instead of the role template system in `roko-compose/src/templates/`
- It spawns agents via `run_claude_cli()` which bypasses the entire provider system
- It records zero feedback (no episodes, no cost tracking, no routing observation)
- `_config` is immediately unused (prefixed with `_`)

## What To Do Instead

Read these files (in order):

1. `tmp/workflow/ANTI-PATTERNS.md` -- Development anti-patterns to avoid
2. `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md` -- The implementation plan for ONE unified runtime
3. `tmp/workflow/07-comparison.md` -- What each runtime has/lacks

## Immediate Actions

1. **Revert** the multi-role review changes to `roko-acp/src/runner.rs`. The feature belongs in the unified engine, not the current ACP runner.

2. **Start Phase 0.1 of the unified plan**: Build `ModelCallService` -- a single trait that every model call in the system goes through. This is the foundation everything else depends on. It replaces the 4 different "spawn claude" paths. Define the trait in `roko-runtime` (or a new `roko-inference` crate):

```rust
#[async_trait]
pub trait ModelCallService: Send + Sync {
    async fn complete(&self, req: ModelCallRequest) -> Result<ModelCallResponse>;
    async fn stream(&self, req: ModelCallRequest) -> Result<ModelCallStream>;
    async fn probe(&self, provider_id: &str) -> ProviderProbeResult;
}
```

Every caller -- runner, ACP, HTTP routes, research, dreams, neuro -- uses this one service. The service handles: model→provider resolution, credential lookup, provider adapter dispatch, cost tracking, event emission.

3. **Do NOT**:
   - Add features to `roko-acp/src/runner.rs` (it will be replaced)
   - Wire `roko-orchestrator` into the ACP pipeline (the unified engine will handle both)
   - Duplicate logic from `orchestrate.rs` or `event_loop.rs` into another file
   - Write inline prompt strings (use `roko-compose` templates)
   - Spawn `claude` via `Command::new("claude")` directly

The full implementation plan has 80+ granular tasks across 7 phases. Phase 0 (foundation services) must be built first. Multi-role reviews come naturally in Phase 1 when the unified `PipelineState` supports `WorkflowTemplate::Full` with configurable `review_roles: [Architect, Auditor]`.
