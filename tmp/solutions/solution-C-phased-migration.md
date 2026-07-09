# Solution C — Phased Migration (Pragmatic Middle Path)

**Philosophy**: Get visible wins fast (security + latency in week 1), then build the
InferenceGateway incrementally by growing the existing `dispatch_direct.rs` into it rather
than replacing it wholesale. Each phase ships a working binary with measurable improvement.

**Total estimate**: ~45-55 hours across 4 phases
**Risk**: Low-medium — never breaks what works, each phase is independently shippable.
**Upside**: Fastest time-to-visible-improvement. User sees results after Phase 1 (day 1-2).

---

## Why This Approach

Solution A patches symptoms but never fixes the root. Solution B is architecturally correct
but requires ~15h of work before anything visibly improves. Solution C front-loads the
cheap wins, then evolves toward Solution B's architecture incrementally.

The key trick: **grow `dispatch_direct.rs` into `InferenceGateway` in-place**, rather than
building a replacement from scratch. Each commit adds one capability (shared client, then
system prompt, then history, then tools, then streaming) and is independently testable.

---

## Phase 1: Immediate Wins (6-8h, Week 1)

Ship a binary that's secure, 2x faster, and honest about what slash commands do.
Every change is mechanical and low-risk.

### Batch 1a: Security (3h)
Identical to Solution A Batch 1 / Solution B Phase 0. See those docs for details.

### Batch 1b: Shared HTTP Client (2-3h)
- Create one `reqwest::Client` at `dispatch_direct.rs` module level (lazy_static or OnceCell)
- Thread it into `dispatch_anthropic_api()` and `dispatch_openai_compat()`
- Apply timeout config from provider settings
- Same client for streaming path in `openai_compat_backend.rs`

### Batch 1c: Slash Command Honesty (1-2h)
- `/system` → store AND warn "not yet sent to model" (honest about limitation)
- `/effort` → same honest warning
- `/gate`, `/config set` → same pattern
- `tune gates --dry-run` → remove the flag (it's always read-only)
- Fix Share.tsx endpoint (`/api/share/` → `/api/shared/`)
- Remove investor demo hack in `useServerHealth.ts`

**What the user sees**: Secure server, halved response latency, commands that don't lie.
**What's still broken**: No tools, no history, no streaming.

---

## Phase 2: Growing dispatch_direct into InferenceGateway (12-15h, Week 2)

Evolve the existing dispatch module into a session-scoped service. Each sub-step is one commit.

### Step 2a: Session struct (2h)
```rust
// dispatch_direct.rs → rename to inference_gateway.rs
pub struct InferenceSession {
    client: reqwest::Client,
    config: ProviderConfig,
    system_message: Option<String>,
    history: Vec<ChatMessage>,
}

impl InferenceSession {
    pub fn new(config: ProviderConfig) -> Self { ... }
    // existing dispatch fns become methods on this struct
}
```

Move the module-level shared client into the struct. All existing callers updated to
use `session.dispatch_anthropic_api()` instead of the free function.

### Step 2b: System prompt (2h)
- Build system prompt from `SystemPromptBuilder` at session start
- Include workspace path, git branch, directory listing
- `/system` now calls `session.set_system_prompt()` — and it actually works

### Step 2c: Conversation history (2-3h)
- Accumulate user messages and assistant responses in `session.history`
- Send history with each API call (with a context window limit)
- Handle context window overflow (truncate oldest, keep system prompt)

### Step 2d: Streaming (6-8h)
- Anthropic API: switch to `"stream": true`, parse SSE events
- OpenAI-compat: same
- Claude CLI: forward stream-json lines incrementally
- Connect to `StreamingState` in chat_inline.rs
- Tokens render as they arrive

**What the user sees**: Multi-turn conversation with context, real-time streaming,
system prompts that work.
**What's still missing**: Tools, knowledge store, MCP.

---

## Phase 3: Tools & Knowledge (8-10h, Week 3)

### Step 3a: Tool registry (4-5h)
- Add `tools: Vec<ToolDefinition>` to `InferenceSession`
- Register builtin tools (file_read, file_write, bash, grep, glob) at startup
- Include in API requests (both Anthropic and OpenAI format)
- Handle `tool_use` responses: dispatch to tool, feed result back as next message

### Step 3b: Knowledge & MCP (3-4h)
- Query knowledge store as a tool (not separate subsystem)
- Pass MCP config from roko.toml into session
- MCP tools registered alongside builtins

### Step 3c: `/run` and `/plan run` inline (1-2h)
- `/run "prompt"` → execute through the session (not a CLI hint)
- `/plan run` → trigger plan execution inline with progress

**What the user sees**: Full Claude-Code-like experience. Model can read files, run commands,
use tools, access knowledge, stream responses.

---

## Phase 4: Reliability & Cleanup (10-12h, Week 4)

### Batch 4a: Subprocess safety (3h)
- Auth detection timeout (3s)
- MCP stderr → log file
- Claude CLI dispatch timeout
- CancellationToken in chat
- Replace bare eprintln! with tracing

### Batch 4b: Learning wiring (3h)
- Persist LinUCB bandit weights
- Auto-trigger episode compaction
- Dream trigger inline at plan completion
- Wire MaxCostPerTurn to session cost tracker

### Batch 4c: Error handling (2h)
- Top-10 .ok() audit in orchestrate.rs
- Fail-fast on empty env vars
- parking_lot::Mutex swap
- Remove crate-level lint suppression

### Batch 4d: Code dedup (quick wins only) (2h)
- Delete legacy `chat.rs`
- Unify init paths
- Extract session summary into one function
- Delete `dispatch_direct.rs` if fully migrated to InferenceSession

---

## Comparison: What Each Phase Resolves

| Systemic # | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|
| S1 (thin pipe) | | history + sys prompt | tools + knowledge | |
| S2 (HTTP clients) | **Resolved** | | | |
| S3 (commands lie) | Honest warnings | /system works | /run inline | |
| S4 (error swallow) | | | | Top-10 fixed |
| S5 (security) | **Resolved** | | | |
| S6 (no streaming) | | **Resolved** | | |
| S7 (hardcoded) | | Config-driven | | |
| S8 (phantom) | | | | Wired |
| S9 (subprocess) | | | | **Resolved** |
| S10 (duplicate) | | | | Quick wins |
| S11 (mutex/unwrap) | | | | **Resolved** |

---

## Cross-Directory Impact

### After Phase 1 (week 1)
- **binary-issues**: S2 + S5 resolved (8 of 11 systemic problems still open)
- **mori-diffs**: GAP-07 (permissions) + GAP-11 (security) resolved
- **subsystem-audits**: Security findings closed
- **converge-runner**: ~8 of 55 issues resolved

### After Phase 2 (week 2)
- **binary-issues**: S1 mostly resolved, S6 resolved, S7 resolved (4 of 11 still open)
- **mori-diffs**: GAP-01, GAP-02, GAP-04, GAP-05, GAP-10 resolved
- **subsystem-audits**: Phase 4 (dispatch unification) resolved
- **converge-runner**: ~25 of 55 issues resolved

### After Phase 3 (week 3)
- **binary-issues**: S1 fully resolved, S3 mostly resolved (3 of 11 still open)
- **mori-diffs**: GAP-03, GAP-08 resolved (10 of 12 GAPs closed)
- **subsystem-audits**: Phase 5 partially resolved
- **converge-runner**: ~35 of 55 issues resolved

### After Phase 4 (week 4)
- **binary-issues**: 9 of 11 systemic problems resolved (S4 partial, S10 deferred)
- **mori-diffs**: 11 of 12 GAPs resolved (GAP-06 two-runtimes deferred)
- **subsystem-audits**: Phases 4-5 resolved, Phase 6 deferred
- **converge-runner**: ~40 of 55 issues resolved

---

## What's Deferred (Regardless of Approach)

These items are deferred across ALL three solutions:

| Item | Why | Effort | When |
|---|---|---|---|
| S10.1 — Merge two execution engines | 21K LOC, high regression risk | 10-15h | Dedicated sprint |
| Full .ok() audit (18+ sites) | Requires case-by-case judgment | 4-6h | Incremental |
| StateHub unification (TUI + serve) | Requires shared process model | 3-4h | After engine merge |
| Chain runtime integration | Phase 2+ (needs blockchain backend) | 20h+ | Later |
| Remaining converge-runner issues (~15) | Subsystem-specific, not systemic | 8-10h | Incremental |

---

## Recommendation

**Solution C is the recommended approach** because:

1. **Visible results in day 1-2** (security + 2x faster) vs day 5+ for Solution B
2. **Same architecture as Solution B** by week 3 (InferenceGateway, just grown incrementally)
3. **Lower risk** — each commit is testable, no big-bang replacement
4. **Fewer total hours** than Solution B (~45h vs ~55h) because growing in-place avoids
   the migration overhead of building a parallel system
5. **Every phase ships** — if you stop after Phase 2, you still have a dramatically better product

The main risk vs Solution B: the incremental growth can accumulate intermediate scaffolding
that needs cleanup. Phase 4d handles the worst of this, but the code won't be as clean as
a from-scratch InferenceGateway.

If code cleanliness matters more than time-to-impact, go with Solution B.
If security is the only immediate concern, go with Solution A (but plan for B or C soon after).
