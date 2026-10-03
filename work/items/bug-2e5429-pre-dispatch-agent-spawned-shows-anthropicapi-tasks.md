+++
id = "bug-2e5429"
kind = "bug"
title = "Pre-dispatch agent_spawned shows AnthropicApi tasks as claude_cli (AgentBackend can't represent the direct API)"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-core/agent"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK74 gap-ce1d11)"
discovered_from = "gap-ce1d11"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher", "crates/roko-core/src/agent.rs::AgentBackend", "crates/roko-core/src/agent.rs::ProviderKind"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_spawned_label_reflects_anthropic_api' crates/roko-cli/ && cargo test -p roko-cli agent_spawned_label_reflects_anthropic_api"
+++

## Problem

Graph dispatch's pre-dispatch `agent_spawned` TUI event derives its provider label by converting the *planned*
`AgentBackend` into a `ProviderKind`, which can never produce `ProviderKind::AnthropicApi`:

```rust
// crates/roko-cli/src/graph_task_dispatch.rs:1375-1377
let planned_provider: String =
    roko_core::ProviderKind::from(dispatch_plan.model.backend)
        .label()
        .to_string();
```

`AgentBackend` (`crates/roko-core/src/agent.rs:161-180`) has no variant for "Anthropic's direct HTTP API" at all —
its `Claude` variant is explicitly documented as "Anthropic's `claude` CLI (stream-json protocol)" only. The
`From<AgentBackend> for ProviderKind` impl (`agent.rs:249-262`) has exactly one arm for `AgentBackend::Claude`,
and it always maps to `ProviderKind::ClaudeCli`:

```rust
AgentBackend::Claude => ProviderKind::ClaudeCli,
```

So any Claude-family model routed through roko's direct Anthropic Messages API (`ProviderKind::AnthropicApi`, a
real, distinct variant used elsewhere for actual dispatch) still gets `AgentBackend::Claude` as its *planned*
backend (the closest `AgentBackend` match for a Claude model), and this display code then converts that back to
`ClaudeCli` — losing the distinction. The round trip `AnthropicApi -> AgentBackend::Claude -> ClaudeCli` is lossy
by construction: `ProviderKind::to_backend()` (referenced in `AgentBackend`'s own doc comment, line 155-156) must
already collapse both API and CLI Claude dispatch onto the single `AgentBackend::Claude` value, and there is no
way back.

## Why it matters

Every `AnthropicApi`-routed task shows as `claude_cli` in the dashboard/TUI's live agent list and in whatever logs
or diagnostics (`roko diagnose`, per an adjacent wave's PK02 findings) read this label. An operator or on-call
reviewer cannot tell, from this label, whether a given attempt is running the real Claude Code CLI subprocess
(with its own tool loop, settings, approvals) or a direct API call (roko's own tool loop, different cost/latency
profile, different failure modes) — exactly the kind of provider-identity confusion an earlier wave's bug-466060/
gap-466060 (duplicate error classifiers) and bug-52c48f (serve_runtime recording failures as `Unknown`) are also
about, in this same provider-identity-fidelity area.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs:1364-1387` (`GraphTaskDispatcher::dispatch`, the "T04: Pre-dispatch
  agent_spawned" block) — the actual bug site.
- `crates/roko-core/src/agent.rs::AgentBackend` (161-180) and `impl From<AgentBackend> for ProviderKind` (249-262)
  — the structural cause: `AgentBackend` cannot represent "Claude via the direct API" as distinct from "Claude via
  the CLI".
- `crates/roko-core/src/agent.rs::ModelSpec` (309-ish, `pub backend: AgentBackend`) — what `dispatch_plan.model`
  actually is at the point this display code runs, before the real provider is resolved.

## Current state

Unfixed. The comment at the call site even says why it takes this lossy shortcut: "Derive a provider label from
the planned backend so the dashboard can display it before the actual dispatch resolves a provider" — the fully
resolved provider (which does carry a correct `ProviderKind`, per other structs seen elsewhere in dispatch code
that have both a `provider_kind: ProviderKind` and a `backend: AgentBackend` field side by side) is not yet known
at this point in the dispatch path.

## Plan

Two options, in increasing order of invasiveness:
1. Have `ModelSpec` (or whatever constructs `dispatch_plan.model`) carry the already-known `ProviderKind` directly
   alongside `backend: AgentBackend`, the same way the later, fully-resolved dispatch-target struct does, and have
   the pre-dispatch display read that field instead of re-deriving a lossy one from `AgentBackend`. This requires
   no new `AgentBackend` variant and fixes the display without touching the CLI/binary-selection logic at all.
2. If (1) is not feasible this early in the pipeline, add the missing distinction to `AgentBackend` itself (e.g.
   a dedicated API-vs-CLI flag, or split `Claude` into two variants) and update `ProviderKind::to_backend()` and
   `From<AgentBackend> for ProviderKind` accordingly — more invasive, touches routing code that currently assumes
   one `AgentBackend::Claude`.
Recommended: option 1, since the information already exists earlier in planning (whatever decided to route this
model to the Anthropic API chose `ProviderKind::AnthropicApi` before ever touching `AgentBackend`).

## Done when

- A task dispatched through `ProviderKind::AnthropicApi` shows `anthropic_api` (or equivalent), not `claude_cli`,
  in the pre-dispatch `agent_spawned` event.
- The `[[verify]]` command passes.

## Notes

- Do not change `AgentBackend::Claude`'s meaning for the CLI-selection path itself (spawning the actual `claude`
  binary) — only this display-label derivation is in scope.
- Related, same wave: bug-52c48f (serve_runtime records failures as `Unknown` instead of the classified error
  taxonomy) and bug-466060 (a third copy of the provider error classifier) are different instances of the same
  underlying pattern — provider identity/classification getting flattened or lost on a side path. Not duplicates;
  don't merge.
