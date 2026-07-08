# Prompt: 18-tools

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/18-tools/`. Covers tool architecture, 19 built-in tools, 423+ DeFi tools (as chain domain plugin), tool categories, profiles, wallets, testing, MCP servers (roko-mcp-github 17 tools, roko-mcp-slack 8 tools, roko-mcp-scripts), roko-plugin SDK, 16 agent templates, plugin loading mechanisms, service integrations.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §2–6 (tools per agent type), §8 Extensibility
2. `/Users/will/dev/nunchi/roko/refactoring-prd/10-developer-guide.md` §6 Plugin System (EventSource, MCP, FeedbackCollector), §7 Integration Patterns
3. `/Users/will/dev/nunchi/roko/refactoring-prd/06-interfaces.md` §1 roko new scaffolders

## Step 3 — SOURCE-INDEX entry `## 18-tools.md`

Key legacy:
- All of `bardo-backup/prd/07-tools/` (00-overview through 24-testing, plus IMPLEMENTATION-PLAN.md)
- All of `bardo-backup/prd/21-integrations/` (MetaMask, Venice, Bankr, AgentCash, Uniswap)
- `bardo-backup/tmp/mori-agents/14-service-integrations.md`, `15-automation-workflows.md`

## Step 4 — implementation-plans

- `07-mcp-tool-wiring.md` (superseded but has context)
- `11-agent-dogfooding.md` §Phase 2 (MCP servers) + §Phase 3 (16 agent templates)
- `11-sections/phase-2.md` — roko-mcp-github (17 tools), roko-mcp-slack (8 tools), roko-mcp-scripts (config-driven wrapper)
- `11-sections/phase-3-4.md` — 16 agent template full definitions

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/**/*.rs`
- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-*/src/**/*.rs` (if exists)

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/18-tools
```

Write **17 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-tool-architecture.md` | ToolDef, ToolContext, ToolResult, ToolExecutor registry. Full Rust types. Tool trait. |
| 01 | `01-19-built-in-tools.md` | List all 19 built-in tools in `roko-std`. For each: name, category, purpose, parameters, result type. |
| 02 | `02-tool-categories.md` | Categories: data, trading, LP, vault, lending, staking, restaking, derivatives, yield, safety, intelligence, memory, identity, wallet, streaming. Organized taxonomy. |
| 03 | `03-defi-tools-as-chain-plugin.md` | 423+ DeFi tools across Uniswap, Aave, Morpho, Pendle, Lido, EigenLayer, GMX. **Framed as chain domain plugin**, not core framework. Explicit: chain is one domain among many. |
| 04 | `04-tool-profiles-and-config.md` | Tool profiles. Per-profile tool allowlist. Configuration in roko.toml. Profile inheritance. |
| 05 | `05-wallet-management.md` | 3 custody modes: Delegation (enclave keys), Embedded (ERC-4337 account abstraction), Local key (dev). Cross-reference 08-chain.md §17. |
| 06 | `06-tool-testing-strategy.md` | Testing DeFi tools. Mirage-rs integration for safe testing. Mock vs. fork mode. Test fixtures. |
| 07 | `07-service-integrations.md` | MetaMask Delegation, Venice, Bankr, AgentCash, Uniswap, Slack, GitHub, Linear. Per-service integration pattern. |
| 08 | `08-mcp-integration.md` | Model Context Protocol. JSON-RPC client. Tool converter. Dynamic registry. `--mcp-config` flag. Auto-discovery fallback. |
| 09 | `09-roko-mcp-github.md` | 17 tools for PR review, issue triage, repo management. Per-tool schema. Auth via GitHub token. |
| 10 | `10-roko-mcp-slack.md` | 8 tools for message processing, notifications. Socket Mode vs. Web API. Auth via Slack bot token. |
| 11 | `11-roko-mcp-scripts.md` | Generic script wrapper. Config-driven (any language). Wraps 30+ Python automations. How to add a new script tool. |
| 12 | `12-roko-mcp-stdio.md` | Generic stdio MCP server scaffold. For building custom MCP servers. |
| 13 | `13-roko-plugin-sdk.md` | EventSource, FeedbackCollector, Integration traits. The developer interface for extending Roko. Full trait signatures. |
| 14 | `14-event-sources.md` | CronEventSource, FileWatchEventSource, GitHubEventSource, SlackEventSource, WebhookEventSource. Per-source implementation. |
| 15 | `15-16-agent-templates.md` | Full list of 16 agent templates from plan 11 Phase 3: doc-lifecycle, digest, meeting-sync, pr-review, triage, auto-plan, code-implementer, gate-fixer, action-tracker, pm-board, slack-notify, freshness, conflict-detector, enrich, pm-health, prd-ingestion. For each: repo, trigger, tools, full system prompt summary. |
| 16 | `16-plugin-loading-mechanisms.md` | 3 mechanisms from 10-developer-guide.md §6: (1) Cargo workspace members (compile-time, recommended for Rust plugins), (2) Config-declared plugins (runtime, for pre-built), (3) MCP tool discovery (runtime, any MCP-compatible tool server). Convention: `roko-domain-*` auto-discovered. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3500 total. Low citation count OK (this is implementation-focused).

Cross-reference 02-agents (MCP integration in agent runtime), 08-chain (DeFi tools as chain plugin), 12-interfaces (roko new scaffolders).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- **Chain tools are ONE set of tools, not the default framing.** The 19 built-in tools are general-purpose. DeFi tools are a chain domain plugin.
- 16 agent templates get full details — these are the starter kit for roko as a platform.
- Apply naming map: golem-tools → roko tools; golem → agent.
- Use Write tool. Don't ask questions.
