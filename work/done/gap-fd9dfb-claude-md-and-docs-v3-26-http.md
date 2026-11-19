+++
id = "gap-fd9dfb"
kind = "gap"
title = "CLAUDE.md and docs/v3/26-HTTP-API.md don't document roko safety release or /api/safety/controls"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs"]
created = 2026-10-02
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "70966c554"
source = "backlog wave reports 2026-10-02 (PK01 gap-625195)"
discovered_from = "gap-625195 (backlog tasks 1104-1106)"
anchors = ["CLAUDE.md", "docs/v3/26-HTTP-API.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["find-d05b5e", "gap-171238", "gap-2c1b76"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'roko safety release' CLAUDE.md && grep -q 'roko safety controls' CLAUDE.md && grep -qF '`/api/safety/controls`' docs/v3/26-HTTP-API.md && grep -qF '/api/safety/controls/{agent_id}/release' docs/v3/26-HTTP-API.md"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T16:27:42Z"
commit = "70966c554"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T16:06:46Z"
forced = false
evidence = "70966c554. CLAUDE.md: the Utilities row `roko safety controls/release` (c1a537d2e), which predated tool controls (gap-a2787f) and find-d05b5e and lacked the string the verify looks for, is now two rows: `roko safety controls` (agents denied before dispatch, tools refused in the workspace) and `roko safety release <agent> --reason <why>` (an agent control, or a tool control with --tool/--scope, with an audit line carrying by, reason and via; only a person releases one: the default bash policy refuses the command and it exits 1 under the variables roko sets on its agents and checks). docs/v3/26-HTTP-API.md section 8.27, renamed Safety Observability and Controls, documents GET /api/safety/controls (ledger, total, controls, reason, each control field, the 7-day expiry of decision 1107, older controls with no expiry, 500 on an unreadable ledger) and POST /api/safety/controls/{agent_id}/release (reason body, the authenticated principal or local-api, permission config:edit with agent tokens refused, a percent-encoded slash in the id, the audit line and its via, the response, 404 and 400), checked against routes/safety.rs, route_permissions.rs, rbac_middleware.rs, middleware.rs, commands/safety.rs, immune_evidence.rs and safety/bash.rs; a paragraph after the table points to the CLI for tool controls, which have no route. The verify now checks both CLI rows and both route paths (its old second half, grep -q controls, matched any use of the word): it fails on main 754f45ebd and passes at 70966c554. Docs only, no cargo run. Filed gap-171238 (docs/v3/28-CLI.md has no section for roko safety, effects, showcase, audit or demo) and gap-2c1b76 (the serve scope guards check hand-kept lists, so eleven mutating routes, the safety release among them, resolve to write:unclassified)."
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

Checked again 2026-10-26 at 754f45ebd. `c1a537d2e` (gap-07c4a4, 2026-10-16) added one
`roko safety controls/release` row to CLAUDE.md's Utilities table. It does not contain the string
`roko safety release`, so the verify still failed, and it predates two changes it does not describe:
gap-a2787f (tool controls in `.roko/immune/tool-controls.json`: `roko safety controls` lists them too, and
`roko safety release --tool <tool> --scope <scope>` releases one) and find-d05b5e (only a person releases a
control: the default bash policy refuses the command in an agent's `bash` and `run_tests` calls, the command
exits 1 under the variables roko sets on its agents and checks (`roko_started_markers`), and both release audit
files record `via`). `docs/v3/26-HTTP-API.md` §8.27 still had no `controls` route. The serve routes list and
release agent controls only; tool controls have no route. The release route needs `config:edit` (owner and admin
roles; the launch token counts as admin), and agent tokens fail closed there.

## Plan

1. Add a `roko safety controls` / `roko safety release` row to `CLAUDE.md`'s CLI reference (the table under
   "Agents" or a new small "Safety" table), describing what each does per `commands/safety.rs`'s own doc comments.
2. Add `GET /api/safety/controls` and `POST /api/safety/controls/{agent_id}/release` rows to
   `docs/v3/26-HTTP-API.md` next to the existing `/api/safety/quarantine` and `/api/safety/incidents` rows,
   describing the request/response shape from `safety.rs`'s module doc comment (lines 1-11) and
   `release_control_handler`.

## Done when

- `CLAUDE.md` has a `roko safety controls` row and a `roko safety release` row that cover tool controls and
  find-d05b5e's rules.
- `docs/v3/26-HTTP-API.md` documents `GET /api/safety/controls` and `POST /api/safety/controls/{agent_id}/release`
  (body, response, permission, status codes) and points to the CLI for tool controls.
- The `[[verify]]` command passes. It checks both CLI rows and both route paths; until 2026-10-26 its second half
  was `grep -q 'controls' docs/v3/26-HTTP-API.md`, which any use of the word would satisfy.

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

2026-10-15 (status-quo assessment at 7b6a5af44, finding 38; tmp/status-quo/2026-10-15/backlog/A01-11-12-safe-runs.md line 122): still open: CLAUDE.md's CLI reference has no `roko safety controls` or `roko safety release` row (grep finds no 'roko safety' in CLAUDE.md at b42578bab). The same finding's other stale CLAUDE.md line (the Gates row calling `run_gate_once` the per-task entry point) is filed as a new docs item; whoever takes either can do both CLAUDE.md edits in one commit.

2026-10-26: the side gaps noted on 2026-10-03 are all fixed elsewhere: CLAUDE.md has a `roko learn patterns` row,
`roko plan run --no-holdout` exists (gap-29fe0a) and has its CLAUDE.md mention, and `[experiments]` is in
`docs/v3/depth/21-config/01-schema-sections.md` (gap-d2c64f, `e0f6466b4`). The Gates row was fixed in
`c1a537d2e`.
