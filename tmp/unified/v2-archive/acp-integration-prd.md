# ACP Integration PRD — Roko as an Agent Client Protocol Server

> **Document**: PRD + Implementation Plan
> **Date**: April 26, 2026
> **Status**: Draft
> **Scope**: Implement ACP (Agent Client Protocol) server mode in Roko so that Roko agents are usable from any ACP-compatible editor — JetBrains IDEs, Zed, Neovim, Emacs, VS Code (community), Obsidian, Toad, and any future ACP client — with zero custom integration work per editor.

---

## 1. Context & Motivation

### 1.1 What ACP Is

The **Agent Client Protocol** is an open standard co-developed by Zed Industries and JetBrains (partnership announced August 27, 2025) that standardizes communication between code editors and AI coding agents. It is the LSP (Language Server Protocol) equivalent for AI agents. LSP made it so any editor could support any language through a shared interface; ACP does the same for coding agents.

The protocol is JSON-RPC 2.0 over stdio. The editor spawns the agent as a subprocess (`roko acp`), and they exchange newline-delimited JSON messages bidirectionally over stdin/stdout. The editor handles all UI — rendering markdown, displaying diffs, showing permission dialogs, managing terminal panels. The agent handles all cognition — model routing, tool execution, planning, verification.

**Protocol version**: v0.12.2 (April 23, 2026), pre-1.0 but core lifecycle is stable.
**Releases**: 41 total.
**License**: Apache 2.0.
**Stars**: 2.9k+ GitHub stars, 226 forks, 1,246 commits.
**Repository**: [agentclientprotocol/agent-client-protocol](https://github.com/agentclientprotocol/agent-client-protocol)
**Specification**: [agentclientprotocol.com](https://agentclientprotocol.com/get-started/introduction)
**Schema**: `schema/schema.json` in the main repository.
**Primary language**: Rust (98.7% of codebase).
**Governance**: Community-governed, documented in `GOVERNANCE.md` and `MAINTAINERS.md`.

Official SDKs:

| Language | Package | Repository |
|---|---|---|
| **Rust** | `agent-client-protocol` | [agentclientprotocol/rust-sdk](https://github.com/agentclientprotocol/rust-sdk) |
| **TypeScript** | `@agentclientprotocol/sdk` | [agentclientprotocol/typescript-sdk](https://github.com/agentclientprotocol) |
| **Python** | `agentclientprotocol` | [agentclientprotocol/python-sdk](https://github.com/agentclientprotocol/python-sdk) |
| **Kotlin** | `acp-kotlin` | [agentclientprotocol/kotlin-sdk](https://github.com/agentclientprotocol/kotlin-sdk) |
| **Java** | `java-sdk` | agentclientprotocol org |
| **Go** | Go SDK | agentclientprotocol org |

Community: Elixir SDK (`acpex` on hex.pm), Vercel AI SDK community provider.

### 1.2 The Protocol Landscape — Five ACPs and Three Exoskeletons

There are five distinct protocols that use the "ACP" acronym. They solve orthogonal problems and compose naturally. Conflating them is common and harmful.

| Acronym | Full Name | Creator | Purpose | Transport | Status |
|---|---|---|---|---|---|
| **ACP** | **Agent Client Protocol** | Zed Industries + JetBrains | Editor ↔ coding agent | JSON-RPC 2.0 over stdio | **Active**, v0.12.2, this document |
| ACP | Agent Communication Protocol | IBM Research / BeeAI | Agent ↔ agent interop | REST (HTTP-native) | **Merged into A2A** (Sept 2025) |
| ACP | Agent Connect Protocol | AGNTCY (Cisco + LangChain) | Remote agent invocation | OpenAPI (REST) | Active, part of AGNTCY framework |
| ACP | Agentic Commerce Protocol | OpenAI + Stripe | AI-mediated purchases | REST / MCP server | Beta |
| ACP | Agent Context Protocols | Independent | Multi-agent coordination | Varies | Early stage |

The three **exoskeleton protocols** (from [doc-12](12-CONNECTIVITY.md)) compose with ACP:

| Protocol | Role | Roko Status | Relationship to ACP |
|---|---|---|---|
| **MCP** | Agent ↔ tools (vertical) | Already wired | ACP reuses MCP JSON representations. ACP passes editor MCP configs to agent. |
| **A2A** | Agent ↔ agent over network | Separate effort | ACP prompts can trigger A2A delegation. Results stream back through ACP. |
| **ERC-8004** | On-chain identity + reputation | Phase 2+ | Agent provenance for ACP registry entries. |

**The consensus stack** (April 2026): MCP for tool access, ACP for editor integration, A2A for cross-agent coordination. These three are complementary and increasingly co-deployed.

### 1.3 What ACP Is Not

ACP is not A2A, not IBM's ACP, not the commerce protocol. It is specifically the **editor ↔ agent** protocol:

- **MCP** — agent ↔ tools (USB-C: device to peripherals)
- **ACP** — agent ↔ editor/IDE (LSP: editor to language server)
- **A2A** — agent ↔ agent over network (Wi-Fi: device to device)

ACP sits between the user and the agent. A2A sits between agents. MCP sits between an agent and external data/tools. All three use JSON-RPC 2.0 but over different transports (stdio vs HTTP vs gRPC).

### 1.4 Why This Matters for Roko

Roko currently has four user interfaces: the CLI (`roko run`), the TUI (ratatui dashboard), the HTTP control plane (`roko serve` on :6677), and the web dashboard (Nunchi). None of these are where most developers spend their time — they're in their editors. ACP puts Roko directly into the editor, right next to the code, without any editor-specific integration work.

The economics are compelling:

- **JetBrains**: 30M+ users across IntelliJ, PyCharm, WebStorm, GoLand, RustRover, CLion. ACP is native since 2025.3. No JetBrains AI subscription required to use ACP agents.
- **Zed**: Fast-growing editor, created the protocol, most complete ACP support. Token-based billing with in-editor counter.
- **VS Code**: 75M+ monthly active users. Community ACP extension ([vscode-acp](https://github.com/formulahendry/vscode-acp)) exists; native support requested via [issue #265496](https://github.com/microsoft/vscode/issues/265496). Even community-only coverage is valuable.
- **Neovim**: Via CodeCompanion, avante.nvim, and agentic.nvim plugins. Large power-user audience.
- **Emacs**: Community ACP plugin ([emacsmirror/acp](https://github.com/emacsmirror/acp)).
- **Obsidian**: Via obsidian-agent-client. Relevant to knowledge management workflow.
- **Toad**: Will McGugan's terminal TUI supporting 18+ agents via ACP.
- **Eclipse**: Listed as compatible in ACP docs.

One implementation, all of these editors. Implement ACP once, reach everywhere.

The ACP Registry (launched January 28, 2026, jointly maintained by JetBrains and Zed) provides one-click installation. Once Roko is in the registry, any JetBrains or Zed user can install it from a dropdown menu — no `pip install`, no manual config, no documentation reading. The registry handles downloading, runtime management, configuration, and auto-updates (hourly cron checks npm, PyPI, and GitHub releases).

### 1.5 The ACP Agent Ecosystem — 33+ Agents

Roko enters a crowded but undifferentiated field:

| Agent | Creator | Notes |
|---|---|---|
| **Claude Agent / Claude Code** | Anthropic | Via Zed SDK adapter |
| **Codex CLI** | OpenAI | Via Zed adapter |
| **Gemini CLI** | Google | Native ACP support |
| **GitHub Copilot** | Microsoft | Public preview |
| **Cursor** | Cursor Inc. | CLI with ACP docs (agent-side only) |
| **Junie** | JetBrains | Native |
| **Goose** | Block (fka Square) | Open source |
| **Kiro CLI** | AWS/Amazon | Native |
| **Kimi CLI** | Moonshot AI | Native |
| **Mistral Vibe** | Mistral AI | Native |
| **Qwen Code** | Alibaba | Native |
| **OpenCode** | SST framework | Open source |
| **OpenHands** | Open source | Community |
| **Cline** | Open source | Dedicated ACP support |
| **Augment Code** | Augment | CLI-based |
| **Hermes Agent** | Nous Research | Open source |
| AgentPool, AutoDev, Blackbox AI, crow-cli, Docker cagent, Factory Droid, fast-agent, fount, Minion Code, OpenClaw, Pi (via adapter), Qoder CLI, Stakpak, stdio Bus, VT Code | Various | Various stages |

Every one of these agents is a single-loop LLM wrapper: prompt → model → tool calls → response. None has a gate pipeline. None has persistent knowledge. None has multi-phase planning with visual step-by-step progress. This is Roko's differentiation.

### 1.6 What Makes Roko Different from Other ACP Agents

Every other agent in the ACP registry is essentially a single-loop LLM wrapper. Roko brings capabilities that no other ACP agent has:

- **Gate pipeline**: Compile, test, clippy, and formal verification gates that objectively verify agent output. When a user sees "Gate: 147/147 tests passed" in their editor, they know the code works — not because the LLM said so, but because the compiler/test harness proved it.
- **Multi-phase execution**: Enriching → Implementing → Gating → Verifying → Reviewing → Merging. ACP's plan support renders this as a visual step-by-step pipeline in the editor.
- **Knowledge store**: Persistent knowledge entries (Engrams/Signals) with confidence scores, demurrage dynamics, and HDC fingerprints (10,240-bit Kanerva vectors). The agent remembers and learns across sessions.
- **Daimon affect state**: PAD vectors (Pleasure/Arousal/Dominance) modulate agent behavior. High arousal from repeated failures triggers strategy shifts visible to the user.
- **Session modes**: Not just "ask" vs "code" — Roko offers plan-first, research, review, and full autonomous modes that map naturally to ACP's session mode system.
- **Conductor watchers**: GhostTurn, ReviewLoop, CompileFailThreshold — autonomous corrective actions that run in the background during execution.
- **Cost-per-decision telemetry**: Real-time cost attribution from the CostLens ([doc-09](09-TELEMETRY.md)), token-level tracking surfaced via ACP's session usage protocol.
- **Vitality & behavioral phases**: Economic pressure scalar drives behavioral modulation (Thriving → Stable → Conservation → Declining → Terminal), visible in the editor as strategy shifts.
- **VCG context assembly**: Budget-constrained attention auction for prompt composition. The agent doesn't just stuff context — it bids for it.

These show up as concrete UX advantages in the editor: plan steps with live status updates, gate results as structured tool call outputs, mode switching between architect and implement, cost tracking in the status area, and slash commands that expose Roko-specific capabilities.

---

## 2. Protocol Mapping — ACP ↔ Roko

### 2.1 Protocol Lifecycle Mapped to Roko Concepts

```
ACP Protocol                          Roko Internals
──────────────────                    ──────────────────────────────────
initialize                            Capability negotiation
  clientCapabilities.fs.readTextFile   → AcpFileSubstrate (replaces local FileSubstrate)
  clientCapabilities.fs.writeTextFile  → AcpFileSubstrate (diffs shown in editor)
  clientCapabilities.terminal          → AcpTerminal (replaces ProcessSupervisor for shell)
  agentCapabilities.loadSession        → KnowledgeStore + DaimonState persistence
  agentCapabilities.promptCapabilities → Image support depends on backend model
  agentCapabilities.configOptions      → Model tier, mode, thinking, gates, knowledge

session/new                            Episode::new() + KnowledgeStore::open()
  → returns sessionId                  → maps to episode_id
  → returns configOptions              → Model tier, mode, toggle settings (§2.2)
  → triggers available_commands_update  → Roko slash commands
  → triggers config_option_update      → Initial config state for editor UI

session/prompt                         CognitiveLoop::run_once()
  prompt[].text                        → user_prompt in PromptComposer
  prompt[].resource                    → file content injected into context

session/update notifications           Bus Pulse events mapped to ACP updates:
  agent_message_chunk                  ← TokenChunk from LLM streaming
  agent_thought_chunk                  ← ThinkingChunk from extended thinking
  tool_call                            ← ToolCallStart (read file, write file, run cmd)
  tool_call_update                     ← ToolCallProgress / ToolCallComplete
  plan                                 ← PlanPhase transitions in PlanState
  available_commands_update            ← Dynamic command updates based on context
  config_option_update                 ← Settings changes (mode, model, dependent options)
  usage_update                         ← CostLens token/cost accumulator (unstable RFD)
  session_info_update                  ← Title/metadata changes
  current_mode_update                  ← Agent-initiated mode switches (legacy, §2.2 note)

session/config/update (client→agent)   Config option changed by user in editor UI:
  "model_tier" → "t3"                 → CascadeRouter::force_tier(T3)
  "agent_mode" → "plan"               → RoleSystemPromptSpec::switch_to(Strategist)
  "thinking" → "verbose"              → ThinkingConfig::set_level(Verbose)
  "gate_pipeline" → false             → GatePipeline::disable()

elicitation/create (agent→client)      Structured form input (unstable, §2.9)
  → editor shows form dialog           → Project-specific gate config, budget limits, etc.
  → user fills and submits             → Values applied to session state

session/request_permission             SafetyLayer::check_pre_execution()
  → editor shows approval dialog       → maps to GateKeeper policy
  → user approves/rejects             → CancelToken or proceed

fs/read_text_file (agent → editor)     Replaces roko_fs::FileSubstrate::read()
fs/write_text_file (agent → editor)    Replaces roko_fs::FileSubstrate::write()
                                        Editor shows inline diff with accept/reject

terminal/create (agent → editor)       Replaces bardo_runtime::ProcessSupervisor
terminal/output                        Editor's terminal panel shows output
terminal/wait_for_exit                 Gate pipeline waits for compile/test results
terminal/kill                          CancelToken propagation

session/cancel                         CancelToken::cancel() propagated to all subsystems

session/load                           Episode::load() + KnowledgeStore::load()
                                        Restores full session state including learned knowledge
```

### 2.2 Session Config Options — Settings, Models, and Modes

> **Note**: ACP's original `session/set_mode` system is **deprecated** as of v0.10.8 (Feb 2026) in favor of Session Config Options. Config Options are strictly more powerful — they support dropdowns, booleans, dependent updates, and categorized grouping. Roko implements both for backward compatibility but defaults to Config Options.

Session Config Options render as **native UI controls** in the editor's agent panel — not text in the chat, not a form dialog, but persistent settings widgets (dropdowns, toggles) visible alongside the conversation. This is how users change models, modes, and behavior.

Two types exist: `select` (dropdown) and `toggle` (boolean on/off, added v0.11.1).

Reserved `category` values: `mode`, `model`, `thought_level`. Custom categories use `_` prefix (e.g., `_roko_gates`).

Roko advertises the following config options on `session/new`:

```json
{
  "configOptions": [
    {
      "id": "agent_mode",
      "name": "Mode",
      "type": "select",
      "category": "mode",
      "currentValue": "code",
      "description": "Agent operating mode",
      "options": [
        {"value": "code", "name": "Code", "description": "Write code with gate verification"},
        {"value": "plan", "name": "Plan", "description": "Architect first — create task DAG, then implement"},
        {"value": "research", "name": "Research", "description": "Deep research with knowledge store queries"},
        {"value": "review", "name": "Review", "description": "Code review — analyze, run gates, suggest improvements"},
        {"value": "auto", "name": "Autonomous", "description": "Full autonomous with conductor watchers and self-correction"}
      ]
    },
    {
      "id": "model_tier",
      "name": "Model",
      "type": "select",
      "category": "model",
      "currentValue": "auto",
      "description": "Model routing strategy",
      "options": [
        {"value": "auto", "name": "Auto (CascadeRouter)", "description": "Routes T0→T3 based on task complexity and budget"},
        {"value": "t0", "name": "T0 — Pattern Match", "description": "Pure Rust, no LLM call, $0/request"},
        {"value": "t1", "name": "T1 — Haiku 4.5", "description": "Fast and cheap — simple edits, lookups"},
        {"value": "t2", "name": "T2 — Sonnet 4.6", "description": "Balanced — most coding tasks"},
        {"value": "t3", "name": "T3 — Opus 4.6", "description": "Maximum quality — architecture, security, complex reasoning"}
      ]
    },
    {
      "id": "thinking",
      "name": "Thinking",
      "type": "select",
      "category": "thought_level",
      "currentValue": "auto",
      "description": "Extended thinking behavior",
      "options": [
        {"value": "auto", "name": "Auto", "description": "Agent decides based on task complexity"},
        {"value": "off", "name": "Off", "description": "No thinking tokens — fastest, cheapest"},
        {"value": "brief", "name": "Brief", "description": "Short reasoning — good default"},
        {"value": "verbose", "name": "Verbose", "description": "Full chain of thought — highest quality, most tokens"}
      ]
    },
    {
      "id": "gate_pipeline",
      "name": "Gate Pipeline",
      "type": "toggle",
      "category": "_roko_verification",
      "currentValue": true,
      "description": "Run compile/test/clippy gates after code changes"
    },
    {
      "id": "auto_correct",
      "name": "Auto-Correct",
      "type": "toggle",
      "category": "_roko_verification",
      "currentValue": true,
      "description": "Conductor watchers auto-fix gate failures"
    },
    {
      "id": "knowledge_store",
      "name": "Knowledge Store",
      "type": "toggle",
      "category": "_roko_memory",
      "currentValue": true,
      "description": "Persist learned heuristics across sessions"
    },
    {
      "id": "daimon",
      "name": "Affect Engine",
      "type": "toggle",
      "category": "_roko_memory",
      "currentValue": true,
      "description": "PAD-based behavioral modulation"
    }
  ]
}
```

**How this looks in practice**: In Zed's agent panel, these render as a settings sidebar — a "Mode" dropdown, a "Model" dropdown, a "Thinking" dropdown, and four toggle switches grouped under "Verification" and "Memory." In JetBrains, they appear in the AI Chat panel's agent configuration area. The user clicks a dropdown, selects "T3 — Opus 4.6", and the next prompt routes through Opus. No slash command, no text input, no configuration file editing.

**Dependent updates**: When the user changes one option, Roko can return updated options for all settings. Example: switching to "Research" mode automatically changes the model tier options to show research-optimized models, disables gate pipeline (research doesn't generate code), and enables knowledge store (research always persists findings):

```rust
async fn handle_config_update(
    session: &mut AcpSession,
    option_id: &str,
    new_value: &str,
) -> Result<Vec<ConfigOption>> {
    match option_id {
        "agent_mode" => {
            session.switch_mode(new_value)?;
            // Return ALL config options with updated values/availability
            Ok(build_config_options_for_mode(new_value))
        }
        "model_tier" => {
            if new_value == "auto" {
                session.cascade_router.enable_auto_routing();
            } else {
                session.cascade_router.force_tier(parse_tier(new_value)?);
            }
            // Update thinking options (T0 doesn't support thinking)
            Ok(build_config_options_for_tier(new_value))
        }
        "gate_pipeline" => {
            session.gates_enabled = new_value == "true";
            // If gates off, auto-correct toggle becomes irrelevant
            Ok(build_config_options_for_gates(session.gates_enabled))
        }
        _ => Ok(vec![]),
    }
}
```

**Backward compatibility with legacy modes**: For older ACP clients that only support `session/set_mode` (not config options), Roko also advertises the mode list via the legacy mechanism. When a client sends `session/set_mode`, Roko translates it to a config option update internally.

### 2.3 Slash Commands

Slash commands are for **actions**, not settings. Config options handle all persistent state (§2.2). Commands handle one-shot operations with text input. Commands support only a `name`, `description`, and optional `input.hint` — no typed arguments, no autocomplete on arguments, no validation.

```json
{
  "availableCommands": [
    {
      "name": "plan",
      "description": "Create a multi-step implementation plan as a task DAG",
      "input": { "hint": "what to build or change" }
    },
    {
      "name": "gate",
      "description": "Run the gate pipeline now (compile, test, clippy)",
      "input": { "hint": "optional: specific gate — compile, test, clippy, all" }
    },
    {
      "name": "learn",
      "description": "Show what the agent has learned — knowledge entries with confidence scores",
      "input": { "hint": "optional: topic to filter by" }
    },
    {
      "name": "inspect",
      "description": "Drill into an episode, engram, or heuristic by ID",
      "input": { "hint": "entity ID or description" }
    },
    {
      "name": "replay",
      "description": "Replay a previous episode — show what the agent did and why",
      "input": { "hint": "episode ID or description" }
    },
    {
      "name": "heuristics",
      "description": "Show applicable heuristics for the current file or context",
      "input": { "hint": "optional: file path or topic" }
    },
    {
      "name": "status",
      "description": "Show agent status — PAD state, vitality, active watchers, knowledge stats"
    },
    {
      "name": "budget",
      "description": "Show remaining token/cost budget and projected session cost"
    }
  ]
}
```

Commands are **dynamic** — `/plan` disappears when already in plan mode, `/gate` shows contextual gates based on the project type (cargo for Rust, forge for Solidity, etc.), and `/heuristics` adjusts based on what files are open. The command list is updated via `available_commands_update` notifications after each prompt and after config option changes.

### 2.4 Agent Plans

ACP has first-class support for rendering multi-step plans with status updates. This is where Roko shines compared to single-loop agents. When Roko creates a plan, the editor renders it as a visual checklist with live status:

```json
{
  "sessionUpdate": "plan",
  "entries": [
    {
      "content": "Analyze existing ERC-4626 vault implementation",
      "priority": "high",
      "status": "completed"
    },
    {
      "content": "Implement share price monotonicity invariant",
      "priority": "high",
      "status": "in_progress"
    },
    {
      "content": "Gate: compile check",
      "priority": "high",
      "status": "pending"
    },
    {
      "content": "Gate: run test suite (147 tests)",
      "priority": "high",
      "status": "pending"
    },
    {
      "content": "Gate: clippy lint pass",
      "priority": "medium",
      "status": "pending"
    },
    {
      "content": "Review: verify no rounding exploits",
      "priority": "medium",
      "status": "pending"
    }
  ]
}
```

As Roko progresses through its `PlanPhase` transitions (Enriching → Implementing → Gating → Verifying → Reviewing → Merging), it sends updated plan notifications with status changes. The user sees each step go from "pending" to "in_progress" to "completed" in real time. When a gate fails, the step shows "pending" again (retry) and a new corrective step appears ("Fix: resolve 3 failing tests").

This is the kind of visibility that no other ACP agent provides. Claude Code, Gemini CLI, and Codex are all opaque — you see tokens streaming, maybe some tool calls, and then a result. Roko shows the full cognitive pipeline.

### 2.5 Tool Calls — Gate Results as Structured Output

When Roko runs its gate pipeline, each gate produces a tool call update with structured results:

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate_compile_001",
  "title": "Compile Gate",
  "kind": "other",
  "status": "completed",
  "content": [
    {
      "type": "text",
      "text": "## Compile Gate: ✓ PASSED\n\n- **Target**: `roko-orchestrator`\n- **Time**: 4.2s\n- **Warnings**: 0\n- **Errors**: 0"
    }
  ]
}
```

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate_test_001",
  "title": "Test Gate",
  "kind": "other",
  "status": "completed",
  "content": [
    {
      "type": "text",
      "text": "## Test Gate: ✓ PASSED\n\n- **Passed**: 147/147\n- **Failed**: 0\n- **Ignored**: 3\n- **Time**: 12.8s\n\n### Coverage\n- Line: 82.4%\n- Branch: 71.2%"
    }
  ]
}
```

The editor renders these as collapsible tool call cards in the chat panel. The user sees objective verification results — not LLM self-assessment, but compiler and test harness output.

### 2.6 File System Bridge

When the editor declares `fs.readTextFile` and `fs.writeTextFile` capabilities during initialization, Roko uses the editor's file system instead of direct disk access. This is critical for two reasons: the editor mediates all file access (security), and file writes show as inline diffs that the user can accept or reject per-hunk (UX).

Roko's `FileSubstrate` trait (L0) already abstracts file I/O. The ACP bridge implements this trait by sending JSON-RPC requests back to the editor:

```
Roko agent needs to read src/main.rs
  → Roko sends fs/read_text_file { path: "/abs/path/to/src/main.rs" }
  → Editor reads the file and returns contents
  → Roko receives file content, proceeds with analysis

Roko agent writes modified src/main.rs  
  → Roko sends fs/write_text_file { path: "/abs/path/to/src/main.rs", content: "..." }
  → Editor shows inline diff: green for additions, red for deletions
  → User reviews the diff, accepts or rejects individual hunks
  → Editor confirms write success back to Roko
```

### 2.7 Terminal Bridge

When the editor declares `terminal` capability, Roko runs shell commands through the editor's terminal instead of spawning processes directly. This means:

- `cargo test` output appears in the editor's terminal panel, not in a hidden subprocess
- The user can see exactly what commands the agent is running
- The editor can kill long-running commands via its own UI
- Terminal output is captured and returned to Roko for analysis

```
Roko needs to run tests:
  → Roko sends terminal/create { command: "cargo", args: ["test", "--lib"], cwd: "/project" }
  → Editor opens terminal tab, runs command, streams output
  → Roko sends terminal/output { terminalId: "term_001" }
  → Editor returns stdout/stderr and exit code
  → Roko analyzes test results, proceeds with gate evaluation
```

### 2.8 Permission Requests

ACP's `session/request_permission` maps to Roko's `SafetyLayer`. Before executing potentially destructive operations (writing files, running commands, modifying git state), Roko requests permission:

```json
{
  "method": "session/request_permission",
  "params": {
    "sessionId": "sess_001",
    "toolCall": {
      "toolCallId": "write_main_rs",
      "title": "Modify src/main.rs",
      "kind": "edit",
      "status": "pending",
      "content": [
        {
          "type": "diff",
          "path": "src/main.rs",
          "diff": "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -42,7 +42,12 @@\n..."
        }
      ]
    },
    "options": [
      { "optionId": "allow", "name": "Allow", "kind": "allow_once" },
      { "optionId": "allow_all", "name": "Allow all edits this turn", "kind": "allow_always" },
      { "optionId": "reject", "name": "Reject", "kind": "reject_once" }
    ]
  }
}
```

The editor shows a native dialog with the diff preview and the three options. This is how Roko's gate-verified workflow looks in practice: the agent proposes a change, the gate pipeline verifies it, and then the user gets a one-click approval with full confidence that the code compiles and tests pass.

### 2.9 Elicitation — Structured Forms for Complex Configuration

> **Status**: Unstable (added v0.11.3–v0.11.5, March–April 2026). Follows MCP's elicitation draft. Not all editors support this yet.

Config options (§2.2) handle simple settings (dropdowns, toggles). Elicitation handles **complex, transient configuration** where the agent needs structured input that doesn't fit a dropdown. The editor renders a native form dialog.

The agent sends `elicitation/create` with a restricted JSON Schema. Supported types: string (with minLength/maxLength/pattern/format), number/integer (with min/max), boolean, enum (single-select), and array of enums (multi-select). **Flat objects only** — no nesting, no arrays of objects, no conditional validation.

Example — Roko asks the user to configure the gate pipeline for a new project:

```json
{
  "method": "elicitation/create",
  "params": {
    "sessionId": "sess_123",
    "mode": "form",
    "message": "Configure gate pipeline for this workspace",
    "requestedSchema": {
      "type": "object",
      "properties": {
        "compile_target": {
          "type": "string",
          "enum": ["workspace", "current-crate", "affected-crates"],
          "description": "What scope to compile"
        },
        "test_filter": {
          "type": "string",
          "description": "Test name filter regex (empty = run all)"
        },
        "clippy_deny_warnings": {
          "type": "boolean",
          "description": "Treat clippy warnings as errors"
        },
        "max_gate_retries": {
          "type": "integer",
          "minimum": 0,
          "maximum": 5,
          "description": "How many times to auto-retry failed gates before stopping"
        },
        "session_budget_usd": {
          "type": "number",
          "minimum": 0.01,
          "maximum": 100.0,
          "description": "Maximum spend for this session (USD)"
        },
        "enabled_gates": {
          "type": "array",
          "items": {
            "type": "string",
            "enum": ["compile", "test", "clippy", "fmt", "doc", "coverage", "security"]
          },
          "minItems": 1,
          "description": "Which gates to run"
        }
      },
      "required": ["compile_target", "enabled_gates"]
    }
  }
}
```

The editor renders this as a form: dropdowns for compile_target, a text field for test_filter, a checkbox for clippy_deny_warnings, a number spinner for max_gate_retries and session_budget_usd, and a multi-select for enabled_gates. The user fills it in, clicks OK, and the response comes back:

```json
{
  "result": {
    "outcome": "accept",
    "data": {
      "compile_target": "affected-crates",
      "test_filter": "",
      "clippy_deny_warnings": true,
      "max_gate_retries": 3,
      "session_budget_usd": 5.0,
      "enabled_gates": ["compile", "test", "clippy", "fmt"]
    }
  }
}
```

**When to use elicitation vs config options**:

| Use Case | Mechanism | Why |
|---|---|---|
| Model tier | Config option (`select`) | Persistent, frequent switching |
| Agent mode | Config option (`select`) | Persistent, core workflow |
| Gate on/off | Config option (`toggle`) | Persistent, simple boolean |
| Gate pipeline configuration | Elicitation (`form`) | Complex, project-specific, one-time setup |
| Budget limit | Elicitation (`form`) | Numeric input needed, not a dropdown |
| MCP server selection | Config option (`select`) | Persistent, editor-provided list |
| Research source selection | Elicitation (`form`) | Multi-select from discovered sources |

Elicitation also supports a `url` mode for OAuth/authentication flows — the agent provides a URL and the editor opens it in a secure browser context. This is relevant for MCP servers that require authentication.

### 2.10 The ACP Interaction Model — What's Possible, What's Not

ACP provides exactly **8 interaction primitives**. Understanding their boundaries prevents designing features that can't be rendered:

| Primitive | Type | Persistent? | What Renders | Limitations |
|---|---|---|---|---|
| **Config Options** | `select` dropdown + `toggle` boolean | Per session | Native sidebar/panel controls | No text input, no sliders, no numeric input |
| **Elicitation** | Flat JSON Schema form | Transient (per request) | Native form dialog | Flat only, no nesting, unstable, not all editors support |
| **Plan** | Flat checklist | Updated per prompt | Checklist with status icons | No hierarchy, no percentage, no nesting. Status is enum (pending/in_progress/completed), not numeric. |
| **Tool Calls** | Structured results with 3 content types | Per invocation | Collapsible cards with status | Content is text, diff, or terminal reference. No custom layouts. |
| **Permission Dialogs** | Allow/reject for tool auth | Per invocation | Native modal dialog | Only 4 option kinds (allow_once, allow_always, reject_once, reject_always). No custom options. |
| **Slash Commands** | User-invoked actions | Dynamic list | Autocomplete menu for command name | Input is unstructured text with a hint. No typed args, no arg autocomplete, no validation. |
| **Text/Markdown** | Agent message stream | Per message | Rendered Markdown + thinking | No HTML, no interactive widgets, no custom components. |
| **Usage Update** | Token/cost reporting | Per prompt | Editor-specific (status bar, widget) | Unstable RFD. Aggregate numbers only. |

**What ACP CANNOT do**:

- No custom components, HTML, or rich widgets
- No sliders, range inputs, color pickers, file pickers
- No progress bars (plan status is enum, not percentage)
- No tree views or hierarchical plans
- No tabs, panels, or tabbed UI from the agent
- No inline buttons or action links in messages
- No charts, graphs, or data visualizations
- No drag-and-drop
- No agent-defined themes or styling
- No streaming images or video
- No autocomplete for slash command arguments
- No multi-step wizard flows (elicitation is one form at a time; can be sequenced but each is independent)

**What IS rich beyond text**:

- **Diffs** (`ToolCallContent::Diff`) — Editors render these as native diff views with per-hunk accept/reject. This is the primary structured interaction for code changes.
- **Terminal embedding** (`ToolCallContent::Terminal`) — Live terminal output visible in the editor's terminal panel.
- **Plan entries** — Visual checklist with status transitions. Editors render pending/in_progress/completed with icons.
- **Tool call cards** — Collapsible cards with title, status icon (spinner/checkmark/error), file locations, and content. Editors render these as a timeline of operations.
- **Config options** — Native dropdown and toggle controls in the settings area, not in the chat.
- **Elicitation forms** — Native form dialogs with proper input types (text, number, checkbox, dropdown, multi-select).

The visual presentation of all of these is **entirely the editor's choice**. The agent provides structured data; the editor renders it however it wants. Zed's rendering of a plan entry may look different from JetBrains' rendering. This is by design — same LSP philosophy.

---

## 3. Architecture

### 3.1 New Crate: `roko-acp`

```
crates/roko-acp/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API: run_acp_server()
│   ├── transport.rs         # Stdio JSON-RPC transport (read/write/notify)
│   ├── handler.rs           # Method dispatch: initialize, session/*, config/*, elicitation/*
│   ├── session.rs           # ACP session ↔ Roko Episode + KnowledgeStore
│   ├── config_options.rs    # Session config options: model tier, mode, thinking, gates, toggles
│   ├── elicitation.rs       # Structured form input: gate config, budget, research sources
│   ├── bridge_fs.rs         # AcpFileSubstrate: impl Substrate via fs/* callbacks
│   ├── bridge_terminal.rs   # AcpTerminal: impl ProcessSupervisor via terminal/* callbacks
│   ├── bridge_events.rs     # Bus Pulse → session/update notification mapper
│   ├── bridge_plan.rs       # PlanPhase → ACP plan entries mapper
│   ├── bridge_gates.rs      # GateResult → ACP tool_call_update mapper
│   ├── bridge_usage.rs      # CostLens → ACP usage_update notification mapper
│   ├── commands.rs          # Slash command definitions and dynamic updates
│   ├── permissions.rs       # SafetyLayer ↔ session/request_permission bridge
│   └── config.rs            # ACP-specific configuration (log file, profile, etc.)
└── tests/
    ├── protocol_conformance.rs  # Validate against ACP spec
    ├── lifecycle.rs             # init → session → prompt → cancel flows
    ├── config_options.rs        # Config option changes, dependent updates
    └── bridge_integration.rs    # End-to-end with mock editor
```

### 3.2 Dependency Graph

```
roko-acp
├── agent-client-protocol   # Official Rust SDK from crates.io (types + transport)
├── roko-core                # L1: Engram, AgentRole, PlanPhase, Verdict, Budget
├── roko-compose             # L2: PromptComposer, RoleSystemPromptSpec
├── roko-gate                # L2: CompileGate, TestGate, ClippyGate
├── roko-agent               # L3: ClaudeCliAgent, ExecAgent, McpConfig
├── roko-orchestrator        # L4: PlanRunner, OrchestratorState, DaimonState
├── roko-fs                  # L0: FileSubstrate trait, RokoLayout
├── bardo-runtime            # L0: CancelToken, EventBus, ProcessSupervisor
├── tokio                    # Async runtime (already in workspace)
├── serde / serde_json       # Serialization (already in workspace)
└── tracing                  # Observability (already in workspace)
```

### 3.3 Entry Point: `roko acp`

New subcommand in `roko-cli`:

```rust
// crates/roko-cli/src/main.rs
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// Start Roko as an ACP agent server (stdio JSON-RPC)
    Acp {
        /// Working directory (defaults to cwd)
        #[arg(long)]
        workdir: Option<PathBuf>,
        
        /// Agent profile to use (defaults to "default")
        #[arg(long, default_value = "default")]
        profile: String,
        
        /// Config file path
        #[arg(long)]
        config: Option<PathBuf>,
        
        /// Log file path (stderr is protocol, must redirect logs)
        #[arg(long, default_value = "/tmp/roko-acp.log")]
        log_file: PathBuf,
    },
}
```

Critical implementation detail: **all logging must go to stderr or a file, never stdout**. Stdout is the JSON-RPC transport — any non-JSON output on stdout corrupts the protocol stream. The `--log-file` flag redirects tracing output.

### 3.4 Layer Placement

`roko-acp` sits alongside `roko-cli` as a presentation layer — it is NOT a new architectural layer. It is an alternative "harness" for the same L4 orchestrator that the CLI and TUI use:

```
┌──────────────────────────────────────────────────────┐
│                  Presentation Layer                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────┐ │
│  │ roko-cli │  │ roko-tui │  │ roko-acp │  │Nunchi│ │
│  │ (term)   │  │ (ratatui)│  │ (stdio)  │  │(web) │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──┬───┘ │
│       │             │             │            │      │
│  ┌────▼─────────────▼─────────────▼────────────▼───┐ │
│  │           L4: roko-orchestrator                  │ │
│  │    PlanRunner + OrchestratorState + DaimonState   │ │
│  └──────────────────┬──────────────────────────────┘ │
│                     │                                 │
│  ┌──────────────────▼──────────────────────────────┐ │
│  │  L3: roko-agent (ClaudeCliAgent, ExecAgent)     │ │
│  │  L2: roko-compose + roko-gate                    │ │
│  │  L1: roko-core                                   │ │
│  │  L0: roko-fs + bardo-runtime                     │ │
│  └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

The key architectural insight: `roko-acp` replaces the I/O substrate (file system and terminal) with editor-mediated versions, but the entire cognitive pipeline — prompt composition, model routing, gate verification, knowledge store, Daimon state — runs identically to the CLI. Same code, same quality, different I/O surface.

---

## 4. Implementation Details

### 4.1 Transport Layer (`transport.rs`)

The transport handles raw JSON-RPC 2.0 over stdin/stdout:

```rust
use agent_client_protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }
    
    /// Read the next JSON-RPC message from stdin.
    /// Blocks until a complete line is available.
    pub async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;
        if line.is_empty() {
            return Err(AcpError::ConnectionClosed);
        }
        let msg: JsonRpcMessage = serde_json::from_str(line.trim())?;
        Ok(msg)
    }
    
    /// Send a JSON-RPC response to stdout.
    pub async fn send_response(&mut self, id: serde_json::Value, result: impl Serialize) -> Result<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        };
        let json = serde_json::to_string(&response)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
    
    /// Send a JSON-RPC notification to stdout (no response expected).
    pub async fn send_notification(&mut self, method: &str, params: impl Serialize) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params: Some(serde_json::to_value(params)?),
        };
        let json = serde_json::to_string(&notification)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
    
    /// Send a JSON-RPC request to the CLIENT (bidirectional).
    /// Used for fs/*, terminal/*, session/request_permission.
    /// Returns the client's response.
    pub async fn send_request<R: DeserializeOwned>(
        &mut self,
        id: u64,
        method: &str,
        params: impl Serialize,
    ) -> Result<R> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::Value::Number(id.into()),
            method: method.into(),
            params: Some(serde_json::to_value(params)?),
        };
        let json = serde_json::to_string(&request)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        
        // Wait for the response with matching id.
        // In practice, use a pending-requests map + channel.
        self.wait_for_response(id).await
    }
}
```

### 4.2 Handler (`handler.rs`)

The main event loop dispatches incoming methods:

```rust
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    // Redirect all logging to file — stdout is the protocol channel
    let _guard = init_file_logging(&config.log_file)?;
    
    let mut transport = StdioTransport::new();
    let mut sessions: HashMap<String, AcpSession> = HashMap::new();
    let mut request_counter: u64 = 1000; // For outbound request IDs
    
    loop {
        let message = transport.read_message().await?;
        
        match message {
            JsonRpcMessage::Request(req) => {
                let result = match req.method.as_str() {
                    "initialize" => {
                        handle_initialize(&config, &req).await
                    }
                    "authenticate" => {
                        handle_authenticate(&config, &req).await
                    }
                    "session/new" => {
                        handle_session_new(&config, &mut sessions, &req).await
                    }
                    "session/load" => {
                        handle_session_load(&config, &mut sessions, &req).await
                    }
                    "session/prompt" => {
                        handle_session_prompt(
                            &config, &mut sessions, &mut transport, 
                            &mut request_counter, &req
                        ).await
                    }
                    "session/set_mode" => {
                        // Legacy — translate to config option update
                        handle_set_mode_legacy(&mut sessions, &req).await
                    }
                    "session/config/update" => {
                        handle_config_update(&mut sessions, &req).await
                    }
                    "session/list" => {
                        handle_session_list(&sessions, &req).await
                    }
                    _ => Err(AcpError::MethodNotFound(req.method.clone())),
                };
                
                match result {
                    Ok(value) => transport.send_response(req.id, value).await?,
                    Err(e) => transport.send_error(req.id, e).await?,
                }
            }
            JsonRpcMessage::Notification(notif) => {
                match notif.method.as_str() {
                    "session/cancel" => {
                        handle_session_cancel(&mut sessions, &notif).await?;
                    }
                    _ => {} // Ignore unknown notifications
                }
            }
            JsonRpcMessage::Response(resp) => {
                // Response to an outbound request (fs/*, terminal/*, permission)
                // Route to the pending request handler
                transport.resolve_pending(resp).await?;
            }
        }
    }
}
```

### 4.3 Initialization (`handler.rs`)

```rust
async fn handle_initialize(
    config: &AcpConfig,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: InitializeParams = serde_json::from_value(req.params.clone().unwrap_or_default())?;
    
    // Store client capabilities for later use
    let client_caps = params.client_capabilities;
    
    Ok(serde_json::to_value(InitializeResult {
        protocol_version: 1,
        agent_capabilities: AgentCapabilities {
            load_session: true,
            prompt_capabilities: PromptCapabilities {
                image: false,     // Enable when vision model backends are wired
                audio: false,
                embedded_context: true,
            },
            mcp_capabilities: McpCapabilities {
                http: true,       // Roko can connect to HTTP MCP servers
                sse: false,       // SSE transport deprecated in MCP spec
            },
        },
        agent_info: AgentInfo {
            name: "roko".into(),
            title: "Roko".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        auth_methods: vec![], // No auth for local stdio
    })?)
}
```

### 4.4 Session Creation (`session.rs`)

```rust
pub struct AcpSession {
    pub session_id: String,
    pub episode: Episode,
    pub knowledge_store: KnowledgeStore,
    pub daimon: DaimonState,
    pub plan_runner: PlanRunner,
    pub cancel_token: CancelToken,
    pub client_caps: ClientCapabilities,
    /// Current config option values — model tier, mode, thinking, gates, etc.
    pub config_state: SessionConfigState,
    /// CostLens projection for usage tracking
    pub usage_bridge: AcpUsageBridge,
}

pub struct SessionConfigState {
    pub agent_mode: String,       // "code" | "plan" | "research" | "review" | "auto"
    pub model_tier: String,       // "auto" | "t0" | "t1" | "t2" | "t3"
    pub thinking: String,         // "auto" | "off" | "brief" | "verbose"
    pub gate_pipeline: bool,
    pub auto_correct: bool,
    pub knowledge_store: bool,
    pub daimon_enabled: bool,
}

async fn handle_session_new(
    config: &AcpConfig,
    sessions: &mut HashMap<String, AcpSession>,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: SessionNewParams = parse_params(req)?;
    let session_id = format!("sess_{}", uuid::Uuid::new_v4().simple());

    // Initialize Roko subsystems for this session
    let knowledge_store = KnowledgeStore::open_or_create(&config.workdir)?;
    let daimon = DaimonState::default();
    let episode = Episode::new(&session_id);
    let plan_runner = PlanRunner::new(
        config.roko_config.clone(),
        knowledge_store.clone(),
        daimon.clone(),
    );
    let config_state = SessionConfigState::defaults_from(&config.roko_config);
    let usage_bridge = AcpUsageBridge::new(session_id.clone());

    let session = AcpSession {
        session_id: session_id.clone(),
        episode,
        knowledge_store,
        daimon,
        plan_runner,
        cancel_token: CancelToken::new(),
        client_caps: params.client_capabilities.unwrap_or_default(),
        config_state,
        usage_bridge,
    };

    sessions.insert(session_id.clone(), session);

    Ok(serde_json::to_value(SessionNewResult {
        session_id,
        // Config options (§2.2) — renders as native UI controls in editor
        config_options: Some(build_config_options(&session.config_state)),
        // Legacy modes for backward compatibility with older ACP clients
        modes: Some(SessionModeState {
            current_mode_id: "code".into(),
            available_modes: roko_modes(),
        }),
    })?)
}
```

### 4.5 Prompt Handling — The Core Bridge (`handler.rs`)

This is the most important function — it connects the ACP prompt lifecycle to Roko's cognitive loop:

```rust
async fn handle_session_prompt(
    config: &AcpConfig,
    sessions: &mut HashMap<String, AcpSession>,
    transport: &mut StdioTransport,
    request_counter: &mut u64,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: SessionPromptParams = parse_params(req)?;
    let session = sessions.get_mut(&params.session_id)
        .ok_or(AcpError::SessionNotFound)?;
    
    // Extract user prompt from ACP content blocks
    let user_prompt = extract_prompt_text(&params.prompt);
    let attached_files = extract_resources(&params.prompt);
    
    // Check for slash commands
    if let Some(command) = parse_slash_command(&user_prompt) {
        return handle_slash_command(session, transport, request_counter, command).await;
    }
    
    // Send initial plan notification based on current mode (from config options)
    let initial_plan = initial_plan_for_mode(&session.config_state.agent_mode, &user_prompt);
    if !initial_plan.is_empty() {
        transport.send_notification("session/update", SessionUpdate {
            session_id: params.session_id.clone(),
            update: UpdateKind::Plan { entries: initial_plan },
        }).await?;
    }
    
    // Advertise dynamic slash commands based on context and config state
    let commands = dynamic_commands(&session.config_state, &user_prompt);
    transport.send_notification("session/update", SessionUpdate {
        session_id: params.session_id.clone(),
        update: UpdateKind::AvailableCommandsUpdate {
            available_commands: commands,
        },
    }).await?;
    
    // Build I/O bridges
    let fs_bridge = if session.client_caps.has_fs() {
        Box::new(AcpFileSubstrate::new(transport.clone(), request_counter))
            as Box<dyn Substrate>
    } else {
        Box::new(FileSubstrate::new(&config.workdir)) as Box<dyn Substrate>
    };
    
    let terminal_bridge = if session.client_caps.has_terminal() {
        Box::new(AcpTerminal::new(transport.clone(), request_counter))
    } else {
        Box::new(LocalTerminal::new())
    };
    
    let permission_bridge = AcpPermissionGate::new(
        transport.clone(), request_counter, params.session_id.clone()
    );
    
    // Create event channel for streaming updates back to editor
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<CognitiveEvent>(256);
    
    // Spawn the cognitive loop in a background task
    let cancel = session.cancel_token.clone();
    let run_config = build_run_config(
        &session, &user_prompt, &attached_files,
        fs_bridge, terminal_bridge, permission_bridge, event_tx,
    );
    
    let cognitive_task = tokio::spawn(async move {
        roko_orchestrator::run_cognitive_loop(run_config, cancel).await
    });
    
    // Stream events from the cognitive loop as ACP notifications
    let stop_reason = stream_events_to_editor(
        &params.session_id, transport, &mut event_rx, &session.cancel_token
    ).await?;
    
    // Wait for the cognitive task to complete
    let outcome = cognitive_task.await??;
    
    // Persist knowledge learned during this turn
    session.knowledge_store.flush()?;
    session.episode.record_turn(&user_prompt, &outcome);

    // Push final usage update (context window + cost)
    session.usage_bridge.push_usage_update(transport).await?;

    Ok(serde_json::to_value(SessionPromptResult {
        stop_reason: match stop_reason {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::Cancelled => "cancelled",
            StopReason::Refused => "refusal",
        },
        // Per-turn token breakdown (§12.1)
        usage: Some(session.usage_bridge.turn_usage()),
    })?)
}

/// Stream CognitiveEvents as ACP session/update notifications.
/// Returns when the cognitive loop completes or is cancelled.
async fn stream_events_to_editor(
    session_id: &str,
    transport: &mut StdioTransport,
    event_rx: &mut mpsc::Receiver<CognitiveEvent>,
    cancel: &CancelToken,
) -> Result<StopReason> {
    while let Some(event) = event_rx.recv().await {
        if cancel.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        
        match event {
            // --- Streaming text from LLM ---
            CognitiveEvent::TokenChunk(text) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::AgentMessageChunk {
                        content: ContentBlock::Text { text },
                    },
                }).await?;
            }
            
            // --- Thinking/reasoning tokens (shown as thought bubbles) ---
            CognitiveEvent::ThinkingChunk(text) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ThoughtMessageChunk {
                        content: ContentBlock::Text { text },
                    },
                }).await?;
            }
            
            // --- Tool call lifecycle ---
            CognitiveEvent::ToolCallStart { id, name, args } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCall {
                        tool_call_id: id,
                        title: human_readable_tool_name(&name),
                        kind: tool_kind(&name),
                        status: "pending".into(),
                        content: None,
                    },
                }).await?;
            }
            
            CognitiveEvent::ToolCallComplete { id, result } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCallUpdate {
                        tool_call_id: id,
                        status: "completed".into(),
                        content: Some(vec![ContentBlock::Text {
                            text: result,
                        }]),
                    },
                }).await?;
            }
            
            // --- Gate pipeline results ---
            CognitiveEvent::GateStarted(gate) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCall {
                        tool_call_id: format!("gate_{}", gate.name),
                        title: format!("{} Gate", gate.name),
                        kind: "other".into(),
                        status: "in_progress".into(),
                        content: None,
                    },
                }).await?;
            }
            
            CognitiveEvent::GateCompleted(gate) => {
                let status_emoji = if gate.passed { "✓" } else { "✗" };
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCallUpdate {
                        tool_call_id: format!("gate_{}", gate.name),
                        status: "completed".into(),
                        content: Some(vec![ContentBlock::Text {
                            text: format!(
                                "## {} Gate: {} {}\n\n{}\n\n**Time**: {:.1}s",
                                gate.name,
                                status_emoji,
                                if gate.passed { "PASSED" } else { "FAILED" },
                                gate.summary,
                                gate.duration.as_secs_f64(),
                            ),
                        }]),
                    },
                }).await?;
            }
            
            // --- Plan phase transitions ---
            CognitiveEvent::PhaseTransition(phase) => {
                let plan = build_plan_from_phase(&phase);
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::Plan { entries: plan },
                }).await?;
            }
            
            // --- Conductor watcher triggers ---
            CognitiveEvent::WatcherTriggered { watcher, action } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::AgentMessageChunk {
                        content: ContentBlock::Text {
                            text: format!(
                                "\n> ⚡ **{}** triggered: {}\n",
                                watcher, action
                            ),
                        },
                    },
                }).await?;
            }
            
            // --- Session complete ---
            CognitiveEvent::Complete(outcome) => {
                return Ok(StopReason::EndTurn);
            }
            
            CognitiveEvent::MaxTokens => {
                return Ok(StopReason::MaxTokens);
            }
        }
    }
    
    Ok(StopReason::EndTurn)
}
```

### 4.6 File System Bridge (`bridge_fs.rs`)

```rust
/// ACP-mediated file system that routes through the editor.
/// Implements the same Substrate trait as FileSubstrate,
/// so the cognitive loop is unaware of the transport difference.
pub struct AcpFileSubstrate {
    transport: StdioTransport,
    request_counter: Arc<AtomicU64>,
}

impl Substrate for AcpFileSubstrate {
    async fn read_file(&self, path: &Path) -> Result<String> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let result: ReadTextFileResult = self.transport.send_request(
            id,
            "fs/read_text_file",
            ReadTextFileParams {
                path: path.to_string_lossy().to_string(),
            },
        ).await?;
        Ok(result.text)
    }
    
    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let _result: WriteTextFileResult = self.transport.send_request(
            id,
            "fs/write_text_file",
            WriteTextFileParams {
                path: path.to_string_lossy().to_string(),
                content: content.to_string(),
            },
        ).await?;
        Ok(())
    }
    
    async fn list_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        // ACP doesn't have a list_files method — fall back to local FS
        // This is fine because listing is read-only and non-destructive
        tokio::fs::read_dir(dir).await?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }
}
```

### 4.7 Terminal Bridge (`bridge_terminal.rs`)

```rust
pub struct AcpTerminal {
    transport: StdioTransport,
    request_counter: Arc<AtomicU64>,
    active_terminals: HashMap<String, TerminalHandle>,
}

impl AcpTerminal {
    pub async fn run_command(
        &mut self,
        command: &str,
        args: &[&str],
        cwd: &Path,
    ) -> Result<CommandOutput> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        
        // Create terminal in editor
        let create_result: TerminalCreateResult = self.transport.send_request(
            id,
            "terminal/create",
            TerminalCreateParams {
                command: command.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                cwd: Some(cwd.to_string_lossy().to_string()),
                env: None,
            },
        ).await?;
        
        let terminal_id = create_result.terminal_id;
        
        // Wait for command to exit
        let wait_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let _wait_result: TerminalWaitResult = self.transport.send_request(
            wait_id,
            "terminal/wait_for_exit",
            TerminalWaitParams {
                terminal_id: terminal_id.clone(),
            },
        ).await?;
        
        // Get output
        let output_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let output_result: TerminalOutputResult = self.transport.send_request(
            output_id,
            "terminal/output",
            TerminalOutputParams {
                terminal_id: terminal_id.clone(),
            },
        ).await?;
        
        // Release terminal
        let release_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        self.transport.send_request::<()>(
            release_id,
            "terminal/release",
            TerminalReleaseParams { terminal_id },
        ).await.ok(); // Best-effort release
        
        Ok(CommandOutput {
            stdout: output_result.stdout,
            stderr: output_result.stderr,
            exit_code: output_result.exit_code,
        })
    }
}
```

### 4.8 MCP Passthrough

When the editor has MCP servers configured, it passes their configuration to the agent during session setup. Roko connects to these MCP servers directly — the editor provides the connection info, and Roko's existing `roko-agent::mcp::McpConfig` handles the connection:

```rust
async fn handle_session_new(/* ... */) -> Result<serde_json::Value> {
    let params: SessionNewParams = parse_params(req)?;
    
    // If the editor provides MCP server configs, merge them with Roko's own
    if let Some(mcp_configs) = params.mcp_servers {
        for mcp in mcp_configs {
            session.plan_runner.add_mcp_server(McpConfig {
                name: mcp.name,
                transport: match mcp.transport {
                    McpTransport::Stdio { command, args } => {
                        McpTransportConfig::Stdio { command, args }
                    }
                    McpTransport::Http { url } => {
                        McpTransportConfig::Http { url }
                    }
                },
            });
        }
    }
    
    // ... rest of session creation
}
```

This means if a user has a GitHub MCP server, a database MCP server, or any other tools configured in their Zed/JetBrains setup, Roko automatically gets access to them.

---

## 5. Editor-Specific Integration Details

### 5.1 JetBrains — Registry Installation

Once implemented, Roko is submitted to the ACP Registry. Users install it from inside their IDE:

1. Open the AI Chat tool window
2. Click the agent selector dropdown
3. Select "Install From ACP Registry"
4. Find "Roko" → click Install
5. The IDE downloads the `roko` binary, configures `acp.json`, and Roko appears in the agent selector

Alternatively, manual configuration via `~/.jetbrains/acp.json`:

```json
{
  "default_mcp_settings": {
    "use_custom_mcp": true,
    "use_idea_mcp": true
  },
  "agent_servers": {
    "Roko": {
      "command": "roko",
      "args": ["acp"],
      "env": {
        "ROKO_LOG_LEVEL": "info"
      }
    }
  }
}
```

With `use_idea_mcp: true`, Roko gets access to IntelliJ's built-in MCP server — which provides refactoring tools, code analysis, test running, and other IDE-native capabilities. This means Roko can use IntelliJ's own refactoring engine in addition to its own gate pipeline.

### 5.2 Zed — Native Integration

Zed settings (`~/.config/zed/settings.json`):

```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

Or use a keyboard shortcut to open a new Roko thread:

```json
[
  {
    "bindings": {
      "cmd-alt-r": [
        "agent::NewExternalAgentThread",
        {
          "agent": {
            "custom": {
              "name": "Roko",
              "command": {
                "command": "roko",
                "args": ["acp"]
              }
            }
          }
        }
      ]
    }
  }
]
```

### 5.3 Neovim — Via avante.nvim

```lua
-- In your init.lua or plugin config
require('avante').setup({
  acp_providers = {
    ["roko"] = {
      command = "roko",
      args = { "acp" },
      env = {}
    }
  }
})
```

### 5.4 Obsidian — Via obsidian-agent-client

```json
{
  "agents": {
    "Roko": {
      "command": "roko",
      "args": ["acp", "--workdir", "/path/to/vault"],
      "env": {}
    }
  }
}
```

This creates a loop between Obsidian (human knowledge surface) and Roko's knowledge store (agent knowledge surface). Research notes in Obsidian can inform Roko's context; Roko's learned heuristics can be surfaced as notes in Obsidian.

---

## 6. Roko-Specific ACP Extensions

ACP supports custom extensions via the `_meta` field and underscore-prefixed methods. Roko uses these to expose capabilities that the base protocol doesn't cover:

### 6.1 Knowledge Store Queries (Extension)

```json
{
  "method": "session/update",
  "params": {
    "sessionId": "sess_001",
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": {
        "type": "text",
        "text": "Found 3 relevant knowledge entries..."
      },
      "_meta": {
        "_roko.knowledge_entries": [
          {
            "id": "engram_0x3f2a",
            "type": "Heuristic",
            "content": "ERC-4626 vaults must ensure share price monotonicity",
            "confidence": 0.91,
            "confirmations": 89
          }
        ]
      }
    }
  }
}
```

Editors that don't understand Roko's extensions safely ignore the `_meta` field. Editors that do (Nunchi, or a future Roko Zed extension) can render knowledge entries as rich cards.

### 6.2 Gate Pipeline Status (Extension)

```json
{
  "_meta": {
    "_roko.gate_pipeline": {
      "gates": [
        { "name": "compile", "status": "passed", "duration_ms": 4200 },
        { "name": "test", "status": "passed", "tests_passed": 147, "tests_total": 147 },
        { "name": "clippy", "status": "passed", "warnings": 0 }
      ],
      "overall": "passed"
    }
  }
}
```

### 6.3 Daimon State (Extension)

```json
{
  "_meta": {
    "_roko.daimon": {
      "pleasure": 0.72,
      "arousal": 0.45,
      "dominance": 0.81,
      "mood": "confident",
      "strategy_shifts": 0
    }
  }
}
```

---

## 7. Testing Strategy

### 7.1 Protocol Conformance Tests

Test against the ACP spec using the official Rust SDK's test utilities:

```rust
#[tokio::test]
async fn test_initialize_handshake() {
    let (mut client, mut server) = create_test_pair().await;
    
    let response = client.send_initialize(InitializeParams {
        protocol_version: 1,
        client_capabilities: ClientCapabilities {
            fs: Some(FsCapabilities { read_text_file: true, write_text_file: true }),
            terminal: Some(true),
        },
        client_info: Some(ClientInfo {
            name: "test-client".into(),
            version: "1.0.0".into(),
            title: None,
        }),
    }).await.unwrap();
    
    assert_eq!(response.protocol_version, 1);
    assert!(response.agent_capabilities.load_session);
    assert_eq!(response.agent_info.unwrap().name, "roko");
}

#[tokio::test]
async fn test_prompt_streams_gate_results() {
    let (mut client, mut server) = create_test_pair().await;
    initialize_and_create_session(&mut client).await;
    
    // Send a prompt that will trigger the gate pipeline
    let updates = client.send_prompt_and_collect_updates(
        "Fix the failing test in src/lib.rs",
    ).await.unwrap();
    
    // Verify gate results appear as tool call updates
    let gate_updates: Vec<_> = updates.iter()
        .filter(|u| matches!(u, UpdateKind::ToolCall { title, .. } if title.contains("Gate")))
        .collect();
    
    assert!(!gate_updates.is_empty(), "Gate results should be reported as tool calls");
}
```

### 7.2 Editor Integration Tests

Run Roko as a subprocess and communicate via stdio, simulating what each editor does:

```rust
#[tokio::test]
async fn test_end_to_end_stdio() {
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "roko", "--", "acp", "--workdir", "/tmp/test-project"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut transport = TestTransport::new(stdin, stdout);
    
    // Full lifecycle test
    transport.send_initialize().await;
    let session_id = transport.send_session_new().await;
    let updates = transport.send_prompt(&session_id, "Hello, what can you do?").await;
    
    assert!(updates.iter().any(|u| matches!(u, UpdateKind::AgentMessageChunk { .. })));
    
    child.kill().await.unwrap();
}
```

### 7.3 Bridge Tests

Test the file system and terminal bridges independently:

```rust
#[tokio::test]
async fn test_fs_bridge_read_write() {
    let (mut client, mut server) = create_test_pair().await;
    let fs = AcpFileSubstrate::new(server.transport.clone(), Arc::new(AtomicU64::new(0)));
    
    // Mock the client responding to fs/read_text_file
    client.on_request("fs/read_text_file", |params| {
        Ok(ReadTextFileResult { text: "fn main() {}".into() })
    });
    
    let content = fs.read_file(Path::new("/test/main.rs")).await.unwrap();
    assert_eq!(content, "fn main() {}");
}
```

---

## 8. ACP Registry Submission

### 8.1 Registry Manifest

The ACP Registry requires a manifest describing the agent:

```json
{
  "name": "roko",
  "title": "Roko",
  "description": "Multi-agent coding runtime with gate-verified execution, knowledge store, and multi-phase planning",
  "version": "0.1.0",
  "homepage": "https://github.com/roko-project/roko",
  "license": "MIT",
  "platforms": {
    "darwin-arm64": {
      "command": "roko",
      "args": ["acp"]
    },
    "darwin-x64": {
      "command": "roko",
      "args": ["acp"]
    },
    "linux-x64": {
      "command": "roko",
      "args": ["acp"]
    },
    "win32-x64": {
      "command": "roko.exe",
      "args": ["acp"]
    }
  },
  "capabilities": {
    "loadSession": true,
    "promptCapabilities": {
      "embeddedContext": true
    }
  }
}
```

### 8.2 Distribution

The registry downloads the agent binary. Two options:

- **Cargo install**: Users need Rust toolchain. `cargo install roko`
- **Pre-built binaries**: GitHub releases with binaries for macOS (arm64/x64), Linux (x64), Windows (x64). The registry manifest points to these.

Pre-built binaries are strongly preferred for the registry — zero-friction installation is the point.

---

## 9. Implementation Roadmap

### Phase 1: Core Protocol (Week 1–2)

**Goal**: `roko acp` compiles, initializes, creates sessions, handles prompts with streaming.

- Create `roko-acp` crate with transport, handler, session management
- Implement `initialize`, `session/new`, `session/prompt`, `session/cancel`
- Implement `stream_events_to_editor` with `agent_message_chunk` and `thought_message_chunk`
- Wire `session/prompt` to `roko_orchestrator::run_cognitive_loop` with local file system (no bridges yet)
- Redirect all logging to stderr/file
- Protocol conformance tests for lifecycle

**Acceptance criteria**: Run `roko acp`, connect with a test harness, send a prompt, receive streamed response.

### Phase 2: Bridges (Week 3–4)

**Goal**: Editor-mediated file I/O, terminal, and permissions.

- Implement `AcpFileSubstrate` (bridge_fs.rs) — `fs/read_text_file`, `fs/write_text_file`
- Implement `AcpTerminal` (bridge_terminal.rs) — `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`
- Implement `AcpPermissionGate` (permissions.rs) — `session/request_permission`
- Wire gate pipeline results as `tool_call` / `tool_call_update` notifications
- Wire plan phase transitions as `plan` notifications
- MCP passthrough from editor MCP configs to `roko-agent::McpConfig`

**Acceptance criteria**: Connect from Zed or JetBrains. File edits show as diffs. Terminal commands run in editor terminal. Gate results display as tool cards. Plans show as step-by-step checklists.

### Phase 3: Config Options, Commands, Elicitation, Sessions (Week 5–6)

**Goal**: Full Roko-specific UX through ACP's config option/command/elicitation/session features.

- Implement Session Config Options (§2.2): model tier, agent mode, thinking level, gate toggle, auto-correct toggle, knowledge store toggle, daimon toggle
- Implement dependent option updates (mode change → update available models/gates)
- Implement `session/config/update` handler for all config option changes
- Implement legacy `session/set_mode` for backward compatibility with older ACP clients
- Implement slash commands (/plan, /gate, /learn, /inspect, /replay, /heuristics, /status, /budget) with dynamic updates
- Implement Elicitation forms (§2.9) for gate pipeline configuration, budget limits, research source selection
- Implement `session/load` with knowledge store and Daimon state persistence
- Implement `session/list` for session history
- Wire `usage_update` notifications from CostLens (unstable RFD)
- Roko-specific `_meta` extensions for knowledge entries, gate pipeline, Daimon state

**Acceptance criteria**: User changes model tier via dropdown in editor settings panel. Mode switch updates dependent options. Slash commands autocomplete. Elicitation form appears for gate configuration on first use. Sessions persist and reload with learned knowledge. Token/cost tracking visible in editor.

### Phase 4: Registry & Distribution (Week 7–8)

**Goal**: One-click installation from JetBrains/Zed.

- Set up cross-compilation for macOS/Linux/Windows binaries
- Create GitHub Actions workflow for release binaries
- Write ACP registry manifest
- Submit to ACP registry
- Write user-facing documentation (README, getting started guide)
- End-to-end testing with JetBrains 2025.3+, Zed, Neovim (avante.nvim)
- Performance profiling (startup time, response latency, memory usage)

**Acceptance criteria**: User installs Roko from JetBrains ACP Registry dropdown. Works out of the box.

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| ACP pre-1.0 breaking changes (now v0.12.2, 41 releases) | Protocol methods change, types restructure | Pin `agent-client-protocol` crate version. Abstract all ACP types behind internal trait boundary. Monitor ACP changelog. Join community Discord. |
| stdout pollution from dependencies | Any non-JSON output on stdout corrupts the protocol stream | All logging to file via `--log-file`. Set `tracing` subscriber to file/stderr only. Test with `RUST_LOG=off` as default in ACP mode. |
| Bidirectional JSON-RPC complexity | Deadlocks if agent and editor both waiting for responses simultaneously | Use a pending-requests map with `oneshot` channels. Separate reader and writer tasks. Timeout on all outbound requests (30s default). |
| Startup time for Roko binary | If `roko acp` takes >2s to start, editors feel sluggish | Lazy-load heavy subsystems (knowledge store, model backends). Only initialize when first `session/new` arrives. Profile and optimize startup path. |
| Memory usage per session | Knowledge store + Daimon state per session adds up with concurrent sessions | Session limit (default: 4 concurrent). LRU eviction for idle sessions. Share knowledge store read handle across sessions. |
| Editor-specific quirks | JetBrains and Zed may interpret ACP slightly differently | Test against both. Join ACP community Discord for spec clarifications. File issues on `agentclientprotocol/agent-client-protocol` for ambiguities. |

---

## 11. What This Unlocks

### 11.1 Immediate

- Roko agents usable from JetBrains, Zed, Neovim, Emacs, VS Code (community), Obsidian, Toad — with zero per-editor integration work
- One-click install from ACP Registry for 30M+ JetBrains users and all Zed users
- Gate-verified coding in any editor (no other ACP agent does this — see §17.2)
- Persistent knowledge across sessions (no other ACP agent does this)
- Multi-phase plan visibility in the editor (no other ACP agent does this)
- Real-time cost tracking via ACP session usage protocol (see §12)
- Token-level budget enforcement via vitality system (see §12.5)

### 11.2 Strategic

- **Nunchi role shift**: Nunchi shifts from "the only way to use Roko" to "the fleet management surface." Individual coding tasks happen in the IDE via ACP. Fleet orchestration, knowledge visualization, arena competition, and collective intelligence monitoring happen in Nunchi. The ACP layer handles the individual developer; Nunchi handles the organization.
- **Distribution**: ACP Registry provides one-click install. Roko sits alongside Claude Code, Gemini CLI, Codex, Cursor, Goose, Kiro — as a peer, not a subordinate. The registry handles downloading, versioning, and auto-updates.
- **Differentiation**: In a registry of 33+ single-loop LLM wrappers, Roko is the only agent showing "147/147 tests passed, verified by compiler" in tool call cards. Gate verification is visible, not hidden.
- **VS Code pathway**: Even without native ACP support, Roko reaches VS Code users via community ACP extension (now) and potentially via Chat Participant extension (later). MCP fallback is already wired. See §13.1 for the dual-track strategy.
- **MCP composability**: Every editor that supports MCP (all major editors as of 2026) can access Roko's gate pipeline and knowledge store as tools, even without ACP. ACP is the premium experience; MCP is the universal fallback.

### 11.3 Composition with A2A

ACP and A2A compose naturally (see §15.2 for the full flow). A user working in JetBrains via ACP asks Roko to "audit this contract using the remote audit agent." Roko receives this via ACP, discovers the remote agent via A2A agent cards (`/.well-known/agent-card.json`), delegates the audit task over HTTP, streams A2A `TaskStatusUpdateEvent`s back to the editor through ACP's `session/update` notifications. The user sees a seamless experience: they typed a prompt in their IDE and got back a gate-verified audit result, without knowing that two protocols, three transports, and a remote agent were involved.

### 11.4 Composition with Editor-Native Agents

The most powerful pattern is Roko + editor-native agent:

- **Roko + Junie (JetBrains)**: Junie provides IDE-native refactoring and code analysis via PSI. Roko provides gate verification and knowledge. User asks Roko to "refactor the auth module." Roko delegates refactoring to Junie via JetBrains MCP server, then runs gate pipeline on the result.
- **Roko + Copilot (VS Code)**: Copilot provides inline suggestions and agent mode. Roko provides gate verification as an MCP tool. Copilot's agent mode calls `roko_gate_run` to verify its own output.
- **Roko + Claude Code (terminal)**: Claude Code provides the general-purpose agent. Roko provides structured knowledge and gate pipeline. Claude Code's subagent system delegates verification to Roko.

In each case, Roko is the verification and knowledge layer; the editor's native agent is the generation layer. This is the **Variance Inequality** ([doc-02](02-CELL.md)) made practical: the verifier (Roko's gate pipeline) is spectrally cleaner than the generator (LLM).

---

---

## 12. Token Usage & Cost Tracking — The Economic Layer

ACP has a dedicated specification for session usage and context status tracking ([RFD: Session Usage](https://agentclientprotocol.com/rfds/session-usage)). This is where Roko's CostLens ([doc-09](09-TELEMETRY.md)) meets the protocol.

### 12.1 ACP Usage Protocol

**Per-turn reporting** — included in `PromptResponse`:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "sessionId": "sess_abc123",
    "stopReason": "end_turn",
    "usage": {
      "total_tokens": 53000,
      "input_tokens": 35000,
      "output_tokens": 12000,
      "thought_tokens": 5000,
      "cached_read_tokens": 5000,
      "cached_write_tokens": 1000
    }
  }
}
```

Required fields: `total_tokens`, `input_tokens`, `output_tokens`. Optional: `thought_tokens`, `cached_read_tokens`, `cached_write_tokens`.

**Session-level reporting** — via `session/update` notification:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "sess_abc123",
    "update": {
      "sessionUpdate": "usage_update",
      "used": 53000,
      "size": 200000,
      "cost": {
        "amount": 0.045,
        "currency": "USD"
      }
    }
  }
}
```

Context window fields (required): `used`, `size`. Cost fields (optional): `cost.amount`, `cost.currency` (ISO 4217).

**Update timing**: On `session/new`, `session/load`, `session/resume`, after each `session/prompt`, and on significant context window changes.

**Client guidance for context thresholds**: <75% normal, 75–90% suggest management, 90–95% recommend new session, >95% warn next prompt may fail.

### 12.2 Roko's CostLens → ACP Usage Bridge

Roko's CostLens (a Lens specialization from [doc-09](09-TELEMETRY.md)) computes real-time cost attribution per Cell per Graph per Agent. The ACP bridge maps this to the protocol's usage fields:

```rust
/// Bridge CostLens output to ACP usage_update notifications.
pub struct AcpUsageBridge {
    session_id: String,
    transport: StdioTransport,
    /// Running totals for this session
    cumulative_tokens: TokenAccumulator,
    /// CostLens projection from StateHub
    cost_projection: watch::Receiver<CostProjection>,
}

impl AcpUsageBridge {
    /// Called after each prompt completes. Returns usage for PromptResponse.
    pub fn turn_usage(&self) -> AcpUsage {
        let turn = self.cumulative_tokens.current_turn();
        AcpUsage {
            total_tokens: turn.total,
            input_tokens: turn.input,
            output_tokens: turn.output,
            thought_tokens: Some(turn.thinking),
            cached_read_tokens: Some(turn.cache_read),
            cached_write_tokens: Some(turn.cache_write),
        }
    }

    /// Called periodically or on significant context changes.
    /// Sends usage_update notification to the editor.
    pub async fn push_usage_update(&self) -> Result<()> {
        let cost = self.cost_projection.borrow();
        self.transport.send_notification("session/update", SessionUpdate {
            session_id: self.session_id.clone(),
            update: UpdateKind::UsageUpdate {
                used: self.cumulative_tokens.total_tokens(),
                size: self.effective_context_window(),
                cost: Some(AcpCost {
                    amount: cost.cumulative_usd,
                    currency: "USD".into(),
                }),
            },
        }).await
    }

    /// Context window accounting. Roko uses VCG auction for context assembly,
    /// so the "used" count is the actual tokens after composition, not a naive sum.
    fn effective_context_window(&self) -> u64 {
        let cost = self.cost_projection.borrow();
        cost.context_tokens_used
    }
}

pub struct TokenAccumulator {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub thinking: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    /// Per-turn snapshots for turn-level reporting
    turns: Vec<TurnSnapshot>,
}
```

### 12.3 Cost Attribution — Unique to Roko

No other ACP agent provides cost attribution at this granularity. Roko can report:

1. **Per-turn cost** — how much this prompt cost (input + output + thinking tokens × model price).
2. **Per-gate cost** — how much each gate run cost (compile gate = terminal time, test gate = terminal time + coverage analysis).
3. **Per-phase cost** — Enriching vs Implementing vs Gating vs Reviewing, with percentage breakdowns.
4. **Cache savings** — how much was saved by prompt caching (Roko's VCG auction maximizes cache hits via section effect tracking).
5. **Model routing savings** — how much was saved by CascadeRouter routing to cheaper models for simple tasks.
6. **Vitality burn rate** — tokens/minute, projected session cost, budget remaining.

The ACP usage protocol only supports aggregate numbers. Roko surfaces the detailed breakdown through `_meta` extensions:

```json
{
  "_meta": {
    "_roko.cost_breakdown": {
      "model_costs": [
        { "model": "claude-sonnet-4-6", "tokens": 35000, "cost_usd": 0.028 },
        { "model": "claude-haiku-4-5", "tokens": 12000, "cost_usd": 0.003 }
      ],
      "phase_costs": {
        "enriching": 0.005,
        "implementing": 0.020,
        "gating": 0.008,
        "reviewing": 0.012
      },
      "cache_savings_usd": 0.015,
      "routing_savings_usd": 0.022,
      "vitality": 0.73,
      "burn_rate_usd_per_min": 0.012,
      "session_projected_total_usd": 0.45
    }
  }
}
```

### 12.4 Token Tracking Across Editors — How They Compare

Understanding how each editor tracks tokens/costs informs where Roko's ACP usage reporting fits:

| Editor | Token/Cost Tracking | How It Works | What Roko Adds |
|---|---|---|---|
| **Zed** | Dashboard + in-editor counter | Token counter near profile selector. Dashboard at `dashboard.zed.dev/account`. Max spend control. Zed Pro includes $5/mo credit. | Roko provides per-turn + session-level usage via ACP protocol. Zed renders it natively. |
| **JetBrains** | AI widget + quota bar | Progress bar in toolbar showing remaining cloud credits. Quota calculated from input + output tokens dynamically. | Roko reports via ACP usage protocol. JetBrains renders in AI widget. |
| **VS Code** | Status bar + 3rd-party extensions | Copilot status dashboard shows inline/chat/premium request quotas. Extensions like "Eating Token" and "AI Engineering Fluency" show per-request costs. | With ACP community extension, Roko usage data flows through. Native VS Code integration pending. |
| **Cursor** | In-editor dashboard (Feb 2026) | Credit-based billing since Aug 2025. Dashboard shows remaining Auto + Composer + API balances. Composer 2 achieves 200+ tok/s. | Roko provides raw token counts. Cursor maps to credits on its end. |
| **Windsurf** | Credits only | Credit-based tiers (Free 25/mo, Pro 500/mo). No per-token visibility. | Roko can expose per-token detail that Windsurf doesn't natively surface. |
| **Neovim** | None built-in | No token tracking. Users rely on provider dashboards. | Roko's `/budget` slash command and session usage provide visibility absent from the editor. |
| **Claude Code CLI** | `/usage` + `/cost` commands | Per-model cost breakdown, cache hit rates, rate-limit utilization. Dollar estimates computed locally from token counts. | Roko operates at the same fidelity. Interop: Roko as agent, Claude Code as tool. |

### 12.5 Budget Enforcement via ACP

Roko's vitality system ([doc-07](07-AGENT-RUNTIME.md)) provides budget enforcement that no other ACP agent offers. When a user sets a spending limit:

1. Budget is configured in `roko.toml` → `[agent.budget]` section.
2. VitalityTracker decrements on every LLM call and terminal operation.
3. When vitality enters Conservation phase (<40% remaining), Roko:
   - Routes to cheaper models (CascadeRouter shifts T2 → T1)
   - Sends a `usage_update` with cost projection via ACP
   - Sends an agent message: "Budget at 40%. Switching to conservation mode."
4. When vitality enters Declining phase (<20% remaining), Roko:
   - Requests permission before any further LLM calls
   - Shows estimated cost of remaining work
5. When budget exhausted, Roko sends `stop_reason: "max_tokens"` and gracefully terminates.

The editor sees all of this through standard ACP notifications — no special extension needed.

---

## 13. Editor-Specific Deep Dives

### 13.1 VS Code — The 75M User Gap

VS Code has no native ACP support. This is the single largest gap in the ACP ecosystem — 75M monthly active users unreachable by any ACP agent.

**Current state**:
- Feature request: [microsoft/vscode#265496](https://github.com/microsoft/vscode/issues/265496) — open, no ETA.
- Community extension: [vscode-acp](https://github.com/formulahendry/vscode-acp) by Henry Li — basic implementation.
- Marketplace extension: [ACP Plugin](https://marketplace.visualstudio.com/items?itemName=strato-space.acp-plugin) — available.

**VS Code's own AI architecture** (competing paradigm):
- **Agent Mode** (GA April 2025): Autonomous peer programmer. Multi-file refactoring, test running, auto-correction.
- **Multi-Agent Development** (VS Code 1.109, Feb 2026): Run Claude, Codex, and Copilot agents simultaneously under a single GitHub Copilot subscription. Agent Sessions view provides unified control plane.
- **Custom Agents**: `.agent.md` files in `.github/agents/` directories with tools + instructions + model selection.
- **Agent Plugins** (VS Code 1.110): Pre-packaged bundles of chat customizations installable from marketplace.
- **Chat Participant API**: Extensions define specialized chat participants invoked via `@mention`.
- **Language Model Tools API**: Extensions contribute tools to Copilot.
- **MCP support**: GA since July 2025. Full specification support including authorization, prompts, resources, sampling.

**Strategic implication**: VS Code has built its own agent integration paradigm around the Chat Participant API and Agent Plugins. ACP adoption is not guaranteed. Roko's VS Code strategy should be dual-track:

1. **ACP via community extension**: Works today. Limited UX (no native diff rendering, no terminal integration in the ACP sense). Good enough for early adopters.
2. **VS Code Chat Participant extension**: Registers Roko as a `@roko` chat participant. Uses VS Code's native diff rendering, terminal panel, and file system access. More work but better UX.
3. **MCP server fallback**: Roko already has MCP. VS Code's MCP support means Roko tools are available to any VS Code agent (Copilot, Claude, Codex) without ACP.

Priority: Track 1 (free), Track 3 (already done), Track 2 (if VS Code ACP doesn't materialize by Q4 2026).

### 13.2 Cursor — Agent-Side ACP, Not Client-Side

Cursor has ACP support but in the wrong direction: **Cursor can act as an ACP agent** (it has CLI docs for being invoked via ACP), but **Cursor cannot act as an ACP client** (it doesn't spawn external ACP agents).

Cursor's AI architecture is deeply integrated:
- **Agent mode** (Composer): Primary AI interface, multi-step autonomous coding.
- **Async subagents**: `/multitask` spawns parallel subagents that decompose tasks.
- **Background/Cloud agents** (GA late 2025): Cloud-based Ubuntu VMs that clone repos, do work, and open PRs. Triggerable from IDE, Slack, or web/mobile.
- **Semantic indexing**: AST-aware code splitting → embeddings → Turbopuffer vector DB. Cross-user index reuse (92% similarity for repo clones).
- **Plugin marketplace** (Feb 2026): MCP servers, skills, subagents, hooks, and rules.
- **Credit-based billing**: No per-token visibility; credits are the unit.

**Roko in Cursor**: Not via ACP. Instead:
1. **MCP**: Roko as an MCP server, exposing gate pipeline and knowledge store as MCP tools to Cursor's native agent.
2. **Background agent target**: Roko as the cloud backend that Cursor's background agent delegates to.
3. **CLI tool**: Cursor's agent mode calls `roko run` as a terminal command.

### 13.3 Windsurf — No ACP, Deep Cascade Integration

Windsurf (Cognition, acquired Codeium Dec 2025) has no ACP support and is unlikely to add it — they have their own deeply integrated system:

- **Cascade**: Core agentic AI with tool calling, multi-file reasoning, repo-scale comprehension.
- **Agent Command Center** (Windsurf 2.0, April 2026): Kanban-style dashboard for all agent sessions.
- **Spaces**: Bundles of agent sessions, PRs, files, and shared context around tasks.
- **Cascade-to-Devin handoff**: One-click delegation to Devin's cloud VM with desktop + browser.
- **MCP**: Natively integrated, 21+ third-party tools, one-click setup.

**Roko in Windsurf**: MCP only. Roko's gate pipeline and knowledge store as MCP tools.

### 13.4 JetBrains — First-Class ACP + Built-in MCP Server

JetBrains is the best ACP deployment target outside Zed:

- **Native ACP since 2025.3**: All IDEs (IntelliJ IDEA, PyCharm, WebStorm, GoLand, RustRover, CLion, etc.).
- **ACP Agent Registry integration**: One-click install from inside the IDE. Jointly maintained with Zed.
- **Built-in MCP server**: When `use_idea_mcp: true`, external agents get access to IntelliJ's refactoring engine, code analysis, test runners, and language-specific analysis via PSI (Program Structure Interface).
- **Junie**: JetBrains' own agent with Code mode (read+write plan execution) and Ask mode (read-only). Runs alongside ACP agents — not exclusive.
- **Multi-agent**: Users can work with Junie, Claude Agent, Codex, Gemini CLI, and any ACP agent simultaneously with their own subscriptions.
- **Cloud credits**: AI widget on toolbar shows progress bar for remaining quota. Computed from token counts dynamically.

**What Roko gets from JetBrains' MCP server**: Refactoring tools, code analysis, test running, and PSI-powered language intelligence. This means Roko can use IntelliJ's own refactoring engine in addition to its gate pipeline. Example: Roko asks IntelliJ to "extract method" via MCP, then runs its own compile+test gate to verify.

### 13.5 Zed — The ACP Birthplace

Zed provides the most complete ACP support as the protocol's creator:

- **Agent Panel**: Rebuilt in 2025. Multibuffer review + agent following.
- **Three built-in profiles**: Write (file editing + terminal), Ask (read-only), Minimal (no tools).
- **Custom profiles**: User-created tool groupings.
- **External agents**: Claude Agent, Gemini CLI, Codex, and any ACP agent run inside Zed.
- **Token-based billing**: Dashboard at `dashboard.zed.dev/account`. In-editor token counter near profile selector. Maximum Token Spend input on account page. Zed Pro includes $5/month credit; provider markup is list price + 10%.
- **MCP**: Multiple servers run as separate child processes. Tool key format: `mcp:<server_name>:<tool_name>`.

**What Roko gets from Zed**: The best ACP UX. Zed renders plan entries as visual checklists, tool calls as collapsible cards, diffs as inline hunks, and terminal output in its terminal panel. Zed's multibuffer review view shows all changes Roko proposes across files in a single view.

### 13.6 Neovim — Plugin-Driven ACP

Neovim's AI integration is entirely plugin-driven. Three relevant plugins:

- **avante.nvim** (17K+ stars): Cursor-style AI. Select code, `<leader>aa`, get sidebar diff. Per-project `avante.md` files. MCP via MCPHub.nvim.
- **codecompanion.nvim** (6K+ stars): Buffer-integrated AI. Slash commands, `@lsp` and `@buffers` context variables. MCP integration.
- **agentic.nvim**: ACP-compatible chat interface supporting Claude Code, Gemini, Codex, OpenCode, Cursor Agent.

**Roko in Neovim**: Via agentic.nvim (ACP) or as an MCP server to avante/codecompanion. ACP provides the full Roko experience (modes, slash commands, gates, plans). MCP provides tool-level access only.

### 13.7 Claude Code CLI — Peer, Not Competitor

Claude Code CLI is Anthropic's terminal-based agentic assistant. It has a rich architecture:

- **Subagent system**: Explore (Haiku, read-only), Plan (read-only research), General-purpose (all tools). Custom subagents via `.claude/agents/` markdown files with YAML frontmatter.
- **Agent Teams** (experimental): One session as team lead coordinating teammates in separate context windows.
- **Remote Control**: Run Claude Code on server/CI without local terminal.
- **MCP integration**: Native, primary external tool path.
- **Token tracking**: `/usage` + `/cost` commands with per-model breakdown and cache hit rates.
- **No codebase pre-indexing**: Uses agentic tool use (search, read) on demand. Amazon Science paper (Feb 2026): keyword search via agentic tool use achieves >90% of RAG-level performance without vector DB.

**Roko ↔ Claude Code relationship**:
1. **Roko as an ACP agent, Claude Code as a tool**: User in Zed/JetBrains uses Roko via ACP. Roko dispatches to Claude Code CLI as a backend agent (already wired via `roko-agent::ClaudeCliAgent`).
2. **Roko as an MCP server, Claude Code as the frontend**: User in terminal uses Claude Code. Roko's gate pipeline and knowledge store are available as MCP tools.
3. **Roko as a Claude Code custom subagent**: Define `roko.md` in `.claude/agents/` that invokes `roko run` as a tool.

These are complementary, not competitive. Roko adds gate verification and persistent knowledge to Claude Code's strong general-purpose agent capabilities.

---

## 14. Workspace Awareness — How Editors Provide Context

Each editor provides different context to agents. Roko must adapt to what's available:

| Editor | Codebase Indexing | Context Provided via ACP | What Roko Does Differently |
|---|---|---|---|
| **Zed** | File tree + MCP tools | Open buffers, file tree, terminal, MCP tools | Uses Roko's own `roko-index` for AST + HDC semantic search |
| **JetBrains** | Deep (PSI/AST analysis) | JetBrains MCP server exposes refactoring, analysis, diagnostics | Merges JetBrains PSI with Roko's HDC knowledge store |
| **VS Code** | Basic (file tree + diagnostics) | MCP servers, open editors, terminal, diagnostics | Roko's agentic search fills the indexing gap |
| **Cursor** | Semantic (AST + embeddings + Turbopuffer) | N/A (no ACP client) | MCP integration only |
| **Windsurf** | Codemap graph (automated structure graph) | N/A (no ACP) | MCP integration only |
| **Neovim** | LSP + plugin-dependent | LSP diagnostics via plugins, file buffers | Roko's on-demand search matches Claude Code's approach |
| **Claude Code CLI** | None (on-demand agentic search) | N/A (terminal, not editor) | Same philosophy — search at need, not pre-index |

### 14.1 Roko's Context Strategy in ACP Mode

When running as an ACP agent, Roko assembles context through three channels:

1. **ACP-provided context**: Files attached to prompts (`prompt[].resource`), open editor tabs (if signaled by client).
2. **MCP-provided context**: Editor's MCP servers (JetBrains PSI, VS Code diagnostics, user-configured tools).
3. **Roko-native context**: Knowledge store queries (HDC similarity search), `roko-index` code intelligence, episode history.

The VCG auction ([doc-07](07-AGENT-RUNTIME.md), CognitiveWorkspace) combines all three sources under a token budget constraint. Context bidders include:

- **TaskBidder**: Bids for task-relevant code (current file, imports, test files).
- **NeuroBidder**: Bids for knowledge store entries relevant to the prompt.
- **ResearchBidder**: Bids for research artifacts from `.roko/research/`.
- **EditorBidder** (new for ACP): Bids for open editor tabs and recent edits.
- **McpBidder**: Bids for MCP tool descriptions relevant to the task.

---

## 15. Multi-Agent Coordination via ACP + A2A

### 15.1 Single-Editor, Multi-Agent

ACP's Session Config Options (§2.2) enable Roko to operate as multiple specialized agents within a single ACP session. The user changes the "Mode" dropdown in the editor's settings panel:

```
User selects "Plan" in Mode dropdown  → Roko activates Strategist role
User selects "Code" in Mode dropdown  → Roko activates Implementer role
User selects "Review" in Mode dropdown → Roko activates Reviewer role
User selects "Research" in Mode dropdown → Roko activates Researcher role
User selects "Autonomous" in Mode dropdown → Roko activates all roles + conductor watchers
```

Each config change triggers a `session/config/update` → Roko updates the system prompt (via `RoleSystemPromptSpec`), the active tools (via Extension chain), the gate configuration, and pushes dependent config option updates back to the editor (e.g., Research mode disables gate pipeline toggle, Autonomous mode enables conductor watcher toggles). The user perceives one agent with multiple personalities. Roko's internal architecture treats each as a distinct cognitive posture.

### 15.2 Cross-Agent via A2A

ACP prompts can trigger A2A delegation. A user in JetBrains via ACP asks: "Audit this contract using the security agent."

Flow:
1. Roko receives prompt via ACP (`session/prompt`).
2. Roko discovers security agent via A2A (`/.well-known/agent-card.json`).
3. Roko creates an A2A Task with the contract code.
4. Remote security agent processes, sends `TaskStatusUpdateEvent` via A2A streaming.
5. Roko maps A2A task events → ACP `session/update` notifications.
6. User sees audit progress as plan steps in their editor.
7. A2A `TaskArtifactUpdateEvent` delivers the audit report.
8. Roko runs its own gate pipeline on the audit results.
9. Final result shown in editor with gate verification.

The user sees a seamless experience. They typed a prompt in their IDE and got back a gate-verified audit result, without knowing that two protocols and a remote agent were involved.

### 15.3 Editor-to-Editor via Relay

Roko's relay system ([doc-04 architecture](../architecture/04-connectivity.md)) enables cross-workspace coordination. Two developers in different editors (one in Zed, one in JetBrains) can coordinate through a shared Roko workspace:

1. Both editors run `roko acp` connected to the same `roko serve` instance.
2. Both see the same plan progress, gate results, and knowledge store.
3. One developer's code changes trigger gate re-runs visible to the other.
4. The relay broadcasts workspace events to both ACP sessions.

This bridges the `session/update` notifications from ACP (single-user, local) to the relay's WebSocket rooms (multi-user, networked).

---

## 16. Remote ACP — HTTP Transport (Emerging)

The ACP spec has work-in-progress HTTP/WebSocket transport for remote agents (not just local stdio). This unlocks:

1. **Cloud-hosted Roko**: `roko acp` runs on a server. The editor connects over WebSocket instead of stdio.
2. **Shared agent sessions**: Multiple editors connect to the same remote Roko session.
3. **Mobile/web clients**: ACP over HTTP means browser-based editors can use ACP agents.

Roko already has the infrastructure for this — `roko serve` on :6677 with ~85 REST routes + SSE + WebSocket. The bridge between `roko serve` and an HTTP-based ACP transport is small:

```
roko serve (HTTP :6677)      ←→    ACP HTTP transport    ←→    Editor (remote)
      ↕                                                            ↕
roko-orchestrator (L4)       ←→    ACP stdio transport   ←→    Editor (local)
```

Both transports feed into the same L4 orchestrator. The cognitive pipeline is transport-agnostic. This is the same architectural insight from section 3.4 — `roko-acp` is a presentation layer, not a new architectural layer.

**Implementation plan**: Wait for ACP HTTP transport spec to stabilize (expected Q3 2026), then add `bridge_http.rs` alongside `transport.rs`. Reuse `roko-serve`'s existing Axum/Tower infrastructure.

---

## 17. The Competitive Landscape — What Each Editor Offers Natively

For Roko to add value via ACP, it must understand what the editor already provides without an external agent. If the editor's built-in AI is "good enough" for a use case, Roko's ACP integration needs to be clearly better.

### 17.1 Feature Matrix (April 2026)

| Feature | VS Code (Copilot) | JetBrains (Junie) | Zed (Built-in) | Cursor | Windsurf (Cascade) | Neovim (plugins) | Roko (ACP) |
|---|---|---|---|---|---|---|---|
| **Agent mode** | GA (Copilot Agent) | GA (Junie) | Agent Panel | GA (Composer) | GA (Cascade) | Plugin-based | Full cognitive loop |
| **Multi-agent** | Claude+Codex+Copilot | Junie+Claude+Codex+Gemini | Via ACP/MCP | Subagent trees + cloud | Cascade+Devin | Via agentic.nvim | Single-session multi-role |
| **Cloud agents** | Codex cloud | No | No | Background agents GA | Devin handoff | No | Via `roko serve` |
| **Gate verification** | None | None | None | None | None | None | **11 gates, 7-rung pipeline** |
| **Persistent knowledge** | None | None | None | None | None | None | **HDC knowledge store** |
| **Multi-phase planning** | None (opaque) | Task decomposition (2 modes) | None | Plan → execute (linear) | None | None | **DAG with 6 phases** |
| **Token tracking** | Status bar + extensions | AI widget + quota | Dashboard + counter | In-editor dashboard | Credits only | None | **CostLens + VitalityTracker** |
| **MCP support** | GA (full spec) | GA (built-in MCP server) | GA | GA | GA | Via MCPHub.nvim | Already wired |
| **ACP support** | Community extension | Native (2025.3+) | Native (creator) | Agent-side only | None | Community plugin | This document |
| **Codebase indexing** | Basic (file tree) | Deep (PSI/AST) | File tree + MCP | Semantic (AST+embeddings) | Codemap graph | LSP + plugins | On-demand + HDC index |
| **Affect/behavioral model** | None | None | None | None | None | None | **Daimon PAD + vitality** |
| **Cost per decision** | None (aggregate only) | None (aggregate) | Per-session | Per-credit | None | None | **Per-cell per-graph per-agent** |

### 17.2 Where Roko Wins

The feature comparison reveals Roko's unique position across every editor:

1. **Gate verification**: Zero editors verify agent output with compilers and test harnesses. Every editor shows "the AI says this is right." Roko shows "the compiler says this is right."
2. **Persistent knowledge**: Zero editors have durable knowledge with confidence tracking. Sessions start fresh. Roko remembers what worked and what didn't.
3. **Multi-phase planning**: Cursor and Junie have basic planning. Neither has a DAG executor with parallel task execution, gate pipeline per task, and replanning on failure.
4. **Cost attribution at depth**: Zed and Claude Code come closest with token counters. Neither reports per-phase, per-gate, or per-model breakdowns.
5. **Behavioral modulation**: Zero editors have affect-driven strategy shifts. Roko's PAD-based behavioral phases are unique.

### 17.3 Where Editors Win (and Roko Should Not Compete)

1. **IDE-native refactoring**: JetBrains PSI-powered refactoring is better than any LLM at rename, extract method, move class. Roko should USE this (via JetBrains MCP server), not replace it.
2. **Semantic indexing**: Cursor's AST+embedding pipeline with Turbopuffer provides better retrieval than any on-demand search. Roko's `roko-index` should complement, not compete.
3. **Cloud agents**: Cursor's background agents and Windsurf's Devin handoff provide VM-based isolation that local ACP agents can't match. Roko's `roko serve` provides a different (not better) deployment model.
4. **Real-time collaboration**: Cursor's multi-agent sessions and VS Code's multi-agent development provide live coordination UX that ACP's single-session model doesn't support (yet — HTTP transport may change this).

---

## 18. Agent Communication Patterns — Beyond ACP

### 18.1 Emerging Standards Roko Should Track

| Standard | Status | Relevance to Roko |
|---|---|---|
| **LSAP** (Language Server Agent Protocol) | Early stage, [github.com/lsp-client/LSAP](https://github.com/lsp-client/LSAP) | Transforms low-level LSP capabilities into agent-native cognitive tools. If adopted, Roko could consume LSP data more efficiently. |
| **NIST AI Agent Standards Initiative** | Launched Feb 2026 | Voluntary guidelines for agent identity, authorization, security, monitoring. AI Agent Interoperability Profile planned Q4 2026. Roko should align with this as it crystallizes. |
| **Agent Protocol** (agent-protocol.ai) | Active | Open standard for agent invocation. Roko already exposes similar REST APIs via `roko serve`. |
| **ANP** (Agent Network Protocol) | Early stage | Peer-to-peer protocol. Relevant if Roko agents form decentralized swarms. |

### 18.2 Framework Interop

Roko's ACP integration means Roko can be used alongside any framework's agents:

| Framework | How It Connects to Roko |
|---|---|
| **OpenAI Agents SDK** | Agents as Tools pattern. Roko as a specialized tool in an OpenAI agent pipeline. |
| **LangGraph** | Roko as a node in a LangGraph graph. LangGraph manages orchestration, Roko provides gate verification. |
| **CrewAI** | Roko as a crew member. CrewAI manages role assignment, Roko provides knowledge store. |
| **AutoGen** | Roko as a conversational agent in AutoGen's multi-agent dialogue. |

In all cases, MCP is the integration layer. ACP is the editor layer. These are orthogonal.

---

## 19. References

### ACP (Agent Client Protocol)
- ACP Specification: https://agentclientprotocol.com/protocol/overview
- ACP Rust SDK: https://crates.io/crates/agent-client-protocol
- ACP GitHub: https://github.com/agentclientprotocol/agent-client-protocol
- ACP Registry: https://agentclientprotocol.com/get-started/registry
- ACP Session Usage RFD: https://agentclientprotocol.com/rfds/session-usage
- ACP Agents List: https://agentclientprotocol.com/get-started/agents
- ACP Overview (PromptLayer): https://blog.promptlayer.com/agent-client-protocol-the-lsp-for-ai-coding-agents/
- ACP Overview (Philipp Schmid): https://www.philschmid.de/acp-overview
- ACP Community Provider (Vercel AI SDK): https://ai-sdk.dev/providers/community-providers/acp

### Editor Integration
- JetBrains ACP Documentation: https://www.jetbrains.com/help/ai-assistant/acp.html
- JetBrains ACP Registry Blog: https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/
- JetBrains ACP Announcement: https://blog.jetbrains.com/ai/2025/10/jetbrains-zed-open-interoperability-for-ai-coding-agents-in-your-ide/
- Zed ACP Page: https://zed.dev/acp
- Zed Bring Your Own Agent: https://zed.dev/blog/bring-your-own-agent-to-zed
- Zed ACP Registry Launch: https://zed.dev/blog/acp-registry
- Zed Plans and Usage: https://zed.dev/docs/ai/plans-and-usage
- VS Code ACP Extension: https://github.com/formulahendry/vscode-acp
- VS Code ACP Feature Request: https://github.com/microsoft/vscode/issues/265496
- VS Code Multi-Agent Development: https://code.visualstudio.com/blogs/2026/02/05/multi-agent-development
- VS Code MCP Support: https://code.visualstudio.com/blogs/2025/06/12/full-mcp-spec-support
- CodeCompanion.nvim ACP: https://codecompanion.olimorris.dev/usage/acp-protocol
- agentic.nvim: https://github.com/carlos-algms/agentic.nvim
- Emacs ACP: https://github.com/emacsmirror/acp
- Kiro ACP Implementation: https://kiro.dev/docs/cli/acp/
- OpenCode ACP Implementation: https://open-code.ai/en/docs/acp

### Agent Implementations
- Cursor 3: https://cursor.com/blog/cursor-3
- Cursor Subagents: https://cursor.com/docs/subagents
- Cursor Cloud Agents: https://cursor.com/blog/cloud-agents
- Cursor Marketplace: https://cursor.com/marketplace
- Windsurf 2.0: https://cognition.ai/blog/windsurf
- Windsurf Cascade MCP: https://docs.windsurf.com/windsurf/cascade/mcp
- Goose ACP: https://goose-docs.ai/blog/2025/10/24/intro-to-agent-client-protocol-acp/
- Claude Code Overview: https://code.claude.com/docs/en/overview
- Claude Code Subagents: https://code.claude.com/docs/en/sub-agents
- Claude Code Costs: https://code.claude.com/docs/en/costs

### Protocols and Standards
- A2A Protocol: https://a2a-protocol.org/latest/
- A2A GitHub: https://github.com/a2aproject/A2A
- IBM Agent Communication Protocol: https://agentcommunicationprotocol.dev/introduction/welcome
- IBM ACP / A2A Merger: https://lfaidata.foundation/communityblog/2025/08/29/acp-joins-forces-with-a2a-under-the-linux-foundations-lf-ai-data/
- AGNTCY Agent Connect Protocol: https://spec.acp.agntcy.org/
- Agentic Commerce Protocol: https://www.agenticcommerce.dev/
- LSAP: https://github.com/lsp-client/LSAP
- NIST AI Agent Standards: https://www.nist.gov/caisi/ai-agent-standards-initiative
- MCP in 2026: https://dev.to/pooyagolchian/mcp-in-2026-the-protocol-that-replaced-every-ai-tool-integration-1ipc

### Comparisons and Surveys
- Protocol Survey (arXiv): https://arxiv.org/html/2505.02279v1
- ACP vs MCP vs A2A (Morph): https://www.morphllm.com/comparisons/acp-vs-mcp-vs-a2a
- MCP, A2A, ACP Explained (Boomi): https://boomi.com/blog/what-is-mcp-acp-a2a/
- AI Agent Protocols 2026 Guide: https://www.ruh.ai/blogs/ai-agent-protocols-2026-complete-guide
- How Cursor Indexes Codebases: https://towardsdatascience.com/how-cursor-actually-indexes-your-codebase/
- Claude Code No Indexing: https://vadim.blog/claude-code-no-indexing
