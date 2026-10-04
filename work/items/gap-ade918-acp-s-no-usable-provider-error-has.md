+++
id = "gap-ade918"
kind = "gap"
title = "ACP's no-usable-provider error has no login hint for an auth quarantine"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-acp", "roko-learn/provider-failover"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (health bundle, gap-d90a93)"
discovered_from = "gap-d90a93, related to bug-9ca6d7 (same health-bundle report, message/UX half)"
anchors = ["crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-learn/src/provider_failover.rs::Failover", "crates/roko-cli/src/graph_task_dispatch/failover.rs::no_usable_provider"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-9ca6d7", "gap-d90a93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_auth_quarantine_message_includes_a_login_hint' crates/roko-acp/ && cargo test -p roko-acp acp_auth_quarantine_message_includes_a_login_hint"
+++

## Problem

When ACP has no usable provider for a prompt, the error it surfaces to the editor has no recovery guidance —
just a bare, nested string. `crates/roko-acp/src/bridge_events/mod.rs:788`:

```rust
let mut candidate = match model_failover.start(&provider_health, &model_key_for_dispatch) {
    Ok(candidate) => candidate,
    Err(why) => {
        emit_dispatch_failure(&event_sender, format!("Error: {why}")).await;
        return Err(anyhow::anyhow!("no usable provider for the prompt: {why}").into());
    }
};
```

`why` comes from `roko_learn::provider_failover::Failover::start` (`crates/roko-learn/src/provider_failover.rs:367`),
which returns `Result<FailoverCandidate, String>` — on the no-candidates-left path it returns `self.next(registry)`'s
`Err(error)` verbatim, a bare `String` with no structure and no login hint, however the refusal was classified.

Compare the CLI/Graph-dispatch path's equivalent, `GraphTaskDispatcher::no_usable_provider`
(`crates/roko-cli/src/graph_task_dispatch/failover.rs`, ~lines 717-802): it builds a message naming every refusal,
every skipped fallback, a fallback-specific recovery suggestion, and — specifically for the case this item is
about — appends a `credentials_hint` for every refusal whose class is `AUTH_FAILURE` ("A provider that rejected
roko's credentials needs a login, which waiting does not bring (backlog 1115)."). ACP's path has no equivalent:
an operator whose Claude Code/Codex session expired gets "no usable provider for the prompt: <bare reason>" with
no hint that logging back in (rather than retrying, or waiting) is what fixes it.

`roko-serve`'s dispatch path (`crates/roko-serve/src/dispatch.rs`) has no matching string at all by direct grep —
either it delegates to the CLI's richer path and inherits the good message, or it has its own undiscovered gap
under different wording. Not confirmed either way; flagged in Notes below for whoever picks this up.

## Why it matters

Goal: truth / operator UX. This is the same root problem bug-52c48f and bug-9ca6d7 cover for provider *health
recording* (an auth failure gets no distinguishing treatment) — this item is the user-facing *message* side of
the same gap, specific to the ACP transport. An ACP user (an editor integration, not a terminal) has fewer cues
than a CLI user to guess "I need to log back in" from a bare error string.

## Where

- `crates/roko-acp/src/bridge_events/mod.rs:788` — the `Err(why)` arm that wraps `Failover::start`'s bare string.
- `crates/roko-learn/src/provider_failover.rs::Failover::start` (line 367) and `::next` — the no-structure error path.
- The reference implementation to mirror or reuse: `crates/roko-cli/src/graph_task_dispatch/failover.rs::no_usable_provider`
  (~717-802) and `::credentials_hint` (~804-816).

## Current state

Unfixed. `Failover::start`'s `String` error type has no way to carry per-refusal classification (`ErrorClass`) or
a structured refusal list out to ACP the way `GraphTaskDispatcher`'s own failover bookkeeping does.

## Plan

1. Either give `Failover` (or a thin wrapper) a richer error type carrying the refusal list and each refusal's
   `ErrorClass`, reusable by both the CLI/Graph-dispatch path and ACP, and have ACP build the same
   `credentials_hint`-bearing message `no_usable_provider` does; or, simpler, have ACP catch an `AUTH_FAILURE`
   class on its own provider-health lookup before/around the `Failover::start` call and prepend a login hint to
   whatever string `why` carries.
2. Add a regression test: ACP dispatch with every candidate provider quarantined on `AUTH_FAILURE` produces an
   error message containing a login hint, not just the bare refusal reason.
3. Check `roko-serve/src/dispatch.rs`'s own no-usable-provider path (confirm whether it shares this gap or already
   inherits the good message) before deciding whether this item's fix needs to touch roko-serve too.

## Done when

- An ACP dispatch that fails because every candidate provider is auth-quarantined produces a message that tells
  the operator to log back in, not just a bare refusal string.
- The `[[verify]]` command passes.

## Notes

- Discovered while investigating the wave-6 "health bundle" report (gap-d90a93) alongside bug-9ca6d7 (the
  Unknown-health-classification half of the same report) — this is the message/UX half, filed separately since
  the fix is in a different crate (roko-acp) with a different mechanism (error message construction, not health
  recording).
- `roko-serve`'s own exposure (if any) is unconfirmed — step 3 above should resolve that before this item is
  closed, not as a precondition to filing it.

## Progress

- 2026-10-04 (w4-length): implemented on `work/gap-fd0c0b` at 0c553f792; cargo verification deferred to the batch
  gate. `credentials_fix` and `credentials_hint` moved from the CLI's Graph failover to
  `roko_learn::provider_failover`, and the CLI now calls them, so one wording serves every path.
  `Failover::credentials_hints` gives one hint per auth-refused provider. ACP's `no_usable_provider_reason`
  appends those hints to the failover's reason. Test `acp_auth_quarantine_message_includes_a_login_hint`.
- Step 3: roko-serve's `dispatch.rs` has no failover start of its own. The other `Failover::start` caller,
  serve's one-shot bench dispatch (`roko-cli/src/serve_runtime.rs::dispatch_bench_prompt`), had the same bare
  message and now appends the same hints.
