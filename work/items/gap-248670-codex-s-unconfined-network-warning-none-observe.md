+++
id = "gap-248670"
kind = "gap"
title = "Codex's unconfined network warning (None/Observe + skip-permissions) is a transient log line, not a durable safety signal"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/provider"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "wave-3 follow-up reports 2026-10-02 (w3-pk05 gap-843aef)"
discovered_from = "gap-843aef (backlog task 1213)"
anchors = ["crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args", "crates/roko-agent/src/safety/sandbox.rs::allows_sandbox_bypass"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn unconfined_codex_run_is_recorded_as_a_safety_signal' crates/roko-agent/ && cargo test -p roko-agent unconfined_codex_run_is_recorded_as_a_safety_signal"
+++

## Problem

When a run's sandbox level is `None` or `Observe` and the runner config enables `dangerously_skip_permissions`,
Codex's own OS sandbox is switched off by design (`SandboxLevel::allows_sandbox_bypass`,
`crates/roko-agent/src/safety/sandbox.rs:94-104`: "Only `None` and `Observe` allow it... a vendor sandbox is the
only OS-level confinement an agent has"). `codex_sandbox_args` (`crates/roko-agent/src/provider/claude_cli.rs:328-346`)
implements this deliberately and is covered by a passing test
(`codex_keeps_workspace_sandbox_with_skip_permissions`, same file, ~line 546) — this is accepted, shipped behaviour,
not a regression.

The gap is what happens to the contract's network pin in that configuration. With the sandbox bypassed, roko can no
longer apply `sandbox_workspace_write.network_access=false`, so a role the contract keeps off the network
(`codex_network_pins`) only loses `web_search`; the shell itself is unconfined. The function's own doc comment says
so plainly: "Without the sandbox the network pins switch off only web search, so a run whose contract keeps it off
the network is logged as unconfined." The only signal is one `tracing::warn!` call
(`crates/roko-agent/src/provider/claude_cli.rs:339-344`), which goes through the default `tracing_subscriber`
log stream (`roko=info` by default, `crates/roko-cli/src/main.rs:1561-1567`) and nowhere else: no safety-incident
ledger entry, nothing in `roko doctor`, nothing on the attempt's own record or episode, nothing in the TUI. An
operator who was not watching the log at the moment that attempt ran has no durable, queryable way to find out
afterward that it ran fully unconfined.

## Why it matters

Goal: truth (safe agents, one settled record per attempt). `None`/`Observe` + skip-permissions is an operator-opted-in,
intentionally loose configuration, so this is not about refusing it — it is about making "this attempt ran with no
network confinement" a fact the operator can find later, the same way other safety-relevant decisions (immune
denials, quarantine) are meant to be. A transient INFO/WARN log line in a long run is easy to miss and is not
part of any record anyone replays or audits.

## Where

- `crates/roko-agent/src/provider/claude_cli.rs::codex_sandbox_args` (lines ~328-346): where the warning is emitted
  and the bypass argument is built.
- `crates/roko-agent/src/safety/sandbox.rs::SandboxLevel::allows_sandbox_bypass` (lines ~94-104): the policy this
  implements.
- No existing sink: checked for a safety-incident recorder (`record_incident`/`SafetyIncident`) and found none wired
  to this call site, and no reference from `roko doctor`, the TUI, or the attempt/episode record.

## Current state

Unfixed. The behaviour (bypass + warn) is correct and tested as designed; only the warning's durability/visibility
is missing.

## Plan

1. Decide where this belongs as a durable record: either the attempt's own verdict/episode record (so it shows up
   wherever that attempt's history is inspected), or a lightweight entry in whatever safety-incident/quarantine
   ledger exists for other denial-adjacent events, or both.
2. Thread a bool/reason (e.g. `ran_unconfined: Some("contract keeps this role off the network, but None/Observe +
   skip-permissions dropped Codex's own sandbox")`) from `codex_sandbox_args`'s caller up to wherever the attempt's
   record is finalized.
3. Keep the `tracing::warn!` for operators watching live logs; add the durable record alongside it, not instead of it.
4. Regression test: build the attempt record for a `None`/`Observe` + skip-permissions + network-pinned-off run and
   assert the durable signal is present; a `Restrict`-or-above run (sandbox stays on) must not set it.

## Done when

- A run matching this configuration leaves a durable, queryable record that it ran without network confinement,
  not only a transient log line.
- The `[[verify]]` command passes.

## Notes

- Do not change the underlying policy (bypass on `None`/`Observe` + skip-permissions) — that is a deliberate,
  already-tested design choice (backlog task 1213, `gap-843aef`, done). This item is about visibility after the
  fact, not about confining Codex further.
- Related: gap-baab0a (done) is a different concern (Codex's own tool-call output policy enforcement), not this.
