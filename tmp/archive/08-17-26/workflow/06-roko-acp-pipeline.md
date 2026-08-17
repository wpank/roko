# Roko ACP Pipeline (Per-Prompt Workflow)

The ACP pipeline is a **new** multi-agent workflow system in `crates/roko-acp/`. It handles per-prompt workflows from editors (JetBrains, Zed, Neovim, VS Code) via the Agent Client Protocol.

This is **separate from** the plan-based `orchestrate.rs` system. It's what you get when typing a prompt in an editor vs running `roko plan run`.

## Architecture: Pure State Machine + Side-Effect Runner

```
pipeline.rs  -- Pure state machine (Events in, Actions out, no I/O)
workflow.rs  -- WorkflowRun wrapper (timing, cost, metadata)
runner.rs    -- Side-effect executor (spawns agents, runs gates, commits)
```

## Pipeline Phases

```rust
pub enum PipelinePhase {
    Pending,        // Created but not started
    Strategizing,   // Strategist agent analyzing prompt
    Implementing,   // Implementer agent writing code
    AutoFixing,     // Auto-fixer patching gate failures
    Gating,         // Running gates (compile, test, clippy)
    Reviewing,      // Reviewer agent analyzing changes
    Committing,     // Creating git commit
    Complete,       // Finished successfully
    Halted { reason }, // Stopped (timeout, budget, cancel)
    Cancelled,      // User cancelled
}
```

## Events and Actions

```rust
// Events (input to state machine)
pub enum PipelineEvent {
    Start, StrategyComplete, StrategySkipped,
    AgentCompleted, AgentFailed,
    GatesPassed, GateFailed,
    ReviewApproved, ReviewRevise,
    CommitDone,
    Timeout, BudgetExceeded, UserCancel,
}

// Actions (output from state machine)
pub enum PipelineAction {
    SpawnStrategist, SpawnImplementer, SpawnAutoFixer, SpawnReviewer,
    RunGates, Commit,
    Done, Halt,
}
```

## Workflow Templates

```rust
pub enum WorkflowTemplate {
    Express,   // Implement -> Gate -> Commit (fastest)
    Standard,  // Implement -> Gate -> Review -> Commit
    Full,      // Strategy -> Implement -> Gate -> Review -> Commit
}
```

### Auto-Selection Heuristic

`WorkflowTemplate::auto_select(prompt)`:
- Short prompts (<15 words) with "fix"/"typo"/"rename"/"update" -> Express
- Long prompts (>50 words) or "refactor"/"architecture"/"redesign" -> Full
- Everything else -> Standard

## Flow Diagrams

### Express (Implement -> Gate -> Commit)
```
Start -> Implementing -> Gating -> Committing -> Complete
                           |
                      (gate fail)
                           |
                      AutoFixing -> Gating (retry, up to max_iterations)
```

### Standard (Implement -> Gate -> Review -> Commit)
```
Start -> Implementing -> Gating -> Reviewing -> Committing -> Complete
                           |          |
                      (gate fail)  (revise)
                           |          |
                      AutoFixing  Implementing (with review feedback)
```

### Full (Strategy -> Implement -> Gate -> Review -> Commit)
```
Start -> Strategizing -> Implementing -> Gating -> Reviewing -> Committing -> Complete
                                           |          |
                                      (gate fail)  (revise)
                                           |          |
                                      AutoFixing  Implementing
```

## Runner: Side Effects

`runner::run_workflow_pipeline()` drives the state machine:

```rust
pub async fn run_workflow_pipeline(
    session_id: &str,
    prompt: &str,
    workdir: &Path,
    config: PipelineConfig,
    cancel_token: CancelToken,
    event_sender: mpsc::Sender<CognitiveEvent>,
) -> anyhow::Result<()>
```

### Agent Spawn
Currently spawns `claude --print --dangerously-skip-permissions` as subprocess.
Each role gets a different prompt:
- **Strategist**: "Analyze this task and create an architectural brief..."
- **Implementer**: "Implement the following: {prompt}" + strategy brief + error context + review feedback
- **AutoFixer**: "Fix the following build errors: {error_output}"
- **Reviewer**: "Review the following changes..." with configurable strictness

### Gate Execution
Sequential: `cargo build` -> `cargo test` -> `cargo clippy`
Each is a shell subprocess. Gates are individually toggleable via config.

### Review Strictness
Configurable: `quick` | `standard` | `thorough`
Verdict parsing: looks for "APPROVED" in output, extracts bullet-point findings for revision.

### Commit
`git add -A` + `git commit -m "feat: {prompt}"` (truncated to 72 chars).

## Session Integration

`AcpSession` config options exposed to editors:

```rust
pub struct SessionConfigState {
    pub workflow: String,           // "none" | "express" | "standard" | "full" | "auto"
    pub review_strictness: String,  // "none" | "quick" | "standard" | "thorough"
    pub max_iterations: u32,        // 1-3
    pub clippy_enabled: bool,
    pub tests_enabled: bool,
}
```

When `session/prompt` arrives:
1. Check workflow config
2. If "auto" -> `WorkflowTemplate::auto_select()`
3. If explicit -> `WorkflowTemplate::from_config()`
4. If template selected -> `run_workflow_pipeline()` (multi-agent)
5. If "none" -> single-agent dispatch (existing behavior)

## Plan Progress Updates

After every phase transition, ACP `session/update` with structured entries:
```
[Strategy]       Completed  (or Pending/InProgress)
[Implementation] InProgress
[Gates]          Pending
[Review]         Pending
[Commit]         Pending
```

Editors display this as a progress tracker.

## Current Limitations

1. Agent dispatch bypasses `roko-agent` crate -- uses raw `claude` CLI subprocess, not the provider system
2. File change detection is heuristic (counting "Edit:"/"Create:" strings) not git diff
3. Cost/token tracking fields initialized but never updated
4. `GateResult`/`ReviewFinding` structs defined but unused (raw strings instead)
5. Commit messages are simplistic
6. Review verdict parsing is basic
