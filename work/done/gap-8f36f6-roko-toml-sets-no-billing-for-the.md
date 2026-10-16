+++
id = "gap-8f36f6"
kind = "gap"
title = "roko.toml sets no billing for the Claude and Codex CLI providers, so their attempts' billed_usd is unknown: mark both subscription (dec-55532d)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko.toml"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "398212ed3"
source = "status-quo-2026-10-15#decision-3"
anchors = ["roko.toml", "crates/roko-core/src/config/provider.rs::ProviderBilling", "crates/roko-core/src/config/provider.rs::ProviderConfig"]
links = { depends_on = [], blocks = [], related = ["dec-55532d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import sys, tomllib; p = tomllib.load(open(\"roko.toml\", \"rb\"))[\"providers\"]; sys.exit(0 if all(p[n].get(\"billing\") == \"subscription\" for n in (\"claude_cli\", \"codex_cli\")) else 1)'"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:57:59Z"
commit = "398212ed3"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:07Z"
forced = false
evidence = "roko.toml [providers.claude_cli] and [providers.codex_cli] now set billing = \"subscription\" (dec-55532d), each with a one-line comment citing the decision. Verify command (tomllib check on both providers) passes. `roko config validate` (prebuilt target/debug/roko) still reports 0 errors, 1 pre-existing unrelated warning (groq API key env unset). Commit 398212ed3."
+++

## Problem

This repo's `roko.toml` sets no `billing` for its two CLI providers (`[providers.claude_cli]` at :36 and
`[providers.codex_cli]` at :43), so every Claude CLI and Codex CLI attempt records `cost.billed_usd` as unknown.
`[providers.<name>] billing = "subscription" | "metered"` exists since PK13 (`crates/roko-core/src/config/provider.rs::
ProviderConfig`, `billing`; `ProviderBilling`, decision 2113): `subscription` makes an attempt's `billed_usd` 0, unset
leaves it unknown.

## Why it matters

Will's decision 3 (2026-10-15) accepted dec-55532d's choices as shipped and its point 6: set `billing =
"subscription"` for the Claude and Codex CLI providers. Both log in with a subscription on this machine. Honest cost
records need the field; NEW-FINDINGS finding 23 (Claude CLI attempts with `billed_usd` null) is partly this.

## Where

- `roko.toml` `[providers.claude_cli]`, `[providers.codex_cli]`.
- `crates/roko-core/src/config/provider.rs::ProviderBilling` (the accepted values).

## Current state

Neither section has `billing`. The global `~/.roko/config.toml` is not part of this item.

## Plan

1. Add `billing = "subscription"` to both sections, with a one-line comment citing dec-55532d.
2. Check the file still loads (`roko config validate`, or the config tests that parse the repo's `roko.toml`).

## Done when

The `[[verify]]` command passes.

## Notes

- Only the repo's `roko.toml`. Never use `roko config set` for this: it writes `~/.roko/config.toml` unless `--project`
  (memory: roko config set is global).
- Finding 23's other half (`cost_source = unknown` while cost rows say `priced = true`, helper calls counted as a known
  $0) belongs to another item.
