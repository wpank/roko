+++
id = "bug-254553"
kind = "bug"
title = "roko serve's signal dispatch names a vendor, 'claude', as the agent of a failed template dispatch (its output's tag and its episode) and of any output with no agent tag"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "hermes-roko round v6, roko-chathost while fixing bug-c7de28, 2026-10-17"
anchors = ["crates/roko-serve/src/dispatch.rs::dispatch_agent", "crates/roko-serve/src/dispatch.rs::dispatch_template", "crates/roko-serve/src/dispatch.rs::append_dispatch_episode"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'tag\\(\"agent\", \"claude\"\\)|\"claude\"\\.to_string\\(\\)' crates/roko-serve/src/dispatch.rs && grep -rqw 'fn template_dispatch_names_no_default_vendor' crates/roko-serve/ && cargo test -p roko-serve template_dispatch_names_no_default_vendor"
+++

## Problem

roko serve's signal dispatch (`crates/roko-serve/src/dispatch.rs`) runs an agent template when a subscription
matches a signal. Where it does not know which agent ran, it names a vendor, "claude", whatever template and model
ran:
- `dispatch_agent` (line 1799 at 527ef29e8): when `dispatch_template` fails, the fallback output is tagged
  `agent = "claude"`.
- `dispatch_template` (line 1910): when the agent backend fails, the failure output is tagged `agent = "claude"`.
- `dispatch_template` (line 1933): the agent output it publishes (`ServerEvent::AgentOutput`, which the hub bridge turns
  into `agent_output`) takes the output's `agent` tag, else "claude".
- `append_dispatch_episode` (line 2552): the episode it writes to `.roko/episodes.jsonl` takes the same tag, else
  "claude", as its `agent_id`.

So a failed dispatch of a template routed to any model is recorded in the episode log as agent "claude". An output
with no `agent` tag is also published to the hub as "claude" and recorded that way. Found 2026-10-17 while fixing
bug-c7de28, which removed the same default from prompt runs (merged at 800c664e4).

## Why it matters

- Goal `visibility`, and the pitch: roko is provider- and model-agnostic, yet its episode log and hub name a vendor
  for work that vendor never did.
- Episodes are learning data: `roko learn episodes` lists them, and serve suggests templates from recent ones
  (`dispatch_loop`). A failure filed under "claude" is put on an agent that did not run.

## Where

- `crates/roko-serve/src/dispatch.rs::dispatch_agent`: `.tag("agent", "claude")` on the fallback output.
- `crates/roko-serve/src/dispatch.rs::dispatch_template`: `.tag("agent", "claude")` on the backend-failure output,
  and `.map_or_else(|| "claude".to_string(), …)` for the published agent id.
- `crates/roko-serve/src/dispatch.rs::append_dispatch_episode`: `.map_or_else(|| "claude".to_string(), …)` for the
  episode's `agent_id`.

The success path usually has a tag: `build_agent` leaves the agent's `name` empty, and each adapter then picks its
own name and tags its output with it. The failure paths never do.

## Current state

Open at 527ef29e8: all four defaults are there.

## Plan

1. Name what is known instead of a vendor. Use the template's name for the agent (the episode already keeps the model
   in `model` and the template in `agent_template`). Never use a vendor name.
2. Tag both failure outputs with that name. Fall back to it, not "claude", where an output has no `agent` tag.
3. Add the test `template_dispatch_names_no_default_vendor`: a template dispatch whose backend fails writes an episode
   and publishes events, and none of them names "claude" as its agent.

## Done when

The `[[verify]]` passes. Its static part checks that `dispatch.rs` no longer tags or defaults an agent to "claude".
