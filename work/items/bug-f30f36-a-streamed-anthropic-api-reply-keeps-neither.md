+++
id = "bug-f30f36"
kind = "bug"
title = "A streamed Anthropic API reply keeps neither its thinking nor its content blocks: the parser reads thinking deltas as type thinking where the API sends thinking_delta and signature_delta, and the collected stream has no content blocks, so the tool loop finds no tool call in it, and a follow-up would go without the thinking block and signature the API requires"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round v7 roko-effort, found while building gap-0eb14e; the lead's ask (file p2 and fix)"
discovered_from = "agent:roko-effort (round v7, gap-0eb14e)"
anchors = ["crates/roko-agent/src/provider/anthropic_api/stream.rs::AnthropicStreamState", "crates/roko-agent/src/tool_loop/mod.rs::collect_stream_to_response", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::AnthropicTranslator"]
links = { depends_on = [], blocks = [], related = ["gap-0eb14e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_streamed_anthropic_tool_turn_sends_its_thinking_back' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_streamed_anthropic_tool_turn_sends_its_thinking_back"
+++

## Problem

roko's Anthropic Messages API backend (`kind = "anthropic_api"`) streams every tool-loop call of a try whose progress
is tracked (bug-3a3b0f). That path has two defects. Both are latent: no run this round used the Anthropic API, for
want of a key.

1. **The stream parser drops thinking.** `stream.rs::handle_content_block_delta` matches a thinking delta as
   `"type": "thinking"`. The API sends `thinking_delta` deltas, then one `signature_delta` just before the block's
   `content_block_stop` ([streaming thinking](https://platform.claude.com/docs/en/build-with-claude/thinking#streaming-thinking)).
   Both fall into "unknown delta, ignored". Thinking is on by default for a `supports_thinking` profile, at the
   family budget, so roko streams none of the reasoning:
   - nothing shows live;
   - the stall watchdog sees silence while the model thinks;
   - `first_token_ms` (gap-0eb14e) times the first text or tool call.

   Three test fixtures use the wrong type too, which is why the tests pass.
2. **A streamed turn keeps no content blocks.** `collect_stream_to_response` builds an OpenAI-shaped response.
   `AnthropicTranslator` reads a reply's tool calls, and the assistant turn it sends back, from the native `content`
   array, which that response lacks.
   - So the loop finds no tool call in a streamed reply and takes it as the final answer: the tool never runs.
   - Had the loop found the call, the follow-up would have gone without the reply's thinking block and its
     signature. The API requires them within a tool-use turn ("when you return tool results, you must pass the
     thinking blocks from the assistant message back to the API, complete and unmodified", [preserving thinking
     blocks](https://platform.claude.com/docs/en/build-with-claude/thinking#preserving-thinking-blocks)). In manual
     mode the final assistant turn must start with a thinking block, and the API answers 400 when it doesn't.

The answered (non-streamed) path keeps the native content, thinking blocks and signatures included. It isn't affected.

## Why it matters

The first person to turn on the Anthropic API with tools gets tries that never run a tool. Once the tool calls are
found, every follow-up is refused until the thinking blocks go back.

## Where

- `crates/roko-agent/src/provider/anthropic_api/stream.rs`: `AnthropicStreamState` (block start, delta and stop,
  `message_delta`), and two of the wrong fixtures.
- `crates/roko-agent/src/tool_loop/mod.rs`: `StreamEvent`, `collect_stream_to_response`.
- `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs`: `AnthropicTranslator` (`parse_calls`,
  `render_assistant_message`) and the third wrong fixture.

## Plan

1. The parser reads the docs' event shapes:
   - a `thinking_delta` becomes a reasoning delta (an empty one, as `display: "omitted"` sends, only says the call
     is alive);
   - a `signature_delta` sets the block's signature;
   - a `redacted_thinking` block is kept as sent.
2. `AnthropicStreamState` keeps the reply's content blocks as the API sent them: thinking with its signature,
   redacted thinking, text, and tool use with its input. The turn's `Done` carries them (`StreamEvent::blocks`).
   `collect_stream_to_response` keeps them as the response's `content`. The translator then reads a streamed reply
   as it reads an answered one: its tool calls run, and its assistant turn goes back on the follow-up with the
   thinking blocks first and unchanged.
3. The three fixtures use the docs' shapes.

## Done when

- A stand-in Messages API holds roko to the rule: red on main, green with the fix. The stand-in streams thinking in
  the API's event shapes and answers 400 on a tool-use follow-up whose last assistant turn doesn't start with the
  prior thinking block, text and signature unchanged. This is the `[[verify]]` test.
- **Still owed:** the live check against the real API, about a cent. It needs an Anthropic key, which is on Will's
  list.

## Not covered (noted for the lead)

- Claude 4.7 and later reject `thinking.type: "enabled"` with a 400, and the 4.6 models deprecate it. roko sends it
  for every `supports_thinking` profile. Adaptive thinking (`type: "adaptive"` with `output_config.effort`) is a
  separate change.
- The thinking tokens (`usage.output_tokens_details.thinking_tokens`) don't reach `reasoning_tokens`.
- Preserved thinking's prefix check (enforced by default for accounts from 2026-08-31): a client-side edit of an
  earlier message, such as compaction, invalidates the thinking blocks after it.
