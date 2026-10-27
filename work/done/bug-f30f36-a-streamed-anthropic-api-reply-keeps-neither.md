+++
id = "bug-f30f36"
kind = "bug"
title = "A streamed Anthropic API tool turn ran no tools: the collected stream kept no content blocks, so the tool loop found no tool call in a streamed reply and the try ended empty_response after one call, which left the Anthropic API unusable for agent work; its parser also dropped the reply's thinking (thinking_delta and signature_delta read as unknown), so a follow-up would have gone without the thinking block and signature the API requires"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "56077878e"
source = "round v7 roko-effort, found while building gap-0eb14e; the lead's ask (file p2 and fix)"
discovered_from = "agent:roko-effort (round v7, gap-0eb14e)"
anchors = ["crates/roko-agent/src/provider/anthropic_api/stream.rs::AnthropicStreamState", "crates/roko-agent/src/tool_loop/mod.rs::collect_stream_to_response", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::AnthropicTranslator"]
links = { depends_on = [], blocks = [], related = ["gap-0eb14e", "bug-0e0dd9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_streamed_anthropic_tool_turn_sends_its_thinking_back' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_streamed_anthropic_tool_turn_sends_its_thinking_back"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T05:22:19Z"
commit = "2d6510089"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-27T02:21:26Z"
forced = false
evidence = "Merged as 2d6510089 (branch 730ac57f6). Post-merge verify by the roster's rule, in roko-effort's worktree detached at 2d6510089 on its APFS target clone: the item's [[verify]] passes (a_streamed_anthropic_tool_turn_sends_its_thinking_back ok; log roko-effort/logs/postmerge-f30f36-unit-0e0dd9.log). The $0 proof, against a stand-in Messages API that enforces the API's rule: red on main (c7c25701b: empty_response, the ls call never dispatched), green on the branch, and the stand-in's 400 fires as the loop's error on a mutant that drops the thinking blocks (unit-f30f36-2.log). Lib suites green on the branch (acp 237, agent 2426, cli 4067, core 2049, learn 1411, serve 1087). Still owed: the live check against the real API (about a cent), which needs an Anthropic key."
+++

## Problem

roko's Anthropic Messages API backend (`kind = "anthropic_api"`) streams every tool-loop call of a try whose progress
is tracked (bug-3a3b0f). On that path:

1. **A streamed tool turn ran no tools.** This is the worst of the three defects.
   - `collect_stream_to_response` built an OpenAI-shaped response.
   - `AnthropicTranslator` reads a reply's tool calls, and the assistant turn it sends back, from the native `content`
     array. That response had no such array.
   - So the loop found no tool call in any streamed reply. With or without thinking, the try ended
     `empty_response` ("the model returned no text and no tool call") after its first call.
   - **The Anthropic API was unusable for agent work.** The red run below shows it.
2. **The stream parser dropped thinking.** `stream.rs::handle_content_block_delta` matched a thinking delta as
   `"type": "thinking"`. The API sends `thinking_delta` deltas, then one `signature_delta` just before the block's
   `content_block_stop` ([streaming thinking](https://platform.claude.com/docs/en/build-with-claude/thinking#streaming-thinking)).
   Both fell into "unknown delta, ignored". Thinking is on by default for a `supports_thinking` profile, so:
   - none of the reasoning showed live;
   - the stall watchdog saw silence while the model thought;
   - `first_token_ms` (gap-0eb14e) timed the first text or tool call.

   Three test fixtures used the wrong type too, which is why the tests passed.
3. **The thinking blocks wouldn't have gone back.** Had the loop found the tool call, the follow-up would have gone
   without the reply's thinking block and its signature. The API requires them within a tool-use turn ("when you
   return tool results, you must pass the thinking blocks from the assistant message back to the API, complete and
   unmodified", [preserving thinking
   blocks](https://platform.claude.com/docs/en/build-with-claude/thinking#preserving-thinking-blocks)), and answers
   400 without them.

The answered (non-streamed) path kept the native content, thinking blocks and signatures included, and wasn't
affected. This was latent this round: no run used the Anthropic API, for want of a key. The provider is also gated:
no default profile uses `anthropic_api`.

## Why it matters

Every Anthropic API run of agent work failed after its first call. Any claim that roko works across vendors excluded
the Anthropic API without saying so.

## Evidence (2026-10-27, $0)

The proof is a stand-in Messages API (`serve_thinking_tool_loop` in the `provider/anthropic_api/tool_loop.rs` tests),
which holds roko to the API's rule:

- it streams a reply in the API's own event shapes: a thinking block (`thinking_delta`s and one `signature_delta`),
  then a `tool_use` of `ls`;
- it answers 400 ("a final `assistant` message must start with a thinking block") to a follow-up whose last assistant
  turn doesn't start with that thinking block, its text and signature unchanged.

Results:

- **Red on main** (c7c25701b, main plus the test): the run ends `BackendError("empty_response: the model returned no
  text and no tool call (finish_reason=tool_calls)")`. One request went and the `ls` call was never dispatched
  (defect 1).
- **Green with the fix** (this branch): the reasoning streams and the `ls` call is dispatched. The follow-up's last
  assistant turn starts with the exact thinking block, and the run ends Stop with the answer.
- **The stand-in's guard fires.** A mutant of the fix that drops the thinking blocks from the reply ends with the
  stand-in's 400 as the loop's error: `BackendError("network error: ... http 400: ... a final assistant message must
  start with a thinking block.")`. The follow-up's last assistant turn there was the bare `tool_use`.
- In all three runs the test's tool policy refuses `ls`, and the refusal goes back as its `tool_result`. The defect is
  whether the call is dispatched at all, not what the tool returns.
- **Suites:** the targeted tests (62), fmt, clippy (6 crates), check --tests and the lib suites are green on the
  branch: acp 237, agent 2426, cli 4067, core 2049, learn 1411, serve 1087.

## Fix

- **The parser reads the docs' event shapes:**
  - a `thinking_delta` is a reasoning delta (an empty one, as `display: "omitted"` sends, only says the call is
    alive);
  - a `signature_delta` sets the block's signature;
  - a `redacted_thinking` block is kept as sent.
- **The reply's content blocks are kept.** `AnthropicStreamState` keeps them as the API sent them: thinking with its
  signature, redacted thinking, text, and tool use with its input. The turn's `Done` carries them
  (`StreamEvent::blocks`), and `collect_stream_to_response` keeps them as the response's `content`. The translator
  then reads a streamed reply as it reads an answered one: its tool calls run, and its turn goes back with the
  thinking blocks first and unchanged.
- **The fixtures:** the three wrong ones use the docs' shapes.

## Still owed

**The live check against the real API**, about a cent: one streamed tool turn with thinking on, then its follow-up.
It needs an Anthropic key, which is on Will's list. Until then the fix is proven only against the stand-in. bug-0e0dd9
needs the same key: Claude 4.7 and later refuse roko's manual thinking request outright.

## Not covered (noted)

- The thinking tokens (`usage.output_tokens_details.thinking_tokens`) don't reach `reasoning_tokens`.
- Preserved thinking's prefix check (enforced by default for accounts from 2026-08-31): a client-side edit of an
  earlier message, such as compaction, invalidates the thinking blocks after it.
