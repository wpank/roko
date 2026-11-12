+++
id = "bug-0e0dd9"
kind = "bug"
title = "roko asks every thinking Claude model for manual extended thinking (thinking.type enabled with budget_tokens), which Claude 4.7 and later reject with a 400 and the 4.6 models deprecate, so on today's models every Anthropic API call with thinking on fails; a profile that marks a model adaptive should get thinking.type adaptive with output_config.effort"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-agent/provider"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "2ab4e7533"
source = "round v7 roko-effort, the current Anthropic docs (platform.claude.com extended-thinking, read 2026-10-27) while fixing bug-f30f36; the lead's ask"
discovered_from = "agent:roko-effort (round v7, bug-f30f36)"
anchors = ["crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::thinking_budget", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs::AnthropicMessagesBackend", "crates/roko-core/src/config/provider.rs::ModelProfile"]
links = { depends_on = [], blocks = [], related = ["bug-f30f36"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_adaptive_claude_profile_asks_for_adaptive_thinking' crates/roko-agent/src && cargo test -p roko-agent --lib -- an_adaptive_claude_profile_asks_for_adaptive_thinking"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T05:27:30Z"
commit = "2ab4e7533"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-27T05:23:30Z"
forced = false
evidence = "Merged as 2ab4e7533 (branch 6766fc1a5) on the lead's call after the round's timebox. Post-merge verify by the roster's rule, in roko-effort's worktree detached at 2ab4e7533 on its APFS target clone: the item's [[verify]] passes (an_adaptive_claude_profile_asks_for_adaptive_thinking ok; log roko-effort/logs/postmerge-0e0dd9-2ab4e7533.log). The $0 proof, against a stand-in Messages API for an adaptive-only model that answers manual extended thinking with the API's 400: red on main (477f6184c: http 400 \"thinking.type.enabled\" is not supported; sent enabled with budget_tokens 32768), green with the fix (the streamed request gets its answer, the answered one asks adaptive at effort high with no beta, a profile that says nothing keeps its budget). On the branch: fmt, clippy (6 crates), check --tests, 54 targeted tests and the lib suites green (acp 237, agent 2427, cli 4071, core 2049, learn 1411, serve 1088; log postmerge-f30f36-unit-0e0dd9.log). Still owed: the live check against the real API (about a cent), which needs an Anthropic key."
+++

## Problem

roko's Anthropic Messages backend turns thinking on for every `supports_thinking` profile with manual extended
thinking: `"thinking": {"type": "enabled", "budget_tokens": N}`, plus the `extended-thinking-2025-02-19` beta header
(`provider/anthropic_api/tool_loop.rs`: `thinking_budget`, `build_body`, `build_streaming_body`, `headers`). Current
Claude models reject that request.

The rule, from [Extended thinking](https://platform.claude.com/docs/en/build-with-claude/extended-thinking), read
2026-10-27:

> Extended thinking (`thinking.type: "enabled"` with `budget_tokens`) is deprecated on the Claude 4.6 models
> (requests using it still succeed). Claude 4.7 and later models do not support it and reject requests that use it,
> returning a 400 error. On Claude 4.5 and earlier models that support thinking, extended thinking is the only
> available thinking mode. Claude Mythos Preview supports both modes.

What that means for each model family:

- **Rejected (400):** Claude 4.7 and later, for example Claude Opus 5.5, Claude Sonnet 5, Claude Sonnet 5.5, Claude
  Fable 5.1 and Claude Haiku 5.5. The 400's message starts with `"thinking.type.enabled" is not supported`.
- **Deprecated, still accepted:** Claude Opus 4.6 and Claude Sonnet 4.6.
- **Extended thinking only:** Claude Sonnet 4.5, Claude Opus 4.5, Claude Haiku 4.5 and earlier Claude 4 models.
  They reject `type: "adaptive"` with a 400.
- **Per-model table:** [thinking troubleshooting, supported
  models](https://platform.claude.com/docs/en/build-with-claude/thinking-troubleshooting#supported-models).

The migration ([Migrating to adaptive
thinking](https://platform.claude.com/docs/en/build-with-claude/extended-thinking#migrating-to-adaptive-thinking)):

- remove `budget_tokens`;
- set `"thinking": {"type": "adaptive"}`;
- control the depth with `"output_config": {"effort": ...}`.

So on today's models every Anthropic API call that roko makes with thinking on fails with a 400, before any of
bug-f30f36's stream handling matters. Latent: no run this round used the Anthropic API, for want of a key.

## Why it matters

The Anthropic API is one of roko's vendor-neutral paths. With bug-f30f36 fixed it can run tools, but on any Claude
model from 4.7 on, it still can't make a single call with thinking on, and thinking is on by default for a
`supports_thinking` profile.

## Where

- `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs`: `thinking_budget`, `create_tool_loop_backend_with_api_key`,
  `AnthropicMessagesBackend` (`build_body`, `build_streaming_body`, `headers`, `reasoning_budget`).
- `crates/roko-core/src/config/provider.rs::ModelProfile`: where the model's thinking capability is declared.

## Design (the lead's constraints, 2026-10-27)

1. **The choice comes from the model's capability in its profile, never from string-matching model names in the
   request path.** A profile marks a model that thinks adaptively. Two options:
   - a new `ModelProfile` field, such as `thinking_mode = "adaptive"`;
   - Anthropic's reading of the existing provider-specific `thinking_level` (`thinking_level = "adaptive"`).

   The first is cleaner but touches every full `ModelProfile` literal (about 23 in 14 files, as `api` did for
   gap-1ba0ac).
2. **An adaptive profile** sends `"thinking": {"type": "adaptive"}` and `"output_config": {"effort": E}`, with no
   `budget_tokens` and no extended-thinking beta header. `E` is the request's effort, else the profile's
   `reasoning_effort`, else `[agent] default_effort`, mapped to `low`, `medium`, `high` or `max`.
3. **Other profiles keep** `enabled` + `budget_tokens`, as now.
4. A request that turns thinking off keeps sending no thinking block.
5. `display` stays at the model's default (`omitted` on 4.7+): bug-f30f36 keeps the empty thinking blocks and their
   signatures and sends them back.

## Done when

- A stand-in Messages API answers `enabled` with the API's 400 (`"thinking.type.enabled" is not supported ...`) for a
  model whose profile marks it adaptive. It answers `adaptive` only for such a model, with `output_config.effort`
  present.
- With that stand-in, red on main (the request carries `enabled` and gets the 400), green with the fix. An older
  profile still sends `enabled` + `budget_tokens`.
- **Still owed:** the live check against the real API (about a cent, one adaptive and one older model). It needs an
  Anthropic key, which is on Will's list.

## Status (2026-10-27: parked at the round's 06:00 timebox, then merged as 2ab4e7533 on the lead's call)

- **Done, on branch `work/bug-0e0dd9`** (6766fc1a5, based on main 2d6510089):
  - 477f6184c adds the stand-in test, `an_adaptive_claude_profile_asks_for_adaptive_thinking`. Its stand-in serves
    an adaptive-only model and answers manual thinking with the API's 400.
  - 6766fc1a5 is the fix, taking the design's second option. `ModelProfile::thinks_adaptively` reads `thinking_level
    = "adaptive"`, and `adaptive_thinking` picks the effort. `set_thinking` writes `thinking: {type: "adaptive"}` and
    `output_config.effort` in the streamed and the answered body, with no budget and no beta header. Any other
    profile keeps `enabled` + `budget_tokens`.
- **Last check** (04:29 to 04:52, log `roko-effort/logs/postmerge-f30f36-unit-0e0dd9.log`):
  - **Red** at 477f6184c (main plus the test): `http 400: "thinking.type.enabled" is not supported for this model
    ...; sent {"budget_tokens":32768,"type":"enabled"}`.
  - **Green** at 6766fc1a5: the test passes, and fmt, clippy (6 crates), check --tests and 54 targeted tests are
    green.
  - **Lib suites** at 6766fc1a5, green: acp 237, agent 2427, cli 4071, core 2049, learn 1411, serve 1088.
- **Merged** as 2ab4e7533 on the lead's call after the timebox. Its `[[verify]]` passed detached at the merge sha.
- **Still owed:** the live check against the real API (about a cent, one adaptive and one older model). It needs an
  Anthropic key.
