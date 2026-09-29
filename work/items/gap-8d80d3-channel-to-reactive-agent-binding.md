+++
id = "gap-8d80d3"
kind = "gap"
title = "Channel-to-Reactive-Agent Binding"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
subsystem = ["roko-runtime"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding"
discovered_from = "audit:tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding"
anchors = ["crates/roko-runtime/src/channel_binding.rs::ChannelBindingRouter", "crates/roko-runtime/src/lib.rs", "crates/roko-runtime/src/reactive_agent.rs::ReactiveAgentSupervisor", "crates/roko-runtime/src/reactive_agent.rs::TriggerCondition", "crates/roko-core/src/config/channels.rs::ChannelConfig", "crates/roko-serve/src/routes/webhooks.rs::slack_webhook", "crates/roko-serve/src/state.rs::AppState"]
links = { depends_on = ["spec-743a7e"], blocks = [], related = ["spec-743a7e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^pub mod channel_binding' crates/roko-runtime/src/lib.rs && grep -rqE 'ChannelBindingRouter' crates/roko-serve/src crates/roko-cli/src && grep -rqw 'fn channel_message_wakes_bound_reactive_agent' crates/roko-runtime/ && cargo test -p roko-runtime channel_message_wakes_bound_reactive_agent"
+++

## Problem

An inbound message on a chat channel (Slack, Telegram, Discord, ...) cannot wake a roko agent. The pieces
exist separately but nothing connects them, and the connecting code is not even compiled:

- `ChannelBindingRouter` (`crates/roko-runtime/src/channel_binding.rs`) would map a `(platform_id,
  channel_id)` message to a `PulseBus` pulse and a direct wake of a `ReactiveAgentHandle`. The file is not
  declared in `crates/roko-runtime/src/lib.rs`, so it does not build and nothing calls it.
- `ReactiveAgentSupervisor` (`crates/roko-runtime/src/reactive_agent.rs`) implements sleep/wake on
  `TriggerCondition::BusTopicMatch` or `CronSchedule`, but has no production caller (only tests and the
  `lib.rs` re-export).
- No agent config can declare a channel trigger: `AgentTriggerSpec` does not exist anywhere, and
  `roko_core::trigger::TriggerKind` has no `Channel` variant.

Expected: with a `[[channels]]` entry binding a Slack channel to an agent, a message posted there (arriving on
`POST /webhooks/slack`) wakes that agent once, and the agent's reply can be routed back.

## Why it matters

- Goal `features`. Channel-driven agents are part of the Hermes-style messaging surface: operators talking to
  roko from chat instead of the terminal.
- Related: `spec-743a7e` (channel config schema; `ChannelConfig` is also uncompiled), `gap-63055e` (platform
  transports: `ChatBridge` and six adapters, uncompiled and stubbed). The original backlog spec (#428) also
  depended on "#02 Reactive Agent Mode" (a host that runs reactive agents), which was never built.

## Where

- `crates/roko-runtime/src/channel_binding.rs::ChannelBindingRouter` (227 lines, 4 tests): `new(Arc<PulseBus>)`,
  `add_binding(&ChannelConfig, Option<ReactiveAgentHandle>)`, `route(platform_id, channel_id, message_id, text)`.
  `route` publishes `platform.message.<platform_id>.<channel_id>`, then one `trigger.<name>.fired` pulse per
  `trigger_bindings` entry, then calls `handle.wake(Some(text))`. Imports
  `roko_core::config::channels::ChannelConfig`.
- `crates/roko-runtime/src/lib.rs`: module list (lines ~26-58). Missing: `channel_binding`, `platforms`,
  `adapters`, and several other files present in `src/` (`command_dispatcher.rs`, `delivery.rs`,
  `message_audit.rs`, `message_formatter.rs`, `notification_batcher.rs`, `webhook_dead_letter.rs`, ...).
- `crates/roko-runtime/src/reactive_agent.rs`: `TriggerCondition` (line 66), `ReactiveWakeEvent` (87),
  `ReactiveAgentConfig { agent_id, conditions, handler_timeout }` (110), `ReactiveAgentHandle::wake` (153),
  `ReactiveAgentSupervisor::new(config, bus, handler)` (214) and `spawn()` (233),
  `condition_for_trigger_name` (455).
- `crates/roko-core/src/config/channels.rs::ChannelConfig` `{ platform_id, channel_id, agent_name,
  trigger_bindings }`: a `[[channels]]` table; not declared in `config/mod.rs`, and `RokoConfig` has no
  `channels` field (that is `spec-743a7e`).
- `crates/roko-serve/src/state.rs:533`: `AppState.pulse_bus: Arc<PulseBus>` (created at ~1141). The bus a
  router would publish on.
- `crates/roko-serve/src/routes/webhooks.rs::slack_webhook` (~455): verifies the Slack signature and timestamp,
  then persists the event as a `Signal`. The natural first inbound source; today it does not publish a pulse
  or route to agents.
- `crates/roko-cli/src/agent_serve.rs`: per-agent sidecar; the #428 spec proposed wiring channel triggers here.

## Current state

- Commit `9c6ec420c` added `channel_binding.rs`, `config/channels.rs`, `platforms.rs` and `adapters/` as orphan
  files. Re-verified 2026-09-29: still orphaned; nothing changed since.
- The router's own dependencies are small: `ChannelConfig` (roko-core, uncompiled), `PulseBus` and
  `ReactiveAgentHandle` (compiled). It does not need `ChatBridge`.
- Design issue in `route`: it both publishes the platform pulse and wakes the handle directly. An agent whose
  conditions also match `platform.message.*` would wake twice per message.
- Topic naming differs between the code (`platform.message.<platform_id>.<channel_id>`) and the #428 spec
  (`channel.<name>.message.<filter>`). The code's naming is the one that exists.
- No reply path exists: agent output has no route back to the originating channel.

## Plan

1. Land `spec-743a7e` first, or at minimum declare `pub mod channels;` in `crates/roko-core/src/config/mod.rs`
   and add `channels: Vec<ChannelConfig>` (`#[serde(default)]`) to `RokoConfig`.
2. Declare `pub mod channel_binding;` in `crates/roko-runtime/src/lib.rs` and re-export
   `ChannelBindingRouter`. Fix whatever no longer compiles (the file has never been built).
3. Pick one wake mechanism per binding. Recommended: bus only. Remove the direct `handle.wake` from `route`
   and give each bound agent `TriggerCondition::BusTopicMatch { topic: "platform.message.<platform_id>.<channel_id>" }`.
   Alternative: keep the direct wake and give bound agents no matching bus condition. Either way, one message
   must produce exactly one wake.
4. Add a channel trigger to agent config: a `Channel { platform_id, channel_id: Option<String> }` trigger kind
   (in `roko_core::trigger::TriggerKind` or a new agent trigger spec), plus a helper
   `channel_trigger_to_condition(..) -> TriggerCondition` next to `condition_for_trigger_name`
   (`None` channel means `platform.message.<platform_id>.*`).
5. Host: at `roko serve` startup, build a `ChannelBindingRouter` on `state.pulse_bus` from `config.channels`,
   and for each channel-triggered agent spawn a `ReactiveAgentSupervisor` whose handler dispatches the message
   text to the agent. In `slack_webhook`, after the signature check, call `router.route(..)` for
   `message` events with the event's channel id, `ts` and text. Other platforms follow via `gap-63055e`.
6. Reply routing (send the agent's answer back to the channel/thread) needs a platform transport; leave it
   to `gap-63055e` or a follow-up item, and say so in the code.
7. Tests: `channel_message_wakes_bound_reactive_agent` in roko-runtime (route a message; assert the bound
   agent's handler sees exactly one wake and an unbound agent none), plus one for two agents on different
   channels waking independently.

## Done when

- `roko-runtime` compiles `channel_binding` and exports `ChannelBindingRouter`.
- An agent bound to a channel wakes exactly once per routed message; agents on other channels do not wake.
- `roko serve` builds the router from `[[channels]]` and the Slack webhook routes channel messages to it.
- Verify (the current command has prose inside it and is not runnable). Suggested replacement:
  `grep -qE '^pub mod channel_binding' crates/roko-runtime/src/lib.rs && grep -rqE 'ChannelBindingRouter' crates/roko-serve/src crates/roko-cli/src && grep -rqw 'fn channel_message_wakes_bound_reactive_agent' crates/roko-runtime/ && cargo test -p roko-runtime channel_message_wakes_bound_reactive_agent`

## Notes

- Hard dependency: `spec-743a7e` (the router takes `&ChannelConfig`). Soft dependency: `gap-63055e` (only
  needed for non-Slack inbound sources and for replies).
- Security: channel text is untrusted input that will reach an agent prompt. Keep the Slack signature check in
  front of the router, run woken agents under a restrictive `AgentContract`, and do not let a channel message
  trigger plan execution or tool use without the normal safety layer. Auth-adjacent: review carefully.
- Adding the other orphan modules to `lib.rs` at the same time is tempting but belongs to `gap-63055e`; keep
  this change to `channel_binding` so it stays reviewable.
- Parallel safety: touches `roko-runtime/src/lib.rs`, `roko-core` config and serve startup; conflicts with
  `spec-743a7e` and `gap-63055e` if done at the same time.

## Original notes

the missing bridge between inbound channel messages and the existing reactive agent supervisor. `ReactiveAgentSupervisor` (line 188, `crates/roko-runtime/src/reactive_agent.rs`) already implements sleep/wake, `TriggerCondition` matching, `PulseBus` integration, and `ReactiveAgentHandle` for…

Imported without verification from:
- `tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding`

Some cited files are gone: `crates/roko-runtime/src/channel_bridge.rs`.

How to verify: Check: `AgentTriggerSpec::Channel { channel, filter }` deserializes from TOML; Channel triggers convert to `TriggerCondition::BusTopicMatch`; `ChannelBridge::inbound()` publishes a `Pulse` that wakes matching reactive agents [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Verified 2026-09-28: commit 9c6ec420c added crates/roko-runtime/src/channel_binding.rs (ChannelBindingRouter, #428) and adapters/{discord,matrix,mattermost,slack,telegram,whatsapp}.rs. These are orphan files: roko-runtime has no `mod channel_binding` or `mod adapters`, so none of it compiles. There is no AgentTriggerSpec::Channel, and ReactiveAgentSupervisor has no production caller (only tests and the lib.rs:96 re-export).

Re-verified 2026-09-29: unchanged. The router, the adapters it would bind (roko-runtime adapters/) and the ChatBridge trait (platforms.rs) are all outside the roko-runtime module tree, so none of it is compiled. This depends on the umbrella gap-63055e wiring those modules first.
