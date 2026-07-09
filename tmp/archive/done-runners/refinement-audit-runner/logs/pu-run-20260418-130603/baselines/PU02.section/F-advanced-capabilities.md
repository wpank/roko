# F — Advanced Capabilities (Doc 12 + Doc 00 advanced)

Parity analysis of `docs/02-agents/12-extensibility.md` and advanced sections of `00-agent-trait.md` vs actual codebase.

---

## F.01 — Five Extension Points (Doc 12 §Extensibility Points)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0 (already exists)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Five extensibility points, each with a clear trait or registration mechanism:

| Extension point | Trait/Interface | Location |
|---|---|---|
| New agent backend | `Agent` | `roko-agent/src/agent.rs` |
| New provider adapter | `ProviderAdapter` | `roko-agent/src/provider/` |
| New tool translator | `Translator` | `roko-agent/src/translate/` |
| New LLM backend | `LlmBackend` | `roko-agent/src/tool_loop/` |
| New tool handler | `ToolHandler` | `roko-core/src/tool/` |

### What exists
All five extension points exist as real, implemented traits:

1. **Agent trait** — `crates/roko-agent/src/agent.rs` defines `pub trait Agent: Send + Sync` with `run()`, `name()`, `supports_streaming()`. Seven concrete implementations ship: `ClaudeCliAgent`, `ClaudeAgent`, `OpenAiAgent`, `OllamaAgent`, `ExecAgent`, `MockAgent`, `CursorAgent`, plus `GeminiNativeAgent`, `GeminiCompatAgent`, `PerplexityChatAgent`, `PerplexityDeepResearchAgent`.

2. **ProviderAdapter trait** — `crates/roko-agent/src/provider/mod.rs` defines the trait. Concrete adapters exist in `provider/openai_compat.rs`, `provider/claude_cli.rs`, `provider/anthropic_api.rs`, `provider/cursor_acp.rs`, plus `gemini/adapter.rs` and `perplexity/adapter.rs`. The `adapter_for_kind()` dispatch function is implemented.

3. **Translator trait** — `crates/roko-agent/src/translate/mod.rs` with implementations in `translate/openai.rs`, `translate/claude.rs`, `translate/ollama.rs`, `translate/gemini.rs`, `translate/react.rs`, `translate/capability.rs`.

4. **LlmBackend trait** — `crates/roko-agent/src/tool_loop/mod.rs`. Concrete backends: `OllamaLlmBackend` (`ollama_backend.rs`), `OpenAiCompatLlmBackend` (`openai_compat_backend.rs`), plus `tool_loop/backends/gemini_native.rs`, `tool_loop/backends/openai_compat.rs`, `tool_loop/backends/hedged.rs`.

5. **ToolHandler** — `crates/roko-core/src/tool/handler.rs` with the trait definition. Related files in `tool/mod.rs`, `tool/def.rs`, `tool/call.rs`, `tool/trace.rs`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub trait Agent' crates/roko-agent/src/agent.rs
grep -n 'pub trait ProviderAdapter' crates/roko-agent/src/provider/mod.rs
grep -rn 'pub trait.*Translator' crates/roko-agent/src/translate/
grep -rn 'pub trait LlmBackend' crates/roko-agent/src/tool_loop/
grep -n 'ToolHandler' crates/roko-core/src/tool/handler.rs
```

---

## F.02 — ProviderKind Enum and Provider Config (Doc 12 §Adding a New Protocol Family)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A `ProviderKind` enum with variants `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`, `CursorAcp` in `crates/roko-core/src/agent.rs`. Adding a new protocol means adding a variant, implementing `ProviderAdapter`, and registering in `adapter_for_kind`.

### What exists
`crates/roko-core/src/agent.rs:35` defines `pub enum ProviderKind` with six variants: `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`, `CursorAcp`, `PerplexityApi`, `GeminiApi`. The enum exceeds the doc spec (6 variants vs 4 documented). The `adapter_for_kind()` function at `crates/roko-agent/src/provider/mod.rs` performs exhaustive dispatch. Config-driven provider resolution from `roko.toml` is fully wired via `crates/roko-core/src/config/schema.rs:983` (`pub kind: ProviderKind` on the provider config struct).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None (doc is behind actual impl: missing PerplexityApi/GeminiApi mentions) | — | — |

### Verify
```bash
grep -n 'pub enum ProviderKind' crates/roko-core/src/agent.rs
grep -n 'pub fn adapter_for_kind' crates/roko-agent/src/provider/mod.rs
```

---

## F.03 — 8-Step Domain Plugin Process (Doc 12 §8-Step Domain Plugin Process)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: ~200 (documentation / automation)
- **Dependencies**: None
- **Files to modify**: No code changes needed; process is manual

### What the doc says
An 8-step process for adding a domain-specific agent type:
1. Add `AgentRole` variant
2. Create role template in `roko-compose/src/templates/`
3. Register domain-specific tools
4. Configure model
5. Wire provider
6. Set gate criteria
7. Add to CascadeRouter
8. Test end-to-end

### What exists
The infrastructure for all 8 steps exists:

1. `AgentRole` enum at `crates/roko-core/src/agent.rs` (confirmed via grep — multiple variants including `Implementer`, `QuickReviewer`, `Auditor`, `Refactorer`, `Strategist`, `Researcher`, `Conductor`, `Architect`, `Critic`).
2. Role templates exist at `crates/roko-compose/src/templates/` (grep confirmed `integration.rs` references role-based composition).
3. Tool registration via `crates/roko-core/src/tool/registry.rs`, `tool/role_allowlist.rs`.
4. Model config via `[models.*]` in `roko.toml` and `crates/roko-core/src/config/schema.rs`.
5. Provider wiring via `[providers.*]` config + `adapter_for_kind()`.
6. Gate pipeline at `crates/roko-gate/`.
7. CascadeRouter at `crates/roko-learn/src/cascade_router.rs`.
8. CLI end-to-end via `roko run "<prompt>"`.

The gap is that the 8-step process is not automated or scaffolded. Each step must be done manually with no `roko plugin add` or similar generator command. No wizard or template generator exists.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.03a | No automated scaffold/generator for the 8-step process | `crates/roko-cli/` | Low |
| F.03b | Steps are not validated by CI — easy to skip step 7 (router registration) | `crates/roko-cli/` | Low |

### Verify
```bash
grep -rn 'pub enum AgentRole' crates/roko-core/src/agent.rs
ls crates/roko-compose/src/templates/
grep -rn 'CascadeRouter' crates/roko-learn/src/cascade_router.rs | head -5
```

---

## F.04 — EventSource Trait (Doc 12 §Event System)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
An `EventSource` trait: `fn events(&self) -> Vec<DomainEvent>`. Agents can emit domain-specific events that the learning subsystem captures.

### What exists
A full `EventSource` trait in `crates/roko-plugin/src/lib.rs:121`:
```rust
#[async_trait]
pub trait EventSource: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn kind(&self) -> EventSourceKind;
    async fn start(&self, sender: SignalSender, cancel: CancellationToken) -> Result<()>;
}
```
The implementation exceeds the doc spec: it is async, long-lived, and publishes `Engram` signals rather than returning a `Vec<DomainEvent>`. Two concrete implementations ship:
- `CronEventSource` (`crates/roko-plugin/src/lib.rs:198`) — schedule-driven
- `FileWatchEventSource` (`crates/roko-plugin/src/lib.rs:67`) — filesystem-driven

The CLI integration is at `crates/roko-cli/src/event_sources.rs` which provides `roko event-sources list` to inspect configured sources. Both sources have extensive tests (12+ test functions in `roko-plugin/src/lib.rs:657-1058`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub trait EventSource' crates/roko-plugin/src/lib.rs
grep -n 'impl EventSource' crates/roko-plugin/src/lib.rs
```

---

## F.05 — FeedbackCollector Trait (Doc 12 §Event System)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A `FeedbackCollector` trait: `fn collect(&self, result: &AgentResult) -> Vec<FeedbackSignal>`. Feedback signals are persisted alongside episodes and used by adaptive gate thresholds.

### What exists
A full `FeedbackCollector` trait at `crates/roko-plugin/src/lib.rs:586`:
```rust
#[async_trait]
pub trait FeedbackCollector: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn services(&self) -> Vec<String>;
    fn interval(&self) -> std::time::Duration;
    async fn collect(&self, since: DateTime<Utc>) -> Result<Vec<FeedbackSignal>>;
}
```
The signature differs from the doc: it takes a `since: DateTime<Utc>` timestamp rather than an `&AgentResult`, polling external systems periodically rather than being called per-result. The `FeedbackSignal` struct at line 39 includes `original_episode_id`, `service`, `outcome` (`FeedbackOutcome` enum), `metadata`, and `timestamp`.

A `PluginManifest` struct (`lib.rs:601`) bundles event sources and feedback collectors together, with a `PluginBuilder` fluent API at line 609.

The learning runtime integrates this via `crates/roko-learn/src/runtime_feedback.rs` which uses the `LearningRuntime` to update all learning subsystems from completed episodes.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.05a | No concrete FeedbackCollector implementations ship (only test DummyFeedbackCollector) | `crates/roko-plugin/` | Medium |
| F.05b | No GitHub/Slack/CI collector exists yet | `crates/roko-plugin/` | Medium |

### Verify
```bash
grep -n 'pub trait FeedbackCollector' crates/roko-plugin/src/lib.rs
grep -n 'pub struct FeedbackSignal' crates/roko-plugin/src/lib.rs
grep -n 'impl FeedbackCollector' crates/roko-plugin/src/lib.rs
```

---

## F.06 — CompositeAgent and Coordination Patterns (Doc 00 §Agent Composition)

- **Status**: DONE
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A `CompositeAgent` that merges multiple agent capabilities. Four composition patterns:
1. **Pipeline** — sequential, output feeds into next input
2. **Parallel** — concurrent, outputs merged via `MergeStrategy`
3. **Conditional** — route to branch based on signal properties
4. **MixtureOfAgents** — layered MoA, fan-out then aggregate

`MergeStrategy` enum: `Concatenate`, `Aggregate`, `MajorityVote`, `BestOfN`.

### What exists
Full implementation at `crates/roko-agent/src/composition.rs` (501 lines).

`AgentComposition` enum at line 142 with all four variants:
```rust
pub enum AgentComposition {
    Pipeline(Vec<Box<dyn Agent>>),
    Parallel(Vec<Box<dyn Agent>>, MergeStrategy),
    Conditional { condition, branches },
    MixtureOfAgents { agents, aggregator },
}
```

`MergeStrategy` enum at line 19: `Concatenate`, `Aggregate`, `Vote`, `BestOfN` — all four implemented.

`CompositeAgent` struct at line 214 wraps a named `AgentComposition` and implements `Agent` at line 451. All four composition methods are fully implemented:
- `run_pipeline()` at line 310 — sequential with short-circuit on failure
- `run_parallel()` at line 355 — uses `futures::join_all`
- `run_conditional()` at line 391 — deserializes `Task` from engram body, uses `SkillSelector`
- `run_mixture()` at line 412 — fan-out via parallel, then aggregator agent

`SkillSelector` at line 44 routes by `TaskCategory`, `TaskComplexityBand`, `TaskReasoningLevel`, `TaskSpeedPriority`, `TaskQualityProfile` — richer than the doc's HDC-based proposal.

Three tests pass: `pipeline_feeds_output_forward`, `parallel_concatenates_results`, `conditional_uses_task_selector`.

Re-exported from `crates/roko-agent/src/lib.rs:65`:
```rust
pub use composition::{AgentComposition, CompositeAgent, MergeStrategy, SkillSelector};
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.06a | `CompositeAgent` is not used from `orchestrate.rs` or any CLI path — built but not wired | `crates/roko-cli/src/orchestrate.rs` | Medium |
| F.06b | Doc mentions `SkillSelector` with HDC embeddings and transition graph; actual impl uses task metadata fields — simpler but functional | `crates/roko-agent/src/composition.rs:44` | Low |
| F.06c | No test for MixtureOfAgents composition | `crates/roko-agent/src/composition.rs` | Low |

### Verify
```bash
grep -n 'pub enum AgentComposition' crates/roko-agent/src/composition.rs
grep -n 'pub struct CompositeAgent' crates/roko-agent/src/composition.rs
grep -n 'impl Agent for CompositeAgent' crates/roko-agent/src/composition.rs
cargo test -p roko-agent composition 2>&1 | tail -5
```

---

## F.07 — MetacognitiveMonitor (Doc 00 §Metacognitive Monitoring)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A `MetacognitiveMonitor` that watches agents for failure patterns (stalling, repetition, confidence collapse) and triggers `Intervention` (EscalateModel, HumanHandoff, Abort, InjectReflection). Config includes `max_stalled_turns`, `max_idle_ms`, `repetition_threshold`, `confidence_floor`.

### What exists
Full implementation at `crates/roko-agent/src/introspection.rs` (297 lines).

`MetacognitiveMonitor` struct at line 90 with configurable thresholds:
```rust
pub struct MetacognitiveMonitor {
    pub repeat_threshold: usize,        // default: 3
    pub contradiction_window: usize,    // default: 4
    pub confidence_threshold: f32,      // default: 0.35
    pub human_handoff_threshold: f32,   // default: 0.15
}
```

`Intervention` enum at line 77: `EscalateModel`, `HumanHandoff`, `Abort`, `InjectReflection(String)`.

`Turn` struct at line 37 captures per-turn data (index, assistant_text, reasoning, tool_calls, confidence).

The `check()` method at line 115 implements three detection strategies:
1. **Repeated tool calls** — `repeated_tool_calls()` at line 146 checks fingerprint equality
2. **Contradiction detection** — `contradiction_detected()` at line 160 scans for positive/negative commitment patterns
3. **Confidence-based escalation** — lines 134-141 compare confidence against thresholds

**Wired into the tool loop** at `crates/roko-agent/src/tool_loop/mod.rs:168` — `monitor: Option<MetacognitiveMonitor>` field, with `with_monitor()` builder at line 229. At line 425 of the tool loop run, the monitor is called after each turn:
```rust
if let Some(monitor) = self.monitor.as_ref() {
    let turn = Turn::from_response(iterations, &response, current_calls);
    turn_history.push(turn);
    if let Some(intervention) = monitor.check(&turn_history) {
        match intervention {
            Intervention::InjectReflection(message) => { /* inject system message */ }
            Intervention::EscalateModel | Intervention::HumanHandoff | Intervention::Abort => {
                /* checkpoint and exit */
            }
        }
    }
}
```

Re-exported from `crates/roko-agent/src/lib.rs:71`:
```rust
pub use introspection::{AgentIdentity, Intervention, MetacognitiveMonitor, Turn};
```

Three tests pass: `repeated_tool_calls_trigger_reflection`, `confidence_can_escalate`, `agent_identity_uses_role_defaults`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.07a | Doc mentions `max_stalled_turns` and `max_idle_ms`; impl uses `repeat_threshold` and `contradiction_window` instead — different detection philosophy | `crates/roko-agent/src/introspection.rs` | Info |
| F.07b | `orchestrate.rs` does not attach MetacognitiveMonitor to the ToolLoop when dispatching agents — wired in the ToolLoop API but not called from the orchestrator | `crates/roko-cli/src/orchestrate.rs` | Medium |

### Verify
```bash
grep -n 'pub struct MetacognitiveMonitor' crates/roko-agent/src/introspection.rs
grep -n 'monitor.check' crates/roko-agent/src/tool_loop/mod.rs
cargo test -p roko-agent introspection 2>&1 | tail -5
```

---

## F.08 — AgentIntrospection (Doc 00 §Engineering Introspection)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: ~150
- **Dependencies**: F.07
- **Files to modify**: `crates/roko-agent/src/introspection.rs`

### What the doc says
An `AgentIntrospection` struct providing five capabilities:
1. **State inspection** — query own memory, tool history, current task context
2. **Capability assessment** — report available tools
3. **Confidence estimation** — estimate uncertainty
4. **History review** — review past actions
5. **Failure detection** — detect loops and budget breaches

Supporting structs: `AgentIdentity` (role, model_tier, temperament, capabilities), `EpisodeSummary` (with reflection field), `ResourceUsage` (tokens_used, tokens_remaining, cost_usd, budget_remaining_usd, elapsed_ms, timeout_ms).

### What exists
`AgentIdentity` is implemented at `crates/roko-agent/src/introspection.rs:11`:
```rust
pub struct AgentIdentity {
    pub role: AgentRole,
    pub model_tier: roko_core::ModelTier,
    pub temperament: String,
    pub capabilities: ToolPermissions,
}
```

`Turn` struct captures per-turn data. `MetacognitiveMonitor` handles failure detection (capability 5).

However, the higher-level `AgentIntrospection` struct described in the doc does not exist. No `EpisodeSummary`, `ResourceUsage`, or combined introspection context is injected into agent runs.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.08a | No `AgentIntrospection` composite struct | `crates/roko-agent/src/introspection.rs` | Medium |
| F.08b | No `EpisodeSummary` struct with reflection field | `crates/roko-agent/src/introspection.rs` | Medium |
| F.08c | No `ResourceUsage` budget-tracking struct injected into agent context | `crates/roko-agent/src/introspection.rs` | Medium |
| F.08d | No mechanism to inject introspection data into the agent's system prompt or context | `crates/roko-agent/src/tool_loop/mod.rs` | Medium |

### Verify
```bash
grep -n 'AgentIntrospection\|EpisodeSummary\|ResourceUsage' crates/roko-agent/src/introspection.rs
# Expected: no matches (these structs do not exist)
```

---

## F.09 — Supervision Strategies (Doc 00 §Erlang/OTP Supervision Trees)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A `SupervisionStrategy` enum with three Erlang-style restart strategies:
- `OneForOne { max_restarts, within_ms, fallback_tier }`
- `OneForAll { max_restarts }`
- `RestForOne { max_restarts }`

### What exists
Full implementation at `crates/roko-runtime/src/process.rs:76`:
```rust
pub enum SupervisionStrategy {
    OneForOne { max_restarts: u32, within_ms: u64, fallback_tier: String },
    OneForAll { max_restarts: u32 },
    RestForOne { max_restarts: u32 },
}
```

Serde support with `tag = "kind", rename_all = "snake_case"`. Default is `OneForOne` with `max_restarts: 0`.

The `ProcessSupervisor` at line 280 stores the strategy and implements:
- `restart_process()` at line 498 — restart a single process respecting budget
- `restart_wave()` at line 533 — restart failed + peers per strategy
- `recovery_targets()` at line 544 — selects targets based on strategy variant:
  - `OneForOne` — only the failed process
  - `OneForAll` — all managed processes
  - `RestForOne` — failed process and those started after it (by PID order)
- `allow_restart()` at line 559 — sliding-window rate limiting

Restart history tracking via `restart_history: Mutex<HashMap<String, Vec<Instant>>>`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.09a | `ProcessSupervisor.restart_wave()` is not called from `orchestrate.rs` — the orchestrator uses its own retry logic, not the supervision strategies | `crates/roko-cli/src/orchestrate.rs` | Medium |
| F.09b | `fallback_tier` on `OneForOne` is a `String` not `Option<ModelTier>` as the doc suggests | `crates/roko-runtime/src/process.rs:84` | Low |

### Verify
```bash
grep -n 'pub enum SupervisionStrategy' crates/roko-runtime/src/process.rs
grep -n 'restart_wave\|restart_process' crates/roko-runtime/src/process.rs
cargo test -p roko-runtime process 2>&1 | tail -5
```

---

## F.10 — OCaps Warrant System (Doc 00 §Capability-Based Security)

- **Status**: DONE
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
An `AgentWarrant` carrying cryptographic capability tokens. Five `Capability` variants: `Tool`, `ReadPath`, `WritePath`, `Exec`, `Network`. Warrants support delegation (each hop reduces authority), expiry, and constraints (`WarrantConstraint` enum with `Subpath`, `Ttl`, `MaxInvocations`, `Cel`).

### What exists
Full implementation at `crates/roko-agent/src/safety/capabilities.rs` (189 lines).

`Capability` enum at line 12 with five variants:
```rust
pub enum Capability {
    Tool(String),
    ReadPath(PathBuf),
    WritePath(PathBuf),
    Exec(String),
    Network { host: String, port: u16 },
}
```

`AgentWarrant` at line 27:
```rust
pub struct AgentWarrant {
    pub id: [u8; 32],            // random 256-bit token
    pub capabilities: Vec<Capability>,
    pub issuer: String,
    pub expires_at: Option<u64>,
    pub delegate_depth: u8,
}
```

Core functions:
- `check_capability()` at line 78 — verifies warrant covers a required capability
- `delegate()` at line 86 — creates a child warrant with reduced scope and depth
- `capability_covers()` at line 110 — per-variant matching (path containment for Read/WritePath)
- `network_capability_from_url()` at line 132 — parses URLs to Network capabilities
- `exec_capability_from_command()` at line 146 — extracts first token for Exec capabilities

Wired into the safety layer at `crates/roko-agent/src/safety/mod.rs:51`:
```rust
pub use capabilities::{AgentWarrant, Capability, CapabilityError, check_capability, delegate};
```

The `SafetyLayer` at `safety/mod.rs:94` carries `pub warrant: Option<AgentWarrant>` with `with_warrant()` builder at line 127.

Re-exported from `crates/roko-agent/src/lib.rs:87-89`.

Tests at `capabilities.rs:154-188`: `check_capability_matches_exact_tool`, `delegate_reduces_scope`, `network_capability_parses_host_and_port`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.10a | Doc mentions `WarrantConstraint` enum (Subpath, Ttl, MaxInvocations, Cel) — not implemented, constraints are implicit via capability matching | `crates/roko-agent/src/safety/capabilities.rs` | Medium |
| F.10b | No cryptographic chain (doc mentions `Vec<DelegationHop>` with signed hops) — current delegation is trust-based, not crypto-verified | `crates/roko-agent/src/safety/capabilities.rs` | Low |
| F.10c | Warrants are not checked in the orchestrator's agent dispatch — `orchestrate.rs` does not use warrants | `crates/roko-cli/src/orchestrate.rs` | Medium |
| F.10d | No expiry enforcement — `expires_at` is stored but never checked | `crates/roko-agent/src/safety/capabilities.rs` | Low |

### Verify
```bash
grep -n 'pub struct AgentWarrant' crates/roko-agent/src/safety/capabilities.rs
grep -n 'pub fn check_capability' crates/roko-agent/src/safety/capabilities.rs
grep -n 'pub fn delegate' crates/roko-agent/src/safety/capabilities.rs
cargo test -p roko-agent capabilities 2>&1 | tail -5
```

---

## F.11 — MorphableAgent (Doc 00 §Agent Metamorphosis)

- **Status**: DONE
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: F.08, F.10
- **Files to modify**: None

### What the doc says
A `MorphableAgent` wrapping an inner `Agent`, with a `RoleProfile` (clarity, differentiation, alignment scores). Methods:
- `should_morph()` — evaluate whether role change is warranted
- `morph()` — swap role, system prompt, tool permissions, model tier

Safety constraint: morphing should only expand capabilities through OCaps warrant chain.

### What exists
Full implementation at `crates/roko-agent/src/metamorphosis.rs` (238 lines).

`RoleProfile` at line 13:
```rust
pub struct RoleProfile {
    pub role: AgentRole,
    pub clarity: f32,
    pub differentiation: f32,
    pub alignment: f32,
}
```

`MorphableAgent` at line 46:
```rust
pub struct MorphableAgent {
    inner: Box<dyn Agent>,
    identity: AgentIdentity,
    profile: RoleProfile,
    allowed_transitions: HashMap<AgentRole, Vec<AgentRole>>,
    system_prompt: String,
    name: String,
}
```

The `morph()` method at line 117 validates against `allowed_transitions`, then updates identity, profile, system prompt, and name. Returns `MorphError::TransitionDenied` for disallowed transitions.

A `default_transition_matrix()` at line 195 defines sensible transitions:
- `Implementer` -> `[QuickReviewer, Auditor, Refactorer]`
- `Auditor` -> `[Implementer, Critic]`
- `Strategist` -> `[Implementer, Architect, Researcher]`
- etc.

Implements `Agent` at line 162 — augments input with role-based system prompt, tags output with role and temperament.

Re-exported from `crates/roko-agent/src/lib.rs:72`:
```rust
pub use metamorphosis::{MorphError, MorphableAgent, RoleProfile};
```

Two tests: `morphable_agent_applies_role_tag`, `morph_rejects_disallowed_transition`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.11a | Doc's `should_morph()` (automatic detection) is not implemented — morphing is caller-initiated only | `crates/roko-agent/src/metamorphosis.rs` | Medium |
| F.11b | No OCaps integration — morphing does not check warrant chains, only transition allowlist | `crates/roko-agent/src/metamorphosis.rs` | Low |
| F.11c | Not wired into orchestrate.rs — `MorphableAgent` is never instantiated from any CLI path | `crates/roko-cli/src/orchestrate.rs` | Medium |
| F.11d | System prompt generated by `system_prompt_for()` at line 185 is a bare format string, not using `SystemPromptBuilder` | `crates/roko-agent/src/metamorphosis.rs:185` | Low |

### Verify
```bash
grep -n 'pub struct MorphableAgent' crates/roko-agent/src/metamorphosis.rs
grep -n 'pub fn morph' crates/roko-agent/src/metamorphosis.rs
grep -n 'MorphableAgent' crates/roko-cli/src/orchestrate.rs
# Expected: last grep returns no matches (not wired)
cargo test -p roko-agent metamorphosis 2>&1 | tail -5
```

---

## F.12 — Self-Evolving Architecture: AgentArchive / Darwin Godel Machine (Doc 12 §Self-Evolving + Doc 00 §Self-Evolving)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~500
- **Dependencies**: F.06, F.07, F.08
- **Files to modify**: New module (likely `crates/roko-learn/src/agent_archive.rs`)

### What the doc says
An `AgentArchive` maintaining a population of agent configurations with:
- `select()` — tournament selection for a task type
- `mutate()` — create configuration variant
- `update()` — compute fitness, add/evict entries

Supporting structs: `ArchiveEntry` (config, fitness, specializations, generation, parents), `AgentConfiguration` (role, model_key, system_prompt_overrides, tool_allowlist, temperament, reasoning_strategy, max_iterations).

### What exists
No `AgentArchive`, `ArchiveEntry`, or `AgentConfiguration` structs exist anywhere in the codebase. The `crates/roko-fs/src/archive.rs` file is unrelated (filesystem archival, not evolutionary archives).

The closest existing infrastructure:
- `CascadeRouter` in `crates/roko-learn/` does model routing with bandits — provides a simpler version of model selection
- `ExperimentStore` in `crates/roko-learn/` runs A/B prompt experiments — provides a simpler version of mutation testing
- `EpisodeLogger` captures execution traces — could feed fitness computation

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.12a | `AgentArchive` struct not implemented | `crates/roko-learn/` | Phase 2+ |
| F.12b | `ArchiveEntry` / `AgentConfiguration` not implemented | `crates/roko-learn/` | Phase 2+ |
| F.12c | Tournament selection not implemented | `crates/roko-learn/` | Phase 2+ |
| F.12d | Mutation operators not implemented | `crates/roko-learn/` | Phase 2+ |
| F.12e | Fitness computation not implemented | `crates/roko-learn/` | Phase 2+ |

### Verify
```bash
grep -rn 'AgentArchive\|ArchiveEntry\|AgentConfiguration' crates/ --include='*.rs' | grep -v target/
# Expected: no matches
```

---

## F.13 — Voyager-Style Skill Library (Doc 12 §Voyager-Style Skill Library + Doc 00 §Self-Evolving)

- **Status**: DONE
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
A skill library capturing reusable patterns from successful episodes with:
- Skill extraction from execution traces
- Semantic descriptions for retrieval
- Composable skills that compound agent abilities

### What exists
Full implementation at `crates/roko-learn/src/skill_library.rs` (file exceeds 25K tokens — large, complete module).

`Skill` struct at line 63 with fields:
```rust
pub struct Skill {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub prompt_template: String,
    pub precondition: String,
    pub procedure: String,
    pub postcondition: String,
    pub confidence: f64,
    pub source_episodes: Vec<String>,
    pub validations: u64,
    pub failures: u64,
    pub task_categories: Vec<String>,
    pub created_at: String,
    ...
}
```

`SkillLibrary` is a JSON-file-backed registry with `parking_lot::RwLock` for in-memory access and `tokio::fs` for persistence (tempfile+rename for consistency).

Key methods (from doc string at line 17-28):
- `register()` — add a skill
- `record_use()` — update usage count and success rate

A `TemplatePatternGenerator` exists for extracting skill patterns from episodes.

Wired into the learning runtime at `crates/roko-learn/src/runtime_feedback.rs:42`:
```rust
use crate::skill_library::{SkillLibrary, SkillLibraryError, TemplatePatternGenerator};
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.13a | No HDC-based semantic retrieval — skill selection uses name/category matching, not embeddings | `crates/roko-learn/src/skill_library.rs` | Low |
| F.13b | No automatic skill extraction trigger from the orchestrator after successful episodes | `crates/roko-cli/src/orchestrate.rs` | Medium |

### Verify
```bash
grep -n 'pub struct Skill ' crates/roko-learn/src/skill_library.rs
grep -n 'pub struct SkillLibrary' crates/roko-learn/src/skill_library.rs
grep -n 'SkillLibrary' crates/roko-learn/src/runtime_feedback.rs
```

---

## F.14 — Shared Agent Memory (Doc 00 §Agent Memory Sharing)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~300
- **Dependencies**: F.12, F.13
- **Files to modify**: New module (likely `crates/roko-learn/src/shared_memory.rs`)

### What the doc says
A `SharedAgentMemory` struct with:
- `strategies: HashMap<String, Vec<LearnedStrategy>>` — successful strategies indexed by task type
- `tool_patterns: ToolTransitionGraph` — what tools work and fail together
- `routing_preferences: HashMap<String, ModelPreference>` — model routing from team experience

`LearnedStrategy` with description, approach (compressed prompt fragment), applicable_to, confidence (EMA), discovered_by, success_count.

### What exists
No `SharedAgentMemory`, `LearnedStrategy`, or `ToolTransitionGraph` structs exist in the codebase (grep confirmed: zero matches for `ToolTransitionGraph`).

Related existing infrastructure:
- `SkillLibrary` in `roko-learn` captures some of the "reusable strategy" concept
- `CascadeRouter` handles model routing preferences
- `PlaybookStore` / `PlaybookRules` in `roko-learn` capture execution patterns
- `ToolTransitionGraph` is referenced in system prompt builder at `crates/roko-compose/src/system_prompt_builder.rs` and role prompts at `crates/roko-compose/src/role_prompts.rs` as a concept for prompt enrichment, plus the dashboard at `crates/roko-cli/src/tui/dashboard.rs` and orchestrate at `crates/roko-cli/src/orchestrate.rs` — but these are string references in prompts, not an actual graph data structure.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| F.14a | `SharedAgentMemory` not implemented | `crates/roko-learn/` | Phase 2+ |
| F.14b | `LearnedStrategy` not implemented | `crates/roko-learn/` | Phase 2+ |
| F.14c | `ToolTransitionGraph` as a data structure not implemented (referenced only as string concepts in prompts) | `crates/roko-learn/` | Phase 2+ |

### Verify
```bash
grep -rn 'SharedAgentMemory\|LearnedStrategy\|pub struct ToolTransitionGraph' crates/ --include='*.rs' | grep -v target/
# Expected: no matches (or only string references, not struct definitions)
```

---

## F.15 — NL-to-Format Two-Pass Pipeline (Doc 12 implied)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The doc does not explicitly cover NL-to-Format, but it is referenced as an extension point for structured output from agents.

### What exists
Full implementation at `crates/roko-agent/src/nl_to_format/mod.rs` (402 lines).

`NlToFormatConverter` struct at line 70 with:
- `convert(nl_response, target_schema)` at line 132 — two-pass extraction pipeline
- `extraction_prompt(schema)` at line 108 — generates CRANE-delimiter instructions
- `wrap(content)` at line 166 — wraps content in delimiters

Sub-modules:
- `delimiters` at `nl_to_format/delimiters.rs` — CRANE-style `<|tag|>...<|/tag|>` extraction
- `routing` at `nl_to_format/routing.rs` — decides whether to use two-pass vs direct decoding

Features:
- CRANE delimiter extraction (pass 2)
- Fallback JSON detection with depth-tracking bracket matcher
- Schema validation (required fields)
- Custom tag support

14 tests covering edge cases (empty, whitespace, nested JSON, custom tags, etc.).

Referenced by `SkillLibrary` at `crates/roko-learn/src/skill_library.rs:37`:
```rust
use roko_agent::{Agent, nl_to_format::NlToFormatConverter};
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub struct NlToFormatConverter' crates/roko-agent/src/nl_to_format/mod.rs
cargo test -p roko-agent nl_to_format 2>&1 | tail -5
```

---

## F.16 — Pointer GC (Doc implied from codebase)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The doc does not explicitly describe pointer GC, but the codebase has it as an extension point for memory management during tool loops.

### What exists
Full implementation at `crates/roko-agent/src/pointer/gc.rs` (280+ lines).

`PointerGcPolicy` at line 46:
```rust
pub struct PointerGcPolicy {
    pub max_age_turns: u32,     // default: 10
    pub max_total_bytes: u64,   // default: 10 MiB
}
```

`PointerMeta` at line 26 — lightweight metadata for GC decisions (id, size_bytes, created_at_turn, last_accessed_turn).

Key methods on `PointerGcPolicy`:
- `should_evict(pointer, current_turn)` at line 87 — age-based check
- `select_evictions(pointers, current_turn)` at line 104 — two-phase: age-based first, then LRU size-based

The GC is passive — called between turns, not on a timer.

Re-exported from `crates/roko-agent/src/pointer/mod.rs:5`:
```rust
pub use gc::{PointerGcPolicy, PointerMeta};
```

11 tests covering age eviction, size LRU, combined, empty input, serde roundtrip, tie-breaking.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub struct PointerGcPolicy' crates/roko-agent/src/pointer/gc.rs
grep -n 'pub fn select_evictions' crates/roko-agent/src/pointer/gc.rs
cargo test -p roko-agent pointer 2>&1 | tail -5
```

---

## Summary

| ID | Item | Status | Priority |
|----|------|--------|----------|
| F.01 | Five Extension Points | DONE | P0 |
| F.02 | ProviderKind + Provider Config | DONE | P0 |
| F.03 | 8-Step Domain Plugin Process | PARTIAL | P2 |
| F.04 | EventSource Trait | DONE | P1 |
| F.05 | FeedbackCollector Trait | DONE | P1 |
| F.06 | CompositeAgent + Coordination | DONE | P2 |
| F.07 | MetacognitiveMonitor | DONE | P1 |
| F.08 | AgentIntrospection | PARTIAL | P2 |
| F.09 | Supervision Strategies | DONE | P1 |
| F.10 | OCaps Warrant System | DONE | P2 |
| F.11 | MorphableAgent | DONE | P2 |
| F.12 | AgentArchive / Darwin Godel Machine | NOT DONE | P3 |
| F.13 | Skill Library (Voyager) | DONE | P2 |
| F.14 | Shared Agent Memory | NOT DONE | P3 |
| F.15 | NL-to-Format Pipeline | DONE | P1 |
| F.16 | Pointer GC | DONE | P1 |

**Totals**: 12 DONE, 2 PARTIAL, 2 NOT DONE

### Key theme: Built but not wired

The dominant pattern in this section is **implementations exist but are not called from the orchestrator**. CompositeAgent, MorphableAgent, MetacognitiveMonitor (partially), OCaps warrants, and SupervisionStrategy restart_wave all have working code with tests, but none are invoked from `orchestrate.rs` or any CLI path. The self-evolving features (AgentArchive, SharedAgentMemory) are Phase 2+ and appropriately not started.

### Highest-impact wiring gaps

1. **F.07b** — Wire `MetacognitiveMonitor` into orchestrate.rs agent dispatch (~20 LOC, immediate safety improvement)
2. **F.06a** — Wire `CompositeAgent` into plan execution for multi-agent coordination (~100 LOC)
3. **F.09a** — Wire `SupervisionStrategy.restart_wave()` into orchestrate.rs failure recovery (~50 LOC)
4. **F.10c** — Wire warrant checking into agent dispatch for capability-based security (~30 LOC)
5. **F.05a** — Implement at least one concrete FeedbackCollector (GitHub PR status) (~200 LOC)

---

## Agent Execution Notes

### What Batch 02 Should Actually Own Here

Most of this file should remain bounded or deferred.

Good batch-`02` candidates:

- `F.07b` if it falls out of ToolLoop universality work,
- maybe a tiny runtime-visible activation of one already-built primitive.

Usually defer:

- `CompositeAgent` runtime orchestration activation,
- supervision-tree recovery behavior,
- concrete feedback collectors,
- Darwin-Godel / shared-memory systems.

### F.07 — Metacognitive Monitor

This is the cleanest advanced feature to activate because the API already exists inside `ToolLoop`.

Acceptance criteria:

- the chosen tool-loop runtime path attaches a monitor,
- at least one intervention path is testable or observable,
- the batch does not widen into an introspection framework buildout.

### F.09 / F.10

Treat runtime supervision and warrant-enforcement activation as follow-on work unless they are a direct side effect of the chosen dispatcher/tool-loop path.
