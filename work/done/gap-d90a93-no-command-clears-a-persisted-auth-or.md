+++
id = "gap-d90a93"
kind = "gap"
title = "No command clears a persisted auth or billing provider quarantine; the operator has to hand-edit provider-health.json"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
size = "M"
subsystem = ["roko-learn/provider-health", "roko-cli/commands"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "730b43d91"
source = "wave-4 follow-up reports 2026-10-02 (PK02)"
discovered_from = "gap-e00238"
anchors = ["crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn provider_health_clear_reopens_the_circuit' crates/roko-learn/ && cargo test -p roko-learn provider_health_clear_reopens_the_circuit"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T02:27:40Z"
commit = "730b43d91"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T01:27:38Z"
forced = false
evidence = "Gate 6a (merged into main as 730b43d91, tree identical to work/backlog-batch-6a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 7,078 passed (roko-agent, roko-cli, roko-learn), roko-cli bin + golden-path canaries + operator_checkout_clean 442/442, hub_ipc 7/7, roko-learn legacy_rule_live + loop_audit_cs_reference; every [[verify]] passes."
+++

## Problem

`ProviderHealthRegistry` (`crates/roko-learn/src/provider_health.rs`) has no way to clear a quarantine from the
outside: its public API is `record_success`, `record_failure`, `record_exhaustion`, `is_available`, `is_healthy`,
`available_providers`, `snapshot`, `get`, `save`, `load_or_new` — no `clear`, `reset`, or `release`-shaped method.
No CLI command, REST route, or MCP action surfaces one either (confirmed: no hits for "clear"/"reset" combined with
"health"/"quarantine"/"circuit" anywhere under `crates/roko-cli/src/commands/`; `config_cmd.rs`'s one mention of
`provider-health.json` is unrelated display/path code).

An `AuthFailure` or `Billing` classification trips the circuit **immediately on the first occurrence** (not after
repeated failures — `ProviderHealth::record_failure`'s `should_trip_billing` check) with a **24-hour cooldown**
(`cooldown_ms`, `provider_health.rs:411`, explicitly by design: "Use a 24-hour cooldown so the provider is
effectively excluded for the remainder of any realistic plan execution"). That reasoning holds for the *current*
run, but the state is then persisted to `provider-health.json` and reloaded by `ProviderHealthRegistry::load_or_new`
at the start of every subsequent run — so an operator who hits an expired-token auth failure, logs back in within
five minutes, and starts a new plan run finds the same provider still excluded for up to 24 real hours, with no
operator-facing way to say "I fixed it, try again now." `is_available`'s only path back to availability is the
cooldown timer itself (`Open` → `HalfOpen` once `now_ms >= cooldown_until`) or manually editing/deleting the JSON
file (undiscoverable without reading the source).

## Why it matters

This is an operator UX/operability gap, not a correctness bug — the circuit-breaker logic itself is working as
designed. Judged severity: **p1**. Reasoning: the consequence is large (up to 24 hours of a provider silently
excluded from every run, after a problem that may have taken 30 seconds to actually fix), the trigger is common (an
expired session token or a momentary billing hiccup, not an exotic edge case), and the only workaround — hand-editing
a JSON file whose existence and location (`.roko/learn/provider-health.json`, via `RokoLayout::for_project(workdir)
.learn_dir()`) are not documented or discoverable from any CLI help text — is not a reasonable operator path. There
is direct precedent for exactly this shape of fix already shipped: `roko safety release` (backlog tasks 1104-1106,
an earlier wave) clears a different kind of persisted "something is blocked and needs an operator to say it's fine
now" state with a real command; this is the same need for provider health.

## Where

- `crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry` — needs a `clear(provider_id)` or
  `reset(provider_id)` method that sets the provider's `ProviderHealth` back to a fresh/closed state and persists
  it.
- `crates/roko-cli/src/commands/` — needs a CLI surface (e.g. `roko provider health clear <id>` or
  `roko provider health list`, mirroring `roko safety controls`/`roko safety release`'s existing shape) that loads
  the registry, clears the named provider (or lists current holds so the operator knows what's quarantined and
  why — `health_hold`'s `class`/`reason`/`until_ms`/`definitive` fields, entry (c) of this report, are exactly the
  data to show), and saves it back.
- Optionally `crates/roko-serve/src/routes/` for a REST equivalent, matching the CLI/REST pairing pattern used for
  safety release.

## Current state

Unmitigated. No read path, clear path, or operator-facing listing exists at all; the only way to inspect or change
quarantine state is to find and edit `provider-health.json` directly, which nothing tells the operator to do.

## Plan

1. `ProviderHealthRegistry::clear(&self, provider_id: &str)`: reset the named provider's `ProviderHealth` to
   `CircuitState::Closed` with `consecutive_failures = 0`, `cooldown_until = None`, leaving historical counters
   (`total_requests`/`total_failures`) alone since they are honest history, not live state. Persist via the
   existing `save` path.
2. `roko provider health list` (read-only: every provider's state, class, reason, `until_ms`, `definitive`, reusing
   entry (c)'s `health_hold` output) and `roko provider health clear <id>` (calls `clear`, persists, reports
   before/after).
3. Tests: `clear` returns a provider whose circuit was Open with `AuthFailure` to `Closed`/available immediately;
   the CLI command round-trips through a real `provider-health.json` file.

## Done when

- An operator can list quarantined providers and their reasons, and clear one by id, without touching the JSON
  file directly.
- The `[[verify]]` command passes.

## Notes

- Keep `total_requests`/`total_failures` history on clear — only the live circuit state (consecutive failures,
  cooldown, circuit state) should reset; the point is to stop blocking the provider, not to erase its track record.
- Pairs with entry (c) of this same report: once auth failures are correctly classified and reported as
  `definitive: true` with a specific reason, this command is what an operator actually uses in response to seeing
  that reason.

## Progress

- 2026-10-03: implemented on `work/gap-d90a93` at d19b0eed1; `roko config providers reset-health [<provider>]` (beside `providers health`, which reads the same file), `ProviderHealthRegistry::clear`, and the 1115 auth hint points at it. Cargo verification deferred to the batch gate.
