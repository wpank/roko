# The Actual Fix — Mori Parity via Wiring

## Why Mori Works and Roko Doesn't

Mori's agent dispatch is ~300 lines of code that does exactly this:

```rust
// Mori: connection.rs:2445-2622
Command::new("claude")
    .arg("--print")
    .arg("--output-format").arg("stream-json")
    .arg("--model").arg(&model)
    .arg("--append-system-prompt").arg(&system_prompt)    // ← ROLE-SPECIFIC PROMPT
    .arg("--tools").arg(&role_tools)                      // ← TOOL WHITELIST
    .arg("--mcp-config").arg(&mcp_config)                 // ← MCP SERVER
    .arg("--resume").arg(&session_id)                     // ← CONVERSATION CONTINUITY
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
```

Then reads stdout line-by-line, parses stream-json, emits events. That's the whole thing.

**Roko's `dispatch_direct.rs` does this:**

```rust
// Roko: dispatch_direct.rs:141-165 (Claude CLI path)
Command::new("claude")
    .arg("--print")
    .arg("--output-format").arg("stream-json")
    // NOTHING ELSE. No --append-system-prompt. No --tools. No --mcp-config. No --resume.
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
```

**The delta is 4 CLI flags.** Everything else (prompt builders, tools, MCP, session persistence)
already exists in the codebase. It's just never passed to the subprocess.

For the Anthropic API and OpenAI-compat paths, the same problem exists in a different form:
no `system` field, no `tools` field, no conversation `messages` history.

---

## What Mori Gets From Claude CLI for Free

By passing `--resume <session_id>`:
- **Conversation history**: Claude CLI maintains it internally. Multi-turn just works.
- **Tool execution**: Claude CLI runs tools and returns results. No tool loop code needed.
- **Streaming**: stream-json output appears line-by-line as the model generates.

By passing `--append-system-prompt`:
- **Role-specific behavior**: The model knows its role, constraints, workspace context.

By passing `--tools`:
- **Principle of least privilege**: Each role gets only the tools it needs.

By passing `--mcp-config`:
- **External tools**: Code intelligence, custom tools available to the agent.

**Roko has all the infrastructure to generate these values.** It just doesn't pass them.

---

## The Fix: 4 Batches, ~25-30 Hours Total

### Batch 1: Wire the Claude CLI Path (4-6h)

**What**: Pass the missing flags to `dispatch_claude_cli()`.

```rust
// dispatch_direct.rs — BEFORE
Command::new("claude")
    .arg("--print")
    .arg("--output-format").arg("stream-json")

// dispatch_direct.rs — AFTER
Command::new("claude")
    .arg("--print")
    .arg("--output-format").arg("stream-json")
    .arg("--model").arg(&model)                             // from config
    .arg("--append-system-prompt").arg(&system_prompt)      // from PromptAssemblyService
    .arg("--tools").arg(&allowed_tools)                     // from role/config
    .arg("--mcp-config").arg(&mcp_config_path)             // from .roko/mcp-config.json
    .arg("--resume").arg(&session_id)                       // from previous turn's result
```

**Where the values come from** (all exist today):
- `model`: `roko.toml` → `[agent] model = "..."` (already read in unified.rs)
- `system_prompt`: `roko-compose/src/prompt_assembly_service.rs` (9-layer builder, works today)
- `allowed_tools`: `roko-agent/src/safety/contract.rs` (role-based tool restrictions, works today)
- `mcp_config_path`: `.roko/mcp-config.json` (auto-discovery, works today for plan runs)
- `session_id`: `DispatchResult.session_id` (already parsed from stream-json output)

**What changes in chat_inline.rs:**
- Store `session_id` from `DispatchResult` after each turn
- Pass it to next `dispatch_prompt()` call
- Build system prompt once at session start (call `PromptAssemblyService`)
- Re-build when `/system`, `/model`, `/mode` change

**Test**: `roko` → "what files are in this directory?" → model uses Read tool → works.
Next message: "now edit the README" → model remembers context → works (--resume).

---

### Batch 2: Wire the API Paths (4-6h)

**What**: For Anthropic API and OpenAI-compat, add system prompt, tools, and history.

```rust
// dispatch_direct.rs — Anthropic API path BEFORE
json!({
    "model": model,
    "max_tokens": 8192,
    "messages": [{"role": "user", "content": prompt}]
})

// AFTER
json!({
    "model": model,
    "max_tokens": 8192,
    "system": system_prompt,                              // ← ADD
    "tools": tool_definitions,                            // ← ADD
    "messages": conversation_history,                     // ← ADD (not just last message)
})
```

**For API paths, we must manage conversation history ourselves** (no --resume equivalent):
- Add `history: Vec<Message>` to `ChatSession` struct
- Append user message + assistant response each turn
- Send full history with each API call
- Implement context window management (drop oldest messages when approaching limit)

**For tools on API paths**:
- Tool definitions already exist in `roko-std/src/tool/builtin/` (19 tools)
- Need to format as Anthropic tool schema and include in request
- Need to handle `tool_use` stop reason → execute tool → send `tool_result` back

This is more work than the CLI path because Claude CLI handles tools internally,
but the API path requires an explicit tool loop.

---

### Batch 3: Streaming (6-8h)

**What**: Forward stream-json events to the UI incrementally instead of buffering.

**Claude CLI path** (easier):
- Currently: reads all stdout lines, buffers, returns complete text
- Fix: emit each `MessageDelta` to the UI as it arrives

```rust
// Instead of collecting all lines then returning:
while let Some(line) = lines.next_line().await? {
    let event: StreamEvent = serde_json::from_str(&line)?;
    match event {
        StreamEvent::Assistant { content } => {
            // Send delta to UI channel immediately
            streaming_tx.send(StreamChunk::Text(content)).await?;
        }
        StreamEvent::Tool { name, content } => {
            streaming_tx.send(StreamChunk::ToolOutput(name, content)).await?;
        }
        StreamEvent::Result { session_id, usage } => {
            // Final event — send completion
            streaming_tx.send(StreamChunk::Done(session_id, usage)).await?;
        }
    }
}
```

**In chat_inline.rs**: Replace `rx.try_recv() → full response` with streaming consumption:
- `StreamingState` already exists (`inline/primitives/streaming.rs`)
- Connect it to the streaming channel
- Render tokens as they arrive (the TUI rendering code exists)

**Anthropic API path** (harder):
- Switch to `"stream": true` in request body
- Parse SSE events (`event: content_block_delta`, `event: message_stop`)
- Forward deltas to same streaming channel

**OpenAI-compat path** (same as Anthropic but different SSE format):
- `"stream": true`
- Parse `data: {"choices":[{"delta":{"content":"..."}}]}`

---

### Batch 4: Security + Reliability (4-6h)

**Security** (same as all previous proposals):
- Auth enabled by default in roko-serve
- Terminal routes behind auth middleware
- CORS restricted to localhost
- Private gists, PTY session limits

**Reliability quick wins**:
- Shared `reqwest::Client` for API paths (not per-request)
- Timeout on Claude CLI subprocess (120s default)
- CancellationToken for Ctrl+C during dispatch
- Store session_id to disk for resume across roko restarts

---

## What This Gets You (vs Mori)

| Capability | Mori | Roko (after fix) | Status |
|---|---|---|---|
| Multi-turn conversation | `--resume` | `--resume` (Batch 1) | ✅ Parity |
| System prompt | `--append-system-prompt` | `--append-system-prompt` via PromptAssemblyService (Batch 1) | ✅ Parity |
| Tool execution | `--tools` + Claude CLI built-in | `--tools` (Batch 1), API tool loop (Batch 2) | ✅ Parity |
| MCP integration | `--mcp-config` | `--mcp-config` (Batch 1) | ✅ Parity |
| Streaming output | stream-json line parsing | stream-json forwarding (Batch 3) | ✅ Parity |
| Role-based tool restrictions | Per-role `--tools` list | From AgentContract (Batch 1) | ✅ Parity |
| Session persistence | session_id stored in state | session_id stored in ChatSession (Batch 1) | ✅ Parity |
| Secure deployment | N/A (local only) | Auth + CORS + terminal (Batch 4) | ✅ Better than mori |
| Learning/feedback | Basic logging | Full episode + CascadeRouter (existing, wire later) | Deferred |
| Knowledge store | None | 9-layer prompt includes anti-patterns (Batch 1) | ✅ Better than mori |

**Mori parity for interactive chat: ~20-25h of actual implementation.**

---

## What About the API/Gateway/Remote Stuff?

That's a **separate concern from mori parity**. Mori works because it correctly uses the
Claude CLI subprocess. Remote inference, shared gateways, caching layers — those are
optimizations and scaling features, not prerequisites for "it works."

**Priority order:**
1. Make it work (this doc) — 25-30h
2. Make it fast (shared HTTP client, streaming) — included in Batch 2-3
3. Make it scale (gateway crate, remote deployment) — future work
4. Make it learn (wire CascadeRouter, episodes, feedback) — future work

Don't build infrastructure before the basic thing works.

---

## Why My Previous Proposals Were Wrong

| Proposal | What it said | Why it was wrong |
|---|---|---|
| Solution A (Surgical) | Patch 7 independent batches | Treated symptoms, not the root cause |
| Solution B (Architectural) | Build InferenceGateway struct | Over-engineered a local-process problem |
| Solution C (Phased) | Grow dispatch_direct into gateway | Same direction, still over-engineered |
| Solution 1 (Service Triad) | Three independently deployable services | Massively over-scoped for "make chat work" |
| Solution 2 (Cell/Graph) | Build a platform engine | ~120h for what 4 CLI flags solve |
| Solution 3 (Hybrid) | Thin engine + cells | Still building infrastructure before product |

**The actual problem**: `dispatch_direct.rs` doesn't pass `--append-system-prompt`,
`--tools`, `--mcp-config`, or `--resume` to the claude subprocess. That's it. Everything
else (prompt builders, tool registries, MCP discovery, session persistence) exists and works
in other code paths. It just needs to be called from the chat path.

---

## After Mori Parity: What's Next?

Once chat works like mori (~25-30h), THEN consider:

1. **Gateway crate** — for teams sharing API keys, centralized caching/routing
2. **CascadeRouter wiring** — so model selection improves over time
3. **Episode recording** — so the system learns from every conversation
4. **Knowledge injection** — so the system gets smarter with use
5. **API path tool loop** — so non-CLI providers also support tools
6. **Plan execution improvements** — real-time feedback, streaming

But none of those are prerequisites for "roko works like mori." The CLI path with
4 flags gets you 90% of the way there.

---

## The Uncomfortable Truth

Roko has 177K LOC across 18 crates. Mori's agent dispatch is ~300 LOC in one function.
The reason mori works is not because it has better architecture — it's because someone
actually wired the CLI flags correctly. Roko's massive infrastructure is sophisticated
but disconnected. The fix is wiring, not building.

This is exactly what the CLAUDE.md says: **"WIRE, don't build."**
