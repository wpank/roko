# Native Roko Harness: Blue Ocean Design

## Current State: Roko Has a Hybrid Architecture

Roko already has **two dispatch paths**:

### Path A: External Harness (Claude CLI)
- `ClaudeCliAgent` spawns `claude` binary as subprocess
- Claude CLI drives its own internal tool loop
- Roko is a passive observer — reads final output, no tool interception
- **This is the default/primary path** for most orchestration

### Path B: Native Tool Loop (API backends)
- `ToolLoop` in `roko-agent/src/tool_loop/` drives the full cycle
- Works for: Anthropic API, OpenAI/Codex, Gemini, Perplexity, Ollama
- Roko sends prompt → gets tool calls → dispatches via `ToolDispatcher` → sends results → loops
- **This path EXISTS but is secondary** — orchestrate.rs defaults to Claude CLI

### The Problem
Path B is a standard tool loop — identical to what every other harness does. It's
`prompt → LLM → parse_tool_calls → dispatch → results → loop`. Nothing novel.

Path A delegates everything. Roko can't intercept, learn from, or modulate tool execution.

Neither path integrates the cognitive subsystems (daimon, neuro, dreams, gating, somatic).

## The Red Ocean (What Everyone Builds)

Every agent harness in 2025-2026 converges on the same architecture:

```
System prompt + context → LLM API → Parse tool calls → Execute tools → Loop
```

| Feature | Claude Code | Codex CLI | Cursor | Aider | Cline |
|---|:-:|:-:|:-:|:-:|:-:|
| LLM API wrapper | X | X | X | X | X |
| File read/write tools | X | X | X | X | X |
| Shell execution | X | X | X | X | X |
| Git integration | X | X | - | X | X |
| MCP tool protocol | X | X | X | - | X |
| Context management | X | X | X | X | X |
| Permission gates | X | X | X | - | X |
| Multi-file editing | X | X | X | X | X |

**Differentiation is purely UX** (terminal vs IDE vs web) and model defaults.

## Fundamental Limitations Nobody Has Solved

1. **Goldfish Memory** — Every session starts at zero. No cross-session learning.
2. **Compounding Errors** — Mistakes at turn 3 bake into code by turn 30. No mid-session self-correction.
3. **Cost Runaway** — Unconstrained agents cost $5-8/task. No proactive cost gating.
4. **No Learning Curve** — Agent #1000 is no smarter than agent #1.
5. **Brittle Context** — Static assembly. Doesn't adapt based on what worked before.
6. **Sequential Bottleneck** — One agent, one context window, one model.
7. **No Embodied Hesitation** — Agents confidently do destructive things with no "gut feeling."

## The Blue Ocean: Five Killer Features

### 1. Cognitive Gating (80% of ticks cost $0)

**What**: The heartbeat pipeline gates expensive LLM calls behind prediction error.
Most ticks are T0 (deterministic Rust, no LLM, $0). Only novel situations escalate.

**Why unique**: Every other harness sends EVERYTHING to the LLM. Roko's 7-rung
adaptive gate pipeline (already wired) means verification is learned, not static.
After 20 consecutive passes on a rung, it auto-skips.

**Economics**: Without gating: $576/day for continuous operation.
With gating: $6-58/day. **35x cost reduction** from architecture alone.

**Already built**: `roko-gate` (7 rungs, 11 gates), `adaptive_threshold.rs` (EMA + CUSUM),
`cascade_router.rs` (static → confidence → LinUCB bandit). Needs: heartbeat integration.

### 2. Learnable Context Assembly (Prompt N+1 > Prompt N)

**What**: Context is a feedback-loop control system, not a static prompt. The system
tracks which sections correlated with success and adjusts allocation.

**Why unique**: Claude Code has 4-level memory hierarchy but it's static — priority
never changes. Roko has:
- **VCG auction** among context bidders (Neuro/Task/Research) competing for budget
- **Section effect tracking** measuring causal impact on outcomes
- **HDC fingerprinting** for O(1) similarity lookup ("tasks like this one")
- **Playbook injection** from proven patterns at dispatch time

**Already built**: `cfactor.rs`, `section_effect.rs`, `vcg_allocate`, `playbook.rs`,
HDC fingerprinting per episode. Needs: CognitiveWorkspace abstraction + feedback loops.

### 3. Sleep/Dream Consolidation (Offline Pattern Discovery)

**What**: Between sessions, the system replays episodes, discovers patterns,
rehearses threats, generates counterfactuals, and consolidates into playbooks.

**Why unique**: Everyone else's "memory" is append-only log retrieval. Roko's dreams
actively transform experience:
- **Replay** with affect weighting (high-surprise, high-failure episodes)
- **Counterfactual** generation ("what if we used a different approach?")
- **Threat rehearsal** ("what dangerous patterns might we encounter next?")
- **Staging buffer** with confidence graduation before playbook promotion

**Already built**: `roko-dreams` (replay, imagination, rehearsal, staging, promotion).
Needs: runtime trigger from sleep pressure, not just plan completion.

### 4. Somatic Markers / Embodied Hesitation

**What**: An affect system that assigns emotional valence to actions based on history.
When an agent is about to take an action matching past failures, it generates a
hesitation signal — increasing gate requirements, forcing approval, or routing
to a more capable model.

**Why unique**: Current safety is binary (allow/deny). Roko creates a continuous gradient:
- Low-affect: proceed at full speed
- Medium-affect: additional verification rungs
- High-affect: escalate to human review or more capable model

**Already built**: `roko-daimon` (ALMA affect, somatic markers, behavioral state).
Needs: wiring into gate routing + tool dispatch decision path.

### 5. Native Rust Agent Loop with Type-State Lifecycle

**What**: The entire agent loop — inference, tool dispatch, gating, learning — runs
as compiled Rust. No subprocess spawning, no JSON-RPC overhead, no cold starts.
Type-state lifecycle makes invalid states compile errors.

**Why unique**:
- Claude Code: tools shell out to subprocesses
- Codex CLI: Rust binary but tools are Node.js subprocesses
- LangGraph/CrewAI: Python (5x memory, 100-400x cold start vs Rust)

Roko's 19 built-in tools execute as direct function calls. Zero serialization.

**Already built**: `roko-std` (19 tools), `ToolDispatcher`, `ToolLoop`.
Needs: type-state Agent<Phase> wrapper, heartbeat integration.

## The Native Harness Architecture

### What Makes It Different From "Yet Another Tool Loop"

The standard tool loop is:
```
prompt → LLM → parse tools → execute → results → LLM → repeat
```

The roko harness adds **four new stages** that no other harness has:

```
OBSERVE
  → Read environment (files, chain, events)
  → Compute prediction error (how surprising is this?)

GATE
  → Is this novel enough to need LLM? (T0/T1/T2)
  → T0: handle with Rust pattern matching, $0
  → T1: cheap model, minimal context
  → T2: full reasoning, complete workspace

ASSEMBLE (learnable)
  → CognitiveWorkspace: typed, budgeted, audited
  → VCG auction among context bidders
  → Affect-modulated allocation (stressed → more warnings)
  → Cache-aligned prefix (90% hit rate on system prompt)

INFER + TOOL LOOP
  → Standard: prompt → LLM → tools → results → loop
  → But: somatic check on every tool call
  → And: per-turn cost tracking with budget pressure

REFLECT
  → Record DecisionCycleRecord (not just "task succeeded")
  → Attribute: which context sections were referenced?
  → Feedback: update section allocations, routing weights
  → Episode → grimoire → playbook (learning pipeline)

CONSOLIDATE (offline, delta tick)
  → Dream cycle: replay → imagine → rehearse → stage → promote
  → Evolve context policy based on accumulated feedback
  → Prune stale knowledge (Ebbinghaus decay)
```

### The Inference Gateway (From Bardo PRDs)

Rather than each backend being a separate code path, all inference routes through
a unified gateway with three cache layers:

```
Agent Request
  ↓
L3: Deterministic Cache (SHA-256 hash → exact match, 100% savings, ~10% hit)
  ↓
L2: Semantic Cache (embedding similarity > 0.92, 100% savings, ~30% of L3 misses)
  ↓
L1: Prefix Cache (provider-side KV reuse, ~90% input token savings)
  ↓
Provider Routing (Intent-based, first-match-wins)
  ↓
Backend (Anthropic API / OpenAI / Gemini / Ollama / etc.)
```

**Expected aggregate savings: 60-80%** on top of cognitive gating savings.

### Intent-Based Provider Routing

Instead of hardcoded model selection, subsystems declare their needs:

```rust
pub struct Intent {
    pub model: Option<String>,     // specific model or "best available"
    pub require: Vec<String>,      // hard requirements
    pub prefer: Vec<String>,       // soft preferences
    pub quality: Quality,          // Minimum → Maximum
    pub max_latency_ms: u64,
    pub cost_sensitivity: f64,     // 0.0 (don't care) to 1.0 (extremely sensitive)
    pub subsystem: String,         // "heartbeat_t1", "dream", "operator"
}
```

The resolver walks the ordered provider list; first match wins. No scoring algorithm,
no central registry. Predictable, debuggable, owner-controlled.

**Mortality pressure** modifies intents: dying agents become more cost-sensitive,
downgrade quality for non-critical subsystems. This is why mortality is architectural —
it creates economic pressure that makes the system optimize itself.

### The Translator Pattern

Each backend has a `Translator` that handles bidirectional conversion:

```rust
pub trait Translator: Send + Sync {
    fn format(&self) -> ToolFormat;
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;
    fn parse_calls(&self, response: &BackendResponse) -> Result<Vec<ToolCall>>;
    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults;
}
```

Formats: `AnthropicBlocks`, `OpenAiJson`, `GeminiNative`, `ReActText` (for models
without native function calling — embeds tool schemas in system prompt, parses
`Action:`/`Action Input:`/`Observation:` markers).

### What About Claude CLI?

Claude CLI becomes **one backend option**, not the primary path. For users who want
the Claude Code experience, the Claude CLI adapter still works. But the native path
offers:

| | Claude CLI (external) | Native Roko Harness |
|---|---|---|
| Tool interception | No (black box) | Yes (every call) |
| Somatic checks | No | Yes (per tool call) |
| Cost gating | No (Claude CLI manages) | Yes (per inference) |
| Learning from tools | No (only final output) | Yes (per turn) |
| Cache layers | Claude's own | Roko's L1/L2/L3 |
| Model routing | Fixed per dispatch | Intent-based per tick |
| Context assembly | Static prompt | Learnable workspace |
| Cognitive tiers | Always T2 | T0/T1/T2 gated |

## Unique Use Cases Enabled

### For Coding Agents
- **Auto-skip verification** for patterns the system has seen 20+ times pass
- **Hesitation before destructive ops** (rm, force push, drop table) based on past incidents
- **Cross-session playbooks** ("last time we refactored auth, we missed the middleware — check it")
- **Dream-discovered patterns** ("3 out of 5 recent TypeScript tasks failed on ESM imports — inject warning")
- **Cost-aware model routing** ("this is a simple rename — use Haiku, not Opus")

### For Blockchain Agents
- **5-second perception ticks** reading chain state at $0 (T0, no LLM)
- **Somatic hesitation** before large trades matching past loss patterns
- **Sleep pressure** accumulates during volatile markets → forced consolidation
- **Mortality-driven strategy** — agents optimize for survival, not just returns
- **Cross-agent pheromones** — implicit threat/opportunity signals between peers

### For Research Agents
- **Knowledge graph growing across sessions** (not just append-only retrieval)
- **Dream synthesis** finding connections between disconnected knowledge clusters
- **Confidence-gated promotion** — insights must validate across multiple episodes
- **Source-aware citation tracking** with provenance chains
- **Hypothesis generation** from counterfactual dream episodes

### For Third-Party Extension Developers
- **22-hook extension trait** — add behavior without touching core
- **Layer-ordered composition** — extensions fire in dependency order
- **Shared CorticalState** — extensions read/write atomic signals
- **Domain profiles** — declare tick frequency, extensions, gates, tools via config
- **MCP compatibility** — existing MCP tools work through standard hooks

## Why Build From Scratch?

1. **Cannot retrofit learning into a stateless loop.** The universal type (Engram),
   substrate (durable store), and feedback loop (score→route→compose→act→verify→write→react)
   must be designed in, not bolted on.

2. **Cannot retrofit native gating into subprocess-based tools.** If tools shell out
   to processes, gates must wait. If tools are Rust functions, gates inspect results
   in the same stack frame. 4ms vs 60ms per invocation × thousands = hours of difference.

3. **Cannot retrofit type-state safety after the fact.** Lifecycle guarantees must
   be in the type system from day zero. Adding them later means `Result<T, InvalidState>`
   everywhere instead of making invalid states unrepresentable.

4. **Cannot retrofit evolutionary dynamics into append-only logs.** Signal metabolism
   (replicator dynamics, Hebbian learning) treats signals as living populations.
   This is an architectural decision, not a feature flag.

## The Integrated Thesis

```
Session N:
  Task arrives
  → Learnable context assembly (prompt N > prompt N-1)
  → Cascade router (bandit-selected model)
  → Cognitive gate (80% of verification free)
  → Somatic check (hesitation on risky actions)
  → Native Rust tool execution (zero overhead)
  → Episode recording (HDC fingerprint)

Between Sessions ($0):
  → Dream: replay high-surprise episodes
  → Dream: extract playbook patterns
  → Dream: rehearse threat scenarios
  → Dream: update section-effect weights
  → Dream: consolidate into neuro store

Session N+1:
  → Measurably better context, routing, safety
  → For the same task categories
  → The system improves without model changes
```

**The core differentiator is time.** Every other harness is memoryless across sessions.
Roko operates across three timescales:
- **Gamma** (fast): native tools, deterministic gates, sub-ms decisions
- **Theta** (session): bandit routing, adaptive thresholds, episode logging
- **Delta** (offline): dream consolidation, playbook extraction, pattern discovery

Agent session #1000 is categorically better than session #1 — not because
the model improved, but because the harness learned.
