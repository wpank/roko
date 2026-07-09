# C — Tool Loop

Refresh target: `docs/02-agents/07-tool-loop.md`, `09-format-translation.md`

Verdict: `rewrite`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| `ToolLoop` | Shipping | real multi-turn loop in `crates/roko-agent/src/tool_loop/mod.rs` |
| `ToolDispatcher` | Shipping | real dispatcher at `crates/roko-agent/src/dispatcher/mod.rs` |
| `SafetyLayer` | Shipping | active part of the routed tool pipeline |
| Built-in tools | Shipping | `roko-std` currently exports a 16-tool built-in registry |
| Shared-path coverage | Partial | several provider families use the shared loop, but not every backend family uses the same factory path |

---

## What Is Definitely True

### The tool runtime is real

The refreshed parity copy should say this plainly:

- `LlmBackend` is live at `crates/roko-agent/src/tool_loop/mod.rs:61`
- `StopReason` is live at `crates/roko-agent/src/tool_loop/mod.rs:121`
- `ToolLoop` is live at `crates/roko-agent/src/tool_loop/mod.rs:164`
- `ToolDispatcher` is live at `crates/roko-agent/src/dispatcher/mod.rs:80`

The audit correction here is about scope, not existence.

### The built-in tool registry is live, but the count must match code

`roko-std` currently exposes:

- `TOOL_COUNT = 16` at `crates/roko-std/src/tool/builtin/mod.rs:41`
- `ROKO_BUILTIN_TOOLS` at `crates/roko-std/src/tool/builtin/mod.rs:47`

The parity material should follow the code and describe a shared built-in registry, not an inflated tool count.

### Multiple provider families already reach the tool loop

Evidence in the current tree:

- OpenAI-compatible shared backend: `OpenAiCompatLlmBackend`
- Anthropic tool-capable path: `provider/anthropic_api/tool_loop.rs`
- Gemini native tool-capable path: `tool_loop/backends/gemini_native.rs`
- Perplexity tool loop: `perplexity/tool_loop.rs`

This is enough to say the tool loop is wired.

---

## What Still Needs Narrow Wording

### It is not one universal path yet

Do not overclaim that every backend family is on a single shared backend factory.

More accurate wording:

- the shared tool runtime exists
- several HTTP-backed families reach it today
- some backend families still own dedicated execution paths

That distinction matters because `create_tool_loop_backend()` in `tool_loop/backends/mod.rs` is narrower than the full set of tool-capable provider paths in the crate.

### Research add-ons should be deferred

Keep the existing tool-runtime explanation.

Defer:

- tool-RAG
- speculative execution
- large reasoning-strategy taxonomies
- benchmark promises that do not map to current code

Those are not needed to explain what is already shipped.

---

## Recommended Refresh Language

- Keep: the loop, dispatcher, safety, translation, pruning, compaction, and checkpointing sections.
- Rewrite: any sentence that implies the missing work is “build the tool loop.”
- Rewrite: tool-count claims so they match `roko-std`.
- Narrow: backend coverage claims to “wired but not totally uniform.”

---

## Verification Anchors

```bash
rg -n "pub trait LlmBackend|pub enum StopReason|pub struct ToolLoop" crates/roko-agent/src/tool_loop/mod.rs
rg -n "pub struct ToolDispatcher" crates/roko-agent/src/dispatcher/mod.rs
rg -n "pub const TOOL_COUNT|pub static ROKO_BUILTIN_TOOLS" crates/roko-std/src/tool/builtin/mod.rs
rg -n "OpenAiCompatLlmBackend|AnthropicMessagesBackend|GeminiNativeBackend|PerplexityToolLoopAgent" crates/roko-agent/src
```
