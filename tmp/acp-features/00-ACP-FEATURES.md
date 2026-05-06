# Roko ACP Features Checklist

> Status of Roko as an ACP agent in Zed/Cursor, mapped against the workflow-v1 PRDs
> (`tmp/archive/workflow-v1/`) and the UX refresh context
> (`nunchi-dashboard/tmp/ux-refresh-context/`).

Legend: `[x]` done, `[~]` partial, `[ ]` not started

---

## 1. Core ACP Protocol

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` JSON-RPC 2.0 stdio transport | Done | `crates/roko-acp/src/transport.rs` — newline-delimited JSON-RPC |
| `[x]` `initialize` handshake | Done | Returns protocolVersion, agentInfo, agentCapabilities |
| `[x]` `session/new` | Done | Creates session with config options + slash commands |
| `[x]` `session/list` | Done | Lists all active sessions |
| `[x]` `session/load` | Done | Reloads a session by ID |
| `[x]` `session/prompt` | Done | Streams agent response via `session/update` notifications |
| `[x]` `session/cancel` | Done | Cooperative cancellation via CancelToken |
| `[x]` `session/config/update` | Done | Updates config options (model, effort, etc.) |
| `[x]` `session/set_mode` | Done | Switches between code/plan/research modes |
| `[x]` Parse error handling | Done | Malformed JSON → PARSE_ERROR response |
| `[x]` Unknown method handling | Done | → METHOD_NOT_FOUND response |
| `[x]` Session-not-found handling | Done | → SESSION_NOT_FOUND response |
| `[x]` Session-busy rejection | Done | → SESSION_BUSY error if prompt in-flight |
| `[x]` Integration test suite | Done | 8 protocol conformance tests |
| `[x]` Unit test suite | Done | 14 unit tests |

## 2. Provider System (Multi-Model)

> **Source**: Will's core requirement — "use the zai and kimi ones", not just Claude.
> **Ref**: `roko.toml` `[providers.*]` + `[models.*]`, PRD-08 CLI redesign (model routing)

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` Claude CLI dispatch | Done | `run_claude_cognitive_task` — spawns `claude --print --output-format stream-json` |
| `[x]` OpenAI-compat HTTP streaming | Done | `run_openai_compat_cognitive_task` — SSE via `reqwest`, parses `StreamChunk` |
| `[x]` Provider routing from roko.toml | Done | `resolve_model()` → `ProviderKind` → correct dispatch path |
| `[x]` 6 providers configured | Done | anthropic, openai, perplexity, moonshot, zhipu, gemini, ollama |
| `[x]` 22 model aliases | Done | haiku→opus, gpt41→codex-mini, kimi-k26→k2, glm51→glm4, gemini-flash/pro |
| `[x]` Extra headers passthrough | Done | `provider.extra_headers` injected into requests |
| `[~]` Cascade router integration | Partial | `routing.mode` config exposed; actual cascade routing not yet wired into ACP dispatch |
| `[ ]` Per-session model override learning | Not started | UX34: manual overrides don't feed back to cascade router |
| `[ ]` Perplexity API dispatch | Not started | Uses openai_compat path; could be specialized |

## 3. Config Options (Editor Status Bar)

> **Source**: Codex CLI comparison screenshots, UX refresh `doc-2a-will-workflow.md`
> **Ref**: PRD-04 (authoring tiers, macros), UX refresh (Effort & Escalation, Budget Limits)

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` Model selector | Done | Dynamic from `[models.*]` in roko.toml, shows provider |
| `[x]` Effort level | Done | low/medium/high/max |
| `[x]` Temperament | Done | conservative/balanced/aggressive/exploratory |
| `[x]` Routing mode | Done | auto/manual |
| `[x]` Clippy gate toggle | Done | on/off |
| `[x]` Test gate toggle | Done | on/off |
| `[ ]` Budget limit (per-session USD) | Not started | UX refresh shows per-task/plan/session sliders |
| `[ ]` Context limit (tokens) | Not started | `agent.context_limit_k` exists in roko.toml |
| `[ ]` Auto-escalate toggle | Not started | UX refresh shows auto-escalate checkbox in agent config |
| `[ ]` Max retries | Not started | Available in roko.toml `conductor.max_auto_fix_attempts` |
| `[ ]` Provider selector | Not started | Currently model implies provider; could expose directly |

## 4. Slash Commands

> **Source**: PRD-08 (CLI redesign — verb sugar), PRD-06 (workflow catalog), UX refresh
> (`doc-2a-will-workflow.md` core loop: Research → Synthesize → Specify → Implement → Verify → Feedback)

### 4a. Status & Diagnostics

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /status | Done | `roko status` | PRD-08: `roko status` |
| `[x]` /doctor | Done | `roko doctor` | PRD-08: `roko doctor` |
| `[x]` /config | Done | `roko config show` | PRD-08: `roko config show` |
| `[x]` /learn | Done | `roko learn all` | PRD-08: learning inspection |

### 4b. Research (Foraging Phase)

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /research <topic> | Done | `roko research topic` | PRD-06: `research-sweep` workflow |
| `[x]` /search <query> | Done | `roko research search` | PRD-08: verb sugar for quick search |
| `[x]` /enhance-prd <slug> | Done | `roko research enhance-prd` | PRD-06: `prd-enrich` workflow |
| `[ ]` /analyze | Not started | `roko research analyze` | PRD-06: execution data analysis |

### 4c. Specification (PRD Lifecycle)

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /prd-idea <text> | Done | `roko prd idea` | PRD-06: Tier 0 `prd-draft` |
| `[x]` /prd-draft <slug> | Done | `roko prd draft new` | PRD-06: Tier 0 |
| `[x]` /prd-list | Done | `roko prd list` | PRD-08: `roko prd list` |
| `[x]` /prd-status | Done | `roko prd status` | PRD-08 |
| `[x]` /prd-plan <slug> | Done | `roko prd plan` | PRD-06: Tier 0 `prd-plan` |
| `[x]` /prd-consolidate | Done | `roko prd consolidate` | PRD-08 |

### 4d. Planning

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /plan-list | Done | `roko plan list` | PRD-08 |
| `[x]` /plan-generate <desc> | Done | `roko plan generate` | PRD-08 |
| `[x]` /plan-validate [dir] | Done | `roko plan validate` | PRD-08 |
| `[x]` /plan-run [dir] | Done | `roko plan run` | PRD-06: Tier 0 `plan-execute` |
| `[ ]` /plan-show <name> | Not started | `roko plan show` | PRD-08 |
| `[ ]` /plan-resume <id> | Not started | `roko plan run --resume` | PRD-05 resumability |

### 4e. Implementation & Execution

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /run <prompt> | Done | `roko run` | PRD-08: main entry, universal loop |
| `[x]` /agents | Done | `roko agent list` | UX refresh: Fleet view |
| `[x]` /agent-chat <name> | Done | `roko agent chat` | UX refresh: Copilot panel |
| `[ ]` /agent-start <name> | Not started | `roko agent start` | UX refresh: Fleet management |
| `[ ]` /agent-stop <name> | Not started | `roko agent stop` | UX refresh: Fleet management |

### 4f. Verification & Gates

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /build | Done | `cargo build --workspace` | PRD-06: `build` workflow |
| `[x]` /test | Done | `cargo test --workspace` | PRD-06: `test-run` workflow |
| `[x]` /clippy | Done | `cargo clippy --workspace` | Gate pipeline |
| `[x]` /fmt | Done | `cargo +nightly fmt --check` | Gate pipeline |
| `[x]` /gate | Done | fmt + clippy + test (full pipeline) | PRD-06: gate pipeline concept |
| `[ ]` /review [pr\|paths] | Not started | Would map to `roko run code-review` | PRD-06: `code-review` workflow |
| `[ ]` /audit | Not started | Would map to `roko run security-audit` | PRD-06: `security-audit` workflow |

### 4g. Knowledge & Dreams

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /knowledge <topic> | Done | `roko knowledge query` | UX refresh: Knowledge Store |
| `[x]` /knowledge-stats | Done | `roko knowledge stats` | UX refresh: Knowledge stats |
| `[x]` /dream | Done | `roko knowledge dream run` | UX refresh: Dream cycles (NREM→REM→integration) |
| `[ ]` /knowledge-gc | Not started | `roko knowledge gc` | UX refresh: Knowledge decay |
| `[ ]` /knowledge-backup | Not started | `roko knowledge backup` | Genomic bottleneck backup |

### 4h. Code Intelligence

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /index [cmd] | Done | `roko index` | PRD-06: `index-build`, `code-search` |
| `[x]` /explain <topic> | Done | `roko explain` | PRD-08 |
| `[x]` /replay <hash> | Done | `roko replay` | PRD-05: episode replay |

### 4i. Feedback & Learning

| Command | Status | CLI mapping | Source doc |
|---------|--------|-------------|-----------|
| `[x]` /learn-router | Done | `roko learn router` | UX refresh: Cascade router state |
| `[x]` /learn-episodes | Done | `roko learn episodes` | UX refresh: Episode log |
| `[x]` /learn-tune [what] | Done | `roko learn tune` | Adaptive thresholds |
| `[x]` /help | Done | Inline listing | — |

**Summary: 35 slash commands total, 31 implemented, 4 deferred**

## 5. Session Update Streaming

> **Source**: ACP spec, PRD-05 (execution engine events)

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` agent_message_chunk | Done | Text content streaming |
| `[x]` agent_thought_chunk | Done | Reasoning/thinking content |
| `[x]` tool_call (in-progress) | Done | Claude CLI tool calls surfaced |
| `[x]` tool_call_update (completed) | Done | Tool results with content |
| `[x]` available_commands_update | Done | Sent after session/new |
| `[x]` Usage info in response | Done | input/output/thought/cached tokens |
| `[ ]` File change notifications | Not started | ACP supports file_change updates |
| `[ ]` Progress notifications | Not started | Could show plan execution progress |

## 6. Modes

> **Source**: Codex CLI comparison (code/plan/research modes), UX refresh

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` Code mode | Done | Default implementation mode |
| `[x]` Plan mode | Done | Planning before execution |
| `[x]` Research mode | Done | Context gathering and analysis |
| `[ ]` Mode-specific system prompts | Not started | Different SystemPromptBuilder templates per mode |
| `[ ]` Mode-specific tool restrictions | Not started | Research mode should be read-only |

## 7. Conversation History

> **Source**: UX refresh (`doc-3-optimal-redesign.md`), PRD-05 (resumability)

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` In-memory session state | Done | Sessions persist across prompts within a process |
| `[ ]` Conversation turn accumulation | Not started | Need to build message history for multi-turn context |
| `[ ]` Session persistence to disk | Not started | PRD-05 describes run snapshots in `.roko/runs/` |
| `[ ]` Session resume across restarts | Not started | `session/load` exists but no disk backing |

## 8. Context & Enrichment

> **Source**: PRD-02 (capabilities), PRD-05 (cascade router), UX refresh (Knowledge Store)

| Feature | Status | Notes |
|---------|--------|-------|
| `[x]` `includeContext` param accepted | Done | Parsed from session/prompt |
| `[ ]` File context injection | Not started | Editor sends open files; need to prepend to prompt |
| `[ ]` Knowledge-informed prompts | Not started | Query neuro store for relevant context before dispatch |
| `[ ]` SystemPromptBuilder integration | Not started | 9-layer prompt assembly exists in roko-compose |
| `[ ]` MCP server passthrough | Not started | `mcpServers` stored per session but not forwarded to agent |

## 9. Workflow-v1 Vision Features (Future)

> **Source**: PRD-00 through PRD-12, full workflow-v1 archive

These represent the long-term vision where roko commands become composable Workflows.
The current ACP implementation provides the **editor surface** for this vision.

### 9a. Workflow Engine Integration

| Feature | Status | How ACP facilitates |
|---------|--------|---------------------|
| `[ ]` `roko run <workflow>` from editor | Not started | /run already maps to universal loop; extend to named workflows |
| `[ ]` Workflow parameter UI | Not started | ACP ConfigOptions could expose workflow macros as dropdowns |
| `[ ]` Run progress streaming | Not started | session/update could stream node-by-node progress (PRD-05 events) |
| `[ ]` Human-in-loop from editor | Not started | ACP elicitation could handle HumanInput nodes (PRD-05) |
| `[ ]` Run cancellation | Done | session/cancel already wired; maps to CancellationToken |
| `[ ]` Run resumption | Not started | /plan-run --resume exists but not exposed via ACP |

### 9b. Trigger System (PRD-03)

| Feature | Status | How ACP facilitates |
|---------|--------|---------------------|
| `[ ]` FileWatch triggers | Not started | Editor file saves could trigger workflows |
| `[ ]` Webhook triggers | Not started | External events (GitHub PR, Slack) → workflow → editor update |
| `[ ]` Trigger status in editor | Not started | session/update could show active triggers |

### 9c. Marketplace (PRD-12)

| Feature | Status | How ACP facilitates |
|---------|--------|---------------------|
| `[ ]` Browse marketplace | Not started | /market-browse slash command |
| `[ ]` Install workflow | Not started | /market-install slash command |
| `[ ]` Publish workflow | Not started | /market-publish slash command |

### 9d. Visual Authoring (PRD-11)

| Feature | Status | How ACP facilitates |
|---------|--------|---------------------|
| `[ ]` Recipe view (linear steps) | Not started | Could render as structured markdown in agent responses |
| `[ ]` Graph view (DAG) | Not started | Requires custom ACP UI or dashboard link |
| `[ ]` Macro promotion | Not started | Would need ACP config option updates at runtime |

## 10. UX Refresh Dashboard Integration

> **Source**: `nunchi-dashboard/tmp/ux-refresh-context/`

| Feature | Status | How ACP facilitates |
|---------|--------|---------------------|
| `[ ]` Copilot overlay parity | Not started | ACP is the protocol equivalent of the dashboard copilot panel |
| `[ ]` Agent fleet monitoring | Partial | /agents lists agents; dashboard shows real-time fleet view |
| `[ ]` Knowledge resonance graph | Not started | /knowledge queries store; dashboard renders force-directed graph |
| `[ ]` C-Factor gauge | Not started | Could expose via /status or config option |
| `[ ]` Cost tracking | Not started | Usage info returned per prompt; no cumulative tracking yet |
| `[ ]` Episode replay in editor | Partial | /replay exists; dashboard has full replay viewer |

---

## Cursor Configuration

Add Roko as a custom ACP agent in Cursor's settings:

```json
{
  "agent.customAgents": [
    {
      "name": "Roko",
      "type": "acp",
      "command": "/Users/will/dev/nunchi/roko/roko/target/debug/roko-cli",
      "args": ["acp"],
      "env": {}
    }
  ]
}
```

Access via: **Cursor Settings (JSON)** → add the above block.

Cursor spawns `roko-cli acp` as a stdio subprocess. Model selector, config dropdowns,
and all 35 slash commands appear in Cursor's agent panel.

For Zed, the equivalent config is in `~/.config/zed/settings.json`:

```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "/Users/will/dev/nunchi/roko/roko/target/debug/roko-cli",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

---

## Architecture Summary

```
Editor (Zed/Cursor)
  │ stdio JSON-RPC 2.0
  ▼
roko-cli acp
  │ crates/roko-acp/src/handler.rs — dispatch loop
  │ crates/roko-acp/src/session.rs — session + config state
  │ crates/roko-acp/src/bridge_events.rs — cognitive dispatch + streaming
  │
  ├─── Provider dispatch (roko-agent)
  │    ├── Claude CLI: subprocess, stream-json parsing
  │    ├── OpenAI-compat: HTTP SSE via reqwest
  │    ├── Zhipu (GLM): OpenAI-compat path
  │    ├── Moonshot (Kimi): OpenAI-compat path
  │    ├── Gemini: OpenAI-compat path
  │    └── Ollama: OpenAI-compat path (local)
  │
  ├─── Slash commands → roko CLI subprocess
  │    └── 35 commands across 9 categories
  │
  └─── Shell commands (/build, /test, /clippy, /fmt, /gate)
       └── Direct cargo invocations
```

## File Inventory

| File | LOC | Role |
|------|-----|------|
| `crates/roko-acp/src/types.rs` | ~950 | ACP JSON-RPC type definitions |
| `crates/roko-acp/src/transport.rs` | ~390 | Stdio transport layer |
| `crates/roko-acp/src/handler.rs` | ~365 | Request/notification dispatch |
| `crates/roko-acp/src/session.rs` | ~800 | Session manager, config options, slash commands |
| `crates/roko-acp/src/bridge_events.rs` | ~1350 | Provider dispatch + event streaming |
| `crates/roko-acp/src/config.rs` | ~55 | AcpConfig with roko.toml loading |
| `crates/roko-acp/src/lib.rs` | ~25 | Module declarations |
| `crates/roko-acp/tests/protocol_conformance.rs` | ~394 | 8 integration tests |
