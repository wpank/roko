+++
id = "gap-8d80d3"
kind = "gap"
title = "Channel-to-Reactive-Agent Binding"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-runtime"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding"
discovered_from = "audit:tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding"
anchors = ["crates/roko-runtime/src/channel_binding.rs::ChannelBindingRouter", "crates/roko-runtime/src/lib.rs", "crates/roko-runtime/src/reactive_agent.rs::ReactiveAgentSupervisor"]
links = { depends_on = [], blocks = [], related = ["spec-743a7e"], supersedes = [], duplicate_of = "" }
+++
the missing bridge between inbound channel messages and the existing reactive agent supervisor. `ReactiveAgentSupervisor` (line 188, `crates/roko-runtime/src/reactive_agent.rs`) already implements sleep/wake, `TriggerCondition` matching, `PulseBus` integration, and `ReactiveAgentHandle` for…

Imported without verification from:
- `tmp/backlog/archive/428-channel-to-reactive-agent-binding.md#428 — Channel-to-Reactive-Agent Binding`

Some cited files are gone: `crates/roko-runtime/src/channel_bridge.rs`.

How to verify: Check: `AgentTriggerSpec::Channel { channel, filter }` deserializes from TOML; Channel triggers convert to `TriggerCondition::BusTopicMatch`; `ChannelBridge::inbound()` publishes a `Pulse` that wakes matching reactive agents [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Verified 2026-09-28: commit 9c6ec420c added crates/roko-runtime/src/channel_binding.rs (ChannelBindingRouter, #428) and adapters/{discord,matrix,mattermost,slack,telegram,whatsapp}.rs. These are orphan files: roko-runtime has no `mod channel_binding` or `mod adapters`, so none of it compiles. There is no AgentTriggerSpec::Channel, and ReactiveAgentSupervisor has no production caller (only tests and the lib.rs:96 re-export).
