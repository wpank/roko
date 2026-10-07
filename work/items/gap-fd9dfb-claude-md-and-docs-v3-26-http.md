+++
id = "gap-fd9dfb"
kind = "gap"
title = "CLAUDE.md and docs/v3/26-HTTP-API.md don't document roko safety release or /api/safety/controls"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK01 gap-625195)"
discovered_from = "gap-625195 (backlog tasks 1104-1106)"
anchors = ["CLAUDE.md", "docs/v3/26-HTTP-API.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'roko safety release' CLAUDE.md && grep -q 'controls' docs/v3/26-HTTP-API.md"
+++

## Problem

Backlog tasks 1104-1106 (part of `gap-625195`, implemented at commits `220045bcf`/`80ab4098c`/`efe7dc553`) added
a real CLI command and a real REST route for releasing an immune isolation control, but neither is documented:

- CLI: `roko safety release` (`crates/roko-cli/src/commands/safety.rs::SafetyCmd::Release`, doc comment "Release
  one agent's isolation control and record who did it and why"), alongside `roko safety controls`. `CLAUDE.md`'s
  CLI command reference tables (Core workflow, Agents, Configuration, Server & deployment, Utilities, etc.) have
  no `roko safety` row at all.
- REST: `GET /api/safety/controls` and `POST /api/safety/controls/{agent_id}/release`
  (`crates/roko-serve/src/routes/safety.rs:7,9,44`, handler `release_control_handler`). `docs/v3/26-HTTP-API.md`
  documents `GET /api/safety/quarantine` and `GET /api/safety/incidents` (same file, around line 853-854) but has
  no entry for `controls` or `controls/{agent_id}/release`.

## Why it matters

Both docs are the operator-facing reference for exactly this class of action (reviewing and releasing safety
containment). Missing them means an operator who needs to release a wrongly-isolated agent has no documented way
to discover the command or route exists, even though both are implemented and tested
(`release_route_clears_isolation_control` in `safety.rs`).

## Where

- `CLAUDE.md` — the CLI commands reference tables.
- `docs/v3/26-HTTP-API.md` — the HTTP route reference, near its existing `/api/safety/*` rows (~853-854).
- `crates/roko-cli/src/commands/safety.rs::SafetyCmd` (`Controls`, `Release` variants).
- `crates/roko-serve/src/routes/safety.rs` (route registration ~:44, handler `release_control_handler` ~:324).

## Current state

Checked at HEAD (2026-10-02): `grep -n safety CLAUDE.md` matches only prose about the safety *layer*
(`crates/roko-agent/src/safety/`), never the `roko safety` command. `grep -n 'safety\|release'
docs/v3/26-HTTP-API.md` matches only `quarantine` and `incidents`, never `controls` or `controls/.../release`.

## Plan

1. Add a `roko safety controls` / `roko safety release` row to `CLAUDE.md`'s CLI reference (the table under
   "Agents" or a new small "Safety" table), describing what each does per `commands/safety.rs`'s own doc comments.
2. Add `GET /api/safety/controls` and `POST /api/safety/controls/{agent_id}/release` rows to
   `docs/v3/26-HTTP-API.md` next to the existing `/api/safety/quarantine` and `/api/safety/incidents` rows,
   describing the request/response shape from `safety.rs`'s module doc comment (lines 1-11) and
   `release_control_handler`.

## Done when

- `CLAUDE.md` names `roko safety release` (and `roko safety controls`).
- `docs/v3/26-HTTP-API.md` documents both safety-controls routes.
- The `[[verify]]` command passes.

## Notes

- Docs-only; don't change the CLI or route code to close this item.

2026-10-03 (wave-6 follow-up, PK35/gap-943046): the same shape of gap, a different command: CLAUDE.md's CLI
reference tables also have no row for `roko learn patterns` (`crates/roko-cli/src/commands/learn.rs:118`,
`LearnCmd::Patterns` — this one is real and already shipped, just undocumented, unlike the next item). PK35 also
flagged `roko plan run --no-holdout` as missing from the docs, but that flag does not exist yet in the CLI
(`grep -rn 'no.holdout' crates/roko-cli/src/main.rs` matches nothing; only an internal `no_holdout: bool` field
in `commands/plan.rs`/`run_cmd.rs`, hardcoded `false`) — that's tracked separately as `gap-29fe0a` (no
`--no-holdout` flag despite D7's decision); document it here only once that lands.

2026-10-03 (filer, same PK35/gap-943046 report): the third missing piece is `[experiments]`
(`crates/roko-core/src/config/experiments.rs::ExperimentsConfig`, wired at `schema.rs:207-209`,
decision 4115) — real and shipped, like `roko learn patterns`, just absent from the config-schema
docs table alongside `[pricing]`/`stream_usage` (gap-d2c64f) and the CLAUDE.md CLI table this item
already tracks.
