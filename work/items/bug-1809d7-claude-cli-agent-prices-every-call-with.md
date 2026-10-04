+++
id = "bug-1809d7"
kind = "bug"
title = "Claude CLI agent prices every call with the default PricingConfig, ignoring a configured [pricing] snapshot pin"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/claude-cli"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK47 gap-62e1b9)"
discovered_from = "gap-62e1b9"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::priced_observation", "crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent", "crates/roko-core/src/pricing_snapshot.rs::PricingConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn claude_cli_prices_at_the_configured_snapshot_pin' crates/roko-agent/ && cargo test -p roko-agent claude_cli_prices_at_the_configured_snapshot_pin"
+++

## Problem

`ClaudeCliAgent::priced_observation` (`crates/roko-agent/src/claude_cli_agent.rs:972-977`) prices every Claude CLI
model call with a hardcoded default pricing config, not the operator's configured one:

```rust
/// The usage observation of `stream_usage`, priced model by model at the
/// price snapshot of the agent's working directory (backlog 6105).
fn priced_observation(&self, stream_usage: &StreamUsage, wall_ms: u64) -> UsageObservation {
    let snapshot = PriceSnapshot::shared(&PricingConfig::default(), &self.current_dir);
```

`PricingConfig::default()` always has `snapshot: ""` (`crates/roko-core/src/pricing_snapshot.rs:58-67`, doc:
"Empty (the default) means unset: a run prices from the newest snapshot in `config/prices/`, else the built-in
copy"). So a `[pricing] snapshot = "prices-2026-09-28"` set in `roko.toml` to pin a specific dated snapshot (for
reproducibility, or because a newer snapshot has bad data) has no effect on Claude CLI dispatches: this path
always re-resolves "newest file in `config/prices/`, else builtin" relative to `self.current_dir` (the agent's
working directory — an attempt's worktree, not necessarily the workspace root), ignoring the pin entirely.
`ClaudeCliAgent` (struct at line 353) has no `PricingConfig`/`pricing` field at all — it was never given the
configured value to use.

## Why it matters

Pinning a snapshot (decision/task 2113, `config/prices/<date>.toml`, covered by earlier batches' gap-ceffb3/
bug-1bc222 pricing work) only matters if every pricing path actually honors the pin. Claude CLI is one of the most
common dispatch paths (most Graph runs execute through it, per other items' notes this batch), so its cost figures
can silently diverge from a pinned snapshot the operator relied on for reproducible cost comparisons across runs,
and from whatever other provider paths DO honor the configured value.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::priced_observation` (line 976) — the call site.
- `crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent` (struct, line 353) — has no field to carry the real
  config.
- `crates/roko-core/src/pricing_snapshot.rs::PricingConfig`/`PriceSnapshot::shared` — the real config type and
  resolver; `snapshot_id()` (line 70) is the accessor that should be consulted instead of the default.
- Whatever constructs `ClaudeCliAgent` (its `new()`, same file, ~line 383) is where the real `PricingConfig` needs
  to be threaded in from the dispatch/factory layer that holds the full `RokoConfig`.

## Current state

Unfixed. `PriceSnapshot::shared` is called with a fresh `PricingConfig::default()` every time, so any configured
`[pricing] snapshot` id is invisible to this path. The doc comment names this as deliberate ("backlog 6105"), but
deliberate-at-the-time does not mean still correct now that `[pricing].snapshot` exists as a real, honored-elsewhere
config knob.

## Plan

1. Add a `pricing: PricingConfig` (or just `snapshot_id: Option<String>`) field to `ClaudeCliAgent`, populated at
   construction from the real `RokoConfig.pricing` wherever the agent is built (the dispatch/factory call site
   that already has the full config in scope).
2. Change `priced_observation` to pass `&self.pricing` (or equivalent) instead of `&PricingConfig::default()`.
3. Regression test: construct an agent with a `PricingConfig { snapshot: "<a specific, non-newest snapshot id>" }`
   and confirm `priced_observation`'s resulting `price_snapshot_id` matches it rather than the newest file's id.

## Done when

- A `ClaudeCliAgent` built with a pinned `[pricing] snapshot` prices its calls at that snapshot, not the newest
  file in `config/prices/`.
- The `[[verify]]` command passes.

## Notes

- Keep the "newest, else builtin" fallback for when `[pricing] snapshot` is unset — only the *pinned* case is
  broken today.

## Progress

- Implemented at ea1559fe2. `ClaudeCliAgent` gained a `pricing: PricingConfig` field
  (`with_pricing()` builder); `priced_observation` now calls `PriceSnapshot::shared(&self.pricing,
  ...)` instead of `&PricingConfig::default()`. `create_agent_for_model` (roko-agent's one factory
  with the full `RokoConfig` in scope) copies `config.pricing` onto a new
  `AgentOptions.pricing` field, alongside the `safety_layer`/`temperament` fields it already
  populates there; `ClaudeCliAdapter::create_agent` threads it through
  `.with_pricing(options.pricing.clone())`. New test
  `claude_cli_prices_at_the_configured_snapshot_pin`: two snapshot files in a temp workspace (an
  older pinned one at $1/M, a newer one at $1000/M, 1000x apart so a wrong resolution could not
  pass by accident); confirms the pinned one prices the call. Verify's static grep passes; `cargo
  test` deferred to the coordinator's gate.
