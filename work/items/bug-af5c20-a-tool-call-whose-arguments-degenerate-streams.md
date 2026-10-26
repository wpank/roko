+++
id = "bug-af5c20"
kind = "bug"
title = "A tool call whose arguments degenerate streams until the stall watchdog cancels the try: gpt-5.4-mini on the Responses API emitted an ls call with its own routing tokens (to=functions.ls) and 34k characters of whitespace for 300 s; nothing checks the argument stream, so the try dies as a stall and bills the whole stream"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round v7 roko-effort, gap-1ba0ac's follow-up A/B (p5-ship T1, reasoning-on arm, 2026-10-26), the lead's ask"
discovered_from = "agent:roko-effort (round v7, gap-1ba0ac A/B)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::guard_reasoning", "crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming", "crates/roko-agent/src/openai_responses.rs::ResponsesTurn"]
links = { depends_on = [], blocks = [], related = ["gap-1ba0ac", "gap-f3f950"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_malformed_tool_argument_stream_is_cut_and_asked_again' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_malformed_tool_argument_stream_is_cut_and_asked_again"
+++

## Problem

A model call can stream a tool call whose arguments never become JSON, and nothing in roko stops it until the stall
watchdog cancels the whole try.

- **The case:** round v7 roko-effort's gap-1ba0ac A/B, task p5-ship T1, the reasoning-on arm (roko-main-bc230d52e,
  gpt-5.4-mini on the Responses API at effort medium, 2026-10-26 20:34).
  - The second call reasoned briefly, then emitted `ls` with arguments that began
    `{"path":".","}ંડાવ to=functions.ls  大发分分彩result to=functions.ls  彩神争霸app to=functions.ls ్ద్వ is not valid? let's just do.{"`.
  - Then came about 34,000 characters of raw whitespace and newlines: 19,686 `response.function_call_arguments.delta`
    events over 300 s.
  - The stall watchdog doesn't count argument deltas as progress. At 300 s it cancelled the try as "agent stalled:
    no progress for 300s".
  - Left alone, the stream would have run to the call's `max_output_tokens` (32,768), about 8 minutes, billing all
    of it.
- **What the tokens mean:**
  - `to=functions.ls` is the model's own tool-call routing header. It leaked into the argument string, three times,
    as if the model had started further (parallel) calls inside the first.
  - "大发分分彩" and "彩神争霸app" are known glitch tokens, a sign of a degenerate sampling state.
- **The request was ordinary.** Every key, parameter and replayed item matches the 13 + 11 calls of the two other
  on-arm runs that streamed cleanly:
  - instructions 35k characters, about 12k input tokens;
  - 14 tools, all `strict: false`, no `parallel_tool_calls` (so OpenAI's default, true);
  - one replayed reasoning item (2.8k characters encrypted, `content` and `summary` empty);
  - two parallel `read_file` calls whose outputs were "cannot stat … No such file" errors (the task creates those
    files).
- **Rate so far:** 1 of about 26 streamed task calls in three on-arm tries. None in the off arms (chat completions,
  no reasoning, about 112 calls).

## Why it matters

- Phase 5.1's resume and the next phase run on the Responses path (gap-1ba0ac). One glitched call costs a try:
  about 300 s and its output tokens, and the failure is misreported as a stall.
- The cause isn't confirmed. The detection and retry below don't depend on it.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs`:
  - `send_turn_streaming` and `guard_reasoning`, gap-f3f950's stream guard, which stops a reasoning-only reply at
    3/8 of its output limit;
  - the turn loop's handling of `REASONING_LIMIT_FINISH`, which asks again within the run, at most
    `LENGTH_CUT_RETRIES` times in a row.
- `crates/roko-agent/src/openai_responses.rs::ResponsesTurn` and the chat-completions `SseTurn`: argument deltas
  arrive as `StreamEventKind::ToolCallDelta` on both APIs.
- `crates/roko-agent/src/openai_responses.rs::request_body`: `strict: false` on every tool, and no
  `parallel_tool_calls` unless a provider disables them.

## Plan

1. **Detect.** A stream guard beside `guard_reasoning` validates each tool call's argument text as it streams.
   Anything outside the JSON grammar stops the call:
   - a raw control character inside a JSON string (JSON forbids unescaped U+0000–U+001F, and models escape newlines
     in real arguments);
   - non-whitespace after the top-level value closes;
   - a first non-whitespace character that isn't `{`;
   - a whitespace run of more than 2 KB, or arguments past a size cap (256 KB), as backstops.
   - **As built:** the recording's whitespace falls outside strings, where JSON allows it. Its key string
     `}ંડાવ … do.{` closed, and the whitespace that followed is legal between tokens. So no control character
     trips, and the 2 KB backstop alone would cut it only after about 2,200 characters. The guard therefore takes a
     whitespace run of more than 256 characters outside strings as malformed: JSON a model writes, pretty-printed or
     not, never needs a tenth of that. That cuts the recording at character 381. Inside a string, a run of spaces
     still gets 2 KB.
2. **End the call.** The guard sends `Done` with a new finish reason, `malformed_tool_arguments`, and drops the
   stream, which closes the connection and stops the billing.
3. **Retry once.** The loop throws the turn away without running its broken call. It tells the model its last tool
   call's arguments were malformed and weren't run, and asks again within the run, as it does for a
   reasoning-limit cut. A second malformed reply in a row ends the try as a provider glitch
   (`provider_glitch: malformed_tool_arguments`), not as a stall.
4. **Record it.** The call's cost row (call_cost's `finish_reason`) and the verdict name it.
5. **Prevent, if a replay shows a knob.** Replay the recorded request (prompt caching makes its input about free) with
   `max_output_tokens` capped: as is, with `parallel_tool_calls: false`, and with `strict: true` on `ls`. If one
   knob removes the glitch at a measurable base rate, adopt it for Responses calls. `strict` needs schemas with
   every property required, optional ones nullable.
   - **Replayed as is, 10 times (2026-10-26 20:55, $0.021): not reproducible at n=10 as a runaway.** The variants
     were not run: the lead's gate was at least 3 of 10.
     - 8 of 10 answered `{"path":"."}`.
     - 2 of 10 derailed at the same point as the recorded call. After `{"path":"."` the model wrote `,"`, a second key
       the schema forbids (`additionalProperties: false`, `path` its only property).
       - #4 ended `…}]}]} }]}]} to=functions.ls  大发快三有error? Wait need valid JSON. Let's use simple. I accidentally
         malformed. "}`: the routing leak, a glitch token and the model's own commentary. 155 characters.
       - #5 ended `…}**ignore**},{" … :[{"path":".","path":"."}]}`. 85 characters.
       - Both are syntactically valid JSON with nonsense keys, and neither collapsed into whitespace.
     - Read: the derail begins with a key outside the tool's schema. Strict decoding would forbid that key and would
       stop the derail there; 10 tries per variant can't show it at a 2-in-10 base rate. Records:
       `tmp/hermes-roko-v7/roko-effort/responses/checks/replay-asis-20261026-205538/` (`replays.jsonl`, the tee's log).
   - Both replayed derails were short, valid JSON, so the guard leaves them be. The tool runs on its `path`, or
     refuses the stray keys, and the model goes on either way. Only the runaway needed cutting.

## Done when

- A streamed reply whose tool-call arguments break JSON is cut within a few hundred characters, its call isn't run,
  the turn is asked again once, and two in a row end the try as a provider glitch.
- The `[[verify]]` test passes.

## Notes

- Records: `tmp/hermes-roko-v7/roko-effort/ab/arms/p5t1/on/run-20261026-203422` (`bodies/07-request.json` and
  `07-response.txt`). Findings: `tmp/hermes-roko-v7/findings/roko-effort.md`.
- The stall watchdog's blindness to argument deltas was right for this call. A legitimate argument stream longer
  than `task_stall_secs` would be cut the same way. That's a separate question; this item shouldn't change what
  counts as progress.
