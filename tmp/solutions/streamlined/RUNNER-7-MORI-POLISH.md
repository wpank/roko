# Runner 7: `mori-polish` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 7 section.

---

## Runner Goal (one sentence)

Complete remaining Mori-like UX polish, slash commands, demo seeding, and deployment prep
after core contracts (Runners 1-6) are stable.

## Context Pack Files

```text
tmp/runners/mori-polish/
  README.md
  batches.toml
  context/
    00-RULES.md                     — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md        — single-owner map for this runner
    ANTI-PATTERNS.md                — forbidden patterns with repo examples
    ACCEPTANCE.md                   — proof commands including negative proofs
    FILE-OWNERSHIP.md               — batch → write path map
    ISSUE-MAP.md                    — batch → issue id map
    MORI-UX-REFERENCE.md           — Mori behaviors to match (Group 0 output)
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Mori-Polish Anti-Patterns

MP-1. **Polish does not bypass contracts.** This runner adds UX features ON TOP of the
      contracts from Runners 2-6. It does NOT add shortcuts that skip model selection,
      gate truth, telemetry, or security.

MP-2. **Demo data is labeled demo data.** Any seeded demo data must be clearly
      distinguishable from real runtime data. Use a `source: "seed"` or `demo: true`
      field in every seeded record.

MP-3. **API provider chat is not rushed.** If API provider tool loops are not ready from
      Runner 3's stub, this runner can implement them — but through existing provider
      adapters, NOT hand-rolled HTTP in CLI code.

MP-4. **Do not improve appearance without improving truth.** Making a page look better
      while showing inaccurate data is worse than showing ugly accurate data.
```

---

## Prerequisites

This runner MUST NOT start until Runners 1-6 have proof artifacts demonstrating:

- Runner 1: `npm run build` passes, API smoke passes, no fake success in demo
- Runner 2: `roko --model X` uses X, shell gates work, state agrees
- Runner 3: `roko` interactive chat works with tools, streaming, session continuity
- Runner 4: Generated PRDs are grounded, invalid plans are rejected
- Runner 5: Cost/usage is truthful, learning paths aligned, router gets real observations
- Runner 6: Default serve is safe, terminal gated, public bind requires auth

---

## Group 0: Contract Guardrails

### Z01 — Document Mori UX behaviors to match

**Type:** Context-only (no code changes)

**Goal:** Catalog remaining Mori behaviors not yet achieved by Runners 1-6.

**Write scope:**
- `tmp/runners/mori-polish/context/MORI-UX-REFERENCE.md`

**Read:**
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` (Mori agent session)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/cli/` (Mori CLI commands)
- CLAUDE.md (current roko CLI commands reference)
- Runner 3 completion proof (what's actually working)

**Required output:**
- Remaining Mori slash commands not yet in roko: `/tools`, `/mcp`, `/context`, `/history`
- Mori tool transcript rendering style
- Mori session history browsing
- Mori tab completion behavior
- Demo seeding patterns (what makes a demo workspace look alive)
- MCP mesh polish items

**DO NOT:** Change any source code.

---

## Group A: Extended Slash Commands

### A01 — `/tools` command: list available tools

**Goal:** User can see what tools the session allows.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (extend `handle_slash_command`)

**Required behavior:**
- `/tools` prints the current `allowed_tools_csv` as a formatted list
- Groups by category if categories are available (file, code, search, system)
- Shows enabled/disabled status
- Shows MCP-provided tools if MCP config is loaded

**DO NOT:**
- Change tool resolution logic
- Add tool management (enable/disable is for later)
- Query Claude CLI for tool lists at runtime

**Verify:** `cargo check -p roko-cli`

---

### A02 — `/mcp` command: show MCP config status

**Goal:** User can see MCP server configuration.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- `/mcp` prints: MCP config path, server names, connection status (if checkable)
- If no MCP config: `"No MCP servers configured. See 'roko config mcp' to add."`
- `/mcp reload` reloads config from disk

**DO NOT:**
- Implement MCP protocol parsing (just show config)
- Add MCP server management (create/delete)
- Make MCP required for chat to work

**Verify:** `cargo check -p roko-cli`

---

### A03 — `/context` command: show session context

**Goal:** User can inspect what context the session is carrying.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- `/context` prints:
  - Working directory
  - Effective model and provider
  - System prompt (first 200 chars + length)
  - Tool count
  - MCP config presence
  - Session id (if multi-turn)
  - API history length (for API providers)
- Useful for debugging "why did the model not see X?"

**DO NOT:**
- Print the full system prompt (too long)
- Print secret values (tokens, keys)
- Add context editing here (that's /system, /tools, etc.)

**Verify:** `cargo check -p roko-cli`

---

### A04 — `/history` command: show turn history

**Goal:** User can see conversation turn summaries.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- `/history` prints a numbered list of turns in the current session:
  ```
  1. [user] What files are here? (42 chars)
  2. [assistant] Here are the files... (1,847 chars, 3 tool calls)
  3. [user] Show me src/main.rs (23 chars)
  4. [assistant] The main file contains... (2,104 chars)
  ```
- Shows: role, first 50 chars of content, total length, tool call count
- If Claude CLI (no local history): print `"Session managed by Claude CLI. Use --resume for continuity."`
- If API provider: show from `api_history`

**DO NOT:**
- Store full turn history for Claude CLI mode (that's Claude's session)
- Implement history persistence across session restarts (later)
- Add history editing

**Verify:** `cargo check -p roko-cli`

---

## Group B: Tool Transcript Rendering

### B01 — Rich tool call display in terminal

**Goal:** Tool output looks like Mori: name, abbreviated input, result summary.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` OR new `crates/roko-cli/src/tool_render.rs`

**Required behavior:**
- Tool start: print `▸ [Read] src/main.rs` (tool name + key argument)
- Tool end (success): print `  ✓ 47 lines` (brief result)
- Tool end (error): print `  ✗ file not found` (error summary)
- For Bash tool: print the command (abbreviated if long)
- For search tools: print query + result count
- Colors: tool name in cyan, success in green, error in red (when terminal supports it)
- Non-color fallback: use ▸/✓/✗ as indicators

**DO NOT:**
- Print full tool input/output (abbreviate to one line each)
- Add animation or spinners (that's TUI territory)
- Require a specific terminal emulator

**Verify:** `cargo check -p roko-cli`

---

### B02 — Cost/token summary after each turn

**Goal:** User sees resource usage per turn.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` OR tool_render.rs

**Required behavior:**
- After each turn, print a status line to stderr:
  ```
  [claude-sonnet-4-6 via claude_cli | 1,247 in / 892 out | $0.03 | 4.2s]
  ```
- If usage is unknown: `[claude-sonnet-4-6 via claude_cli | usage: unknown | 4.2s]`
- If cost is zero: `[claude-sonnet-4-6 via claude_cli | 1,247 in / 892 out | free | 4.2s]`
- Configurable: `config.ui.show_usage_per_turn = true` (default true)
- Suppressible with `--quiet` flag

**DO NOT:**
- Print to stdout (interferes with piped output)
- Show fake/estimated values without marking them as estimated
- Make this verbose enough to distract from the response

**Verify:** `cargo check -p roko-cli`

---

## Group C: Demo Seeding

### C01 — `roko init --demo` seeds realistic data

**Goal:** A demo workspace immediately shows interesting data in dashboards.

**Write scope:**
- `crates/roko-cli/src/commands/init.rs` (add --demo flag)
- `crates/roko-cli/src/demo_seed.rs` (NEW FILE)

**Required behavior:**
- `roko init --demo` writes seeded data files:
  - `.roko/learn/efficiency.jsonl`: 10 sample events with real-looking usage
  - `.roko/memory/episodes.jsonl`: 5 sample episodes with varied models/outcomes
  - `.roko/learn/cascade-router.json`: router with 10+ observations
  - `.roko/neuro/knowledge.jsonl`: 8 sample knowledge entries
- ALL seeded records include `"source": "seed"` field
- Seeded data uses configured model names from the generated config
- Seeded agent profiles have capabilities and domain_tags

**DO NOT:**
- Make seeded data indistinguishable from real data (always mark as seed)
- Seed invalid/broken data that would fail Runner 2-5 validation
- Make `--demo` the default
- Seed data that references files not in the workspace

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 16.7, 13.1-13.3

---

### C02 — Dashboard renders seed data as "seed data" mode

**Goal:** When viewing seed data, the UI clearly labels it.

**Write scope:**
- `demo/demo-app/src/hooks/useApiWithFallback.ts` (or equivalent)
- `demo/demo-app/src/components/AppShell.tsx`

**Required behavior:**
- API responses with only `source: "seed"` entries: show "SEED DATA" indicator
- Indicator is less prominent than "DEMO" mode (seed data came from the real API)
- Mixed seed + real: show "LIVE" mode (real data present)
- Seed-only: show "SEED" mode badge

**DO NOT:**
- Remove seed data from display (it's useful for demos)
- Make seed and real data look identical
- Change the fallback policy from Runner 1

**Verify:** `cd demo/demo-app && npx tsc --noEmit`

---

## Group D: API Provider Chat (if not done in Runner 3)

### D01 — Route API providers through existing adapters

**Goal:** Non-Claude-CLI models work for chat (via Anthropic API, OpenAI-compat).

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (replace E01 stub from Runner 3)

**Read:**
- `crates/roko-agent/src/provider/anthropic_api.rs` (tool loop)
- `crates/roko-agent/src/provider/openai_compat.rs` (tool loop)
- `crates/roko-agent/src/model_call_service.rs`

**Required behavior:**
- If `model_selection.provider_kind == "anthropic_api"`:
  - Route through existing Anthropic adapter with system prompt + history
  - Include tool definitions from session
  - Handle tool use response loop
- If `model_selection.provider_kind == "openai_compat"`:
  - Route through existing OpenAI-compat adapter with system prompt + history
  - Include tool definitions from session
  - Handle tool use response loop
- Conversation history managed in `api_history`
- Streaming through provider if supported

**DO NOT:**
- Build raw HTTP requests in chat_session.rs
- Duplicate tool loop logic from providers
- Make API chat the default when Claude CLI is available
- Skip system prompt for API providers

**Verify:** `cargo check -p roko-cli`

---

### D02 — API provider streaming support

**Goal:** API responses stream where the provider supports it.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- For Anthropic API: use streaming endpoint, forward text deltas
- For OpenAI-compat: use streaming endpoint if available
- For non-streaming providers: buffer and display at end (with a "generating..." indicator)
- Same `StreamEvent` enum as Claude CLI path (from Runner 3 C01)

**DO NOT:**
- Implement SSE parsing from scratch — use existing provider adapter streaming
- Require streaming (fallback to non-streaming is OK)
- Make streaming the only way to get a response

**Verify:** `cargo check -p roko-cli`

---

## Group E: Deployment and Misc Polish

### E01 — Deploy workflow uses security posture

**Goal:** `roko deploy` won't create an insecure deployment.

**Write scope:**
- `crates/roko-cli/src/commands/deploy.rs` (if it exists)

**Required behavior:**
- Before deploying: check that auth is enabled in the deploy config
- Railway/Fly/Docker deploy templates include `auth_enabled = true` by default
- Deploy command prints security checklist:
  ```
  Pre-deploy checklist:
  ✓ Auth enabled
  ✓ Terminal routes disabled (or gated)
  ✓ CORS restricted to deploy domain
  ✗ Share scrubbing: not configured (recommend: enable)
  ```
- Refuse to deploy if `auth_enabled = false` without `--unsafe-public`

**DO NOT:**
- Implement new deployment backends
- Change Railway/Fly adapters significantly
- Block local development deploys

**Verify:** `cargo check -p roko-cli`

---

### E02 — Session history persistence (basic)

**Goal:** Users can review previous sessions.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- On session end: write a summary to `.roko/sessions/<timestamp>.json`:
  ```json
  {
    "started_at": "...",
    "ended_at": "...",
    "model": "claude-sonnet-4-6",
    "turns": 7,
    "total_tokens": 15847,
    "total_cost_usd": 0.23,
    "session_id": "...",
    "summary": "First turn: 'What files are here?' ... Last turn: 'Thanks!'"
  }
  ```
- `roko history` lists recent sessions (last 20)
- `roko history <id>` shows session detail
- Summaries only — full transcripts are not stored (Claude CLI owns the session)

**DO NOT:**
- Store full conversation transcripts (privacy, size)
- Make session persistence blocking
- Break Claude CLI session continuity (--resume still works)

**Verify:** `cargo check -p roko-cli`

---

### E03 — MCP server mesh polish

**Goal:** MCP server discovery and status is user-visible.

**Write scope:**
- `crates/roko-cli/src/commands/config_cmd.rs` (or new MCP subcommand)

**Required behavior:**
- `roko config mcp list`: lists configured MCP servers with status
- `roko config mcp test <name>`: basic connectivity test
- `roko config mcp add <path>`: adds an MCP config entry
- Uses existing MCP config format (don't invent a new one)

**DO NOT:**
- Implement MCP protocol handling (Claude CLI does that)
- Add MCP server lifecycle management (start/stop)
- Make MCP required for any command

**Verify:** `cargo check -p roko-cli`

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| 0: Contracts | 1 | Mori UX reference |
| A: Slash Commands | 4 | /tools, /mcp, /context, /history |
| B: Tool Rendering | 2 | rich display, usage summary |
| C: Demo Seeding | 2 | init --demo, seed mode badge |
| D: API Provider | 2 | route through adapters, streaming |
| E: Misc Polish | 3 | deploy checklist, session history, MCP polish |
| **Total** | **14** | |

## Suggested Execution Waves

Wave 1: Z01 (context-only)
Wave 2: A01, A02, A03, A04 (slash commands — all in same file, sequential)
Wave 3: B01, B02 (tool rendering — parallel if separate files)
Wave 4: C01 (demo seed)
Wave 5: C02 (dashboard seed mode badge)
Wave 6: D01, D02 (API provider chat — sequential)
Wave 7: E01, E02, E03 (misc polish — parallel)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- `/tools` shows available tools with categories
- `/mcp` shows MCP server config or "none configured"
- `/context` shows full session state summary
- `/history` shows turn summaries
- Tool calls render with name, input, and result summary
- Per-turn usage shows model, tokens, cost, time
- `roko init --demo` creates a workspace with seeded dashboard data
- Dashboard shows "SEED DATA" mode for seeded workspaces
- API provider chat works through existing adapters (if D01/D02 included)
- `roko history` lists recent sessions

**Negative proofs:**
- No slash command bypasses model selection contract
- No seeded data is indistinguishable from real data
- No API provider code is hand-rolled HTTP in CLI layer
- No demo seeding makes the workspace look live when it isn't
- Deploy without auth refuses to proceed
