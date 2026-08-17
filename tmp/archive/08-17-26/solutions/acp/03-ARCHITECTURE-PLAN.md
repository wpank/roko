# ACP Architecture Plan — Shared & Extensible

## Design Philosophy

The goal is NOT to add more code to `roko-acp`. It's to **wire ACP into existing crates**
so it participates in the same learning/safety/knowledge ecosystem as `orchestrate.rs`.

```
Before (current):
  orchestrate.rs ──→ roko-learn, roko-compose, roko-agent/safety, roko-neuro
  roko-acp ──→ (isolated: raw subprocess, static prompts, no feedback)

After (target):
  orchestrate.rs ──┐
                   ├──→ Shared services (FeedbackService, PromptAssembly, SafetyLayer)
  roko-acp ────────┘
```

## Shared Service Layer

### 1. FeedbackService (new, small)

A thin facade that both orchestrate.rs and ACP call:

```rust
// crates/roko-learn/src/feedback.rs (or roko-core)
pub struct FeedbackService {
    episode_logger: EpisodeLogger,
    cascade_router: CascadeRouter,
    efficiency_tracker: EfficiencyTracker,
}

impl FeedbackService {
    pub fn record_dispatch(&self, event: DispatchEvent) { ... }
    pub fn record_outcome(&self, event: OutcomeEvent) { ... }
    pub fn suggest_model(&self, task_hint: &str) -> ModelSuggestion { ... }
}
```

Both runtimes call the same service. No duplication.

### 2. PromptAssemblyService (extract from orchestrate.rs)

Currently the 9-layer SystemPromptBuilder is called inline in orchestrate.rs.
Extract to a callable service:

```rust
// crates/roko-compose/src/assembly_service.rs
pub struct PromptAssemblyService { ... }

impl PromptAssemblyService {
    pub fn build_for_session(&self, params: SessionPromptParams) -> String { ... }
}

pub struct SessionPromptParams {
    pub mode: AgentMode,         // code/plan/research
    pub role: Option<AgentRole>, // strategist/implementer/reviewer
    pub context: Vec<ContextItem>,
    pub playbooks: Vec<Playbook>,
    pub knowledge: Vec<KnowledgeEntry>,
}
```

ACP calls this instead of its static `CODE_MODE_SYSTEM_PROMPT` strings.

### 3. SafetyGate (extract from roko-agent)

```rust
// Already exists as AgentContract in roko-agent/src/safety/
// Just needs to be callable from ACP session context

pub fn enforce_contract(session: &AcpSession, action: &Action) -> Result<()> {
    let contract = load_contract_for_mode(session.mode)?;
    contract.check_pre(action)?;
    Ok(())
}
```

## Wiring Pattern (Incremental)

Each integration follows the same pattern:

1. **Add crate dependency** to `roko-acp/Cargo.toml`
2. **Initialize service** in `run_acp_server()` (the entry point)
3. **Pass service handle** to bridge_events / runner via struct field or Arc
4. **Call service** at the appropriate hook point
5. **Test** with integration test that verifies cross-crate flow

### Hook Points in ACP

```
handler.rs: session/prompt received
  ├─ [HOOK: safety check] enforce_contract(session, prompt)
  │
  ├─ bridge_events.rs: dispatch agent
  │   ├─ [HOOK: prompt assembly] PromptAssemblyService::build_for_session()
  │   ├─ [HOOK: model routing] FeedbackService::suggest_model()
  │   ├─ [HOOK: dispatch via roko-agent] instead of raw subprocess
  │   └─ [HOOK: per-turn logging] FeedbackService::record_dispatch()
  │
  ├─ runner.rs: pipeline phase complete
  │   ├─ [HOOK: outcome logging] FeedbackService::record_outcome()
  │   └─ [HOOK: efficiency event] track tokens/latency/cost
  │
  └─ session.rs: session end
      └─ [HOOK: episode close] FeedbackService::close_episode()
```

## Extensibility Points

### For Future Workflow Types
Pipeline state machine already uses trait-like dispatch:
```rust
pub fn step(&self, event: PipelineEvent) -> (PipelinePhase, PipelineAction)
```
New templates just define different phase sequences. No code change to FSM.

### For Additional Providers
Provider routing goes through `resolve_model()` in roko-core. Adding a new provider:
1. Add variant to `ProviderKind` enum
2. Implement HTTP dispatch in roko-agent
3. ACP automatically picks it up via config

### For New Slash Commands
Commands are defined in `session.rs` as a Vec. Adding one:
1. Add entry to `available_commands()` vec
2. Add match arm in `execute_slash_command()`
3. Map to existing CLI command or new handler

### For New Config Options
Config options defined in `config_options()` method. Adding one:
1. Add field to `SessionConfigState` struct
2. Add entry to `config_options()` vec
3. Handle in `session/config/update` method

## Dependency Graph (After Integration)

```
roko-acp/Cargo.toml:
  roko-core = { path = "../roko-core" }       # (already)
  roko-agent = { path = "../roko-agent" }     # (already)
  roko-gate = { path = "../roko-gate" }       # (already)
  roko-compose = { path = "../roko-compose" } # NEW — for PromptAssemblyService
  roko-learn = { path = "../roko-learn" }     # NEW — for FeedbackService
  roko-neuro = { path = "../roko-neuro" }     # NEW — for knowledge queries
```

## What NOT to Do

- **Don't reimplement SystemPromptBuilder** — use roko-compose
- **Don't add a second episode logger** — use roko-learn's
- **Don't fork CascadeRouter** — call the shared one
- **Don't add safety checks inline** — use roko-agent safety layer
- **Don't make ACP "aware" of other runtimes** — use shared services at the boundary
