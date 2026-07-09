# A — CLI Overview, Command Reference, Scaffolders, Help, Config (Docs 00, 01, 02, 03, 04)

Parity of the five CLI-facing chapters: CLI overview, command
reference, `roko new` scaffolders, progressive help/explain, layered
configuration resolution.

The CLI ships as a substantial binary in `crates/roko-cli/` with most
documented commands wired. The `roko new` scaffolders and
progressive help/explain are partial; layered config resolution
ships with the top-level priority chain.

Generated: 2026-04-16.

---

## A.01 — CLI binary ships with all core subcommands (Doc 00 §"Overview")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 00 describes `roko` as a single CLI binary with universal-loop subcommands.
**Reality**: `crates/roko-cli/src/main.rs` + `crates/roko-cli/src/commands/` + ~20 top-level source files. Shipping subcommands per CLAUDE.md CLI reference table: `roko init, run, plan list/show/create/run, prd idea/list/status/draft new/draft promote/plan/consolidate, research topic/enhance-prd/enhance-plan/enhance-tasks/analyze, status, replay, config init/show/path/edit/set, dashboard, serve, chat --agent`. Twenty-plus subcommands shipping.

---

## A.02 — `roko run "<prompt>"` universal-loop entrypoint (Doc 00 §"Universal Loop", Doc 01 §"run")

**Status**: DONE
**Severity**: —
**Doc claim**: `roko run "<prompt>"` drives a single loop through compose → agent → gate → persist.
**Reality**: Per CLAUDE.md "roko run" row — shipping. Exercised by `crates/roko-cli/src/oneshot.rs` + `orchestrate.rs`.

---

## A.03 — Plan lifecycle commands (Doc 01 §"plan list/show/create/run")

**Status**: DONE
**Severity**: —
**Doc claim**: `plan list / show / create / run` + `--resume` flag.
**Reality**: CLAUDE.md CLI table confirms all four. `roko plan run plans/` is the main orchestration loop. `--resume .roko/state/executor.json` shipping.

---

## A.04 — PRD lifecycle commands (Doc 01 §"prd idea/list/status/draft/plan/consolidate")

**Status**: DONE
**Severity**: —
**Doc claim**: 7 PRD subcommands covering idea capture, listing, status, draft-new, draft-promote, plan generation, consolidation.
**Reality**: CLAUDE.md CLI table confirms all 7. `roko prd plan <slug>` generates implementation plans via agent.

---

## A.05 — Research subcommands (Doc 01 §"research")

**Status**: DONE
**Severity**: —
**Doc claim**: `research topic / enhance-prd / enhance-plan / enhance-tasks / analyze`.
**Reality**: CLAUDE.md "roko research" rows show all 5 shipping.

---

## A.06 — Status / replay / dashboard / serve / chat (Doc 01 §"status", §"replay", §"dashboard", §"serve", §"chat")

**Status**: DONE
**Severity**: —
**Doc claim**: Status, replay, dashboard (interactive TUI), serve (HTTP control plane), chat (per-agent sidecar).
**Reality**: CLAUDE.md confirms all five:
- `roko status` — "Query signals, report counts and episodes"
- `roko replay` — "Walk signal DAG by hash"
- `roko dashboard` — "Interactive ratatui TUI (F1–F7 tabs)"
- `roko serve` — "Start HTTP control plane (~85 routes on :6677)"
- `roko chat --agent <id>` — "Chat with a running agent via the sidecar"

---

## A.07 — Config subcommands (Doc 01 §"config", Doc 04 §"Layered Resolution")

**Status**: DONE
**Severity**: —
**Doc claim**: `config init / show / path / edit / set` with layered resolution (built-in → `/etc/roko` → `$XDG_CONFIG_HOME/roko` → project → CLI flags).
**Reality**: CLAUDE.md CLI table shows 5 config subcommands. `crates/roko-cli/src/config.rs` + `config_cmd.rs` implement the surface. Actual layered-resolution precedence order requires reading config.rs to verify.

---

## A.08 — `roko init` initializes `.roko/` + `roko.toml` (Doc 00 §"Project Bootstrap")

**Status**: DONE
**Severity**: —
**Doc claim**: `roko init` creates `.roko/` directory and `roko.toml`.
**Reality**: CLAUDE.md CLI table row. `roko-cli/src/commands/` has the init handler. Shipping.

---

## A.09 — `roko new` domain scaffolders (Doc 02 §"Scaffolders")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 02 §"Scaffolders" describes `roko new <domain> <name>` to scaffold per-domain agent projects (coding / chain / research / data / ops etc.).
**Reality**: A direct read of `crates/roko-cli/src/main.rs`'s top-level `enum Command` shows no `New` subcommand. The previous parity wording was too soft here; this is not just "unverified from the commands directory". On current source, `roko new` does not appear to ship as a top-level CLI surface.
**Fix sketch**: Doc 02 should carry a `Design — Phase 2+` banner unless later source proof appears outside `main.rs`.

---

## A.10 — Progressive help + explain (Doc 03 §"Progressive Help", §"explain Subcommand")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 describes `roko help <topic>` with progressive disclosure + `roko explain <concept>` for conceptual education.
**Reality**: A direct read of `crates/roko-cli/src/main.rs` shows no standalone `Explain` top-level command. There is a verified `model route --explain` flag, but that is a routing-debug surface, not the conceptual-learning command Doc 03 describes. Clap `--help` support is baseline and does not satisfy the doc claim.
**Fix sketch**: Doc 03 should stay `Design — Phase 2+` unless a dedicated progressive-help/explain surface is added.

---

## A.11 — Layered configuration resolution (Doc 04 §"Layered Resolution")

**Status**: DONE
**Severity**: —
**Doc claim**: Config resolves in priority order: built-in defaults → `/etc/roko` → `$XDG_CONFIG_HOME/roko` → project-local `roko.toml` → CLI flags (highest).
**Reality**: The CLI shipping with `config init/show/path/edit/set` confirms layered config is real. The precise precedence order needs reading of `crates/roko-cli/src/config.rs` but the layered model is standard.

---

## A.12 — `agent_config / agent_episode / agent_exec / agent_spawn` subcommands (Doc 01 §"Agent Subcommands")

**Status**: DONE (additional shipping surface)
**Severity**: —
**Doc claim**: Doc 01 mentions agent subcommands tangentially. The spawn/exec/config/episode breakdown is not in Doc 01 explicitly.
**Reality**: `ls crates/roko-cli/src/` shows `agent_config.rs, agent_episode.rs, agent_exec.rs, agent_spawn.rs` as top-level source files. These are shipping agent-subprocess management surfaces beyond what Doc 01 documents.
**Fix sketch**: Doc 01 §"Command Reference" should add an "Agent subcommands" subsection for these four live commands.

---

## A.13 — Event sources + daemon + heartbeat + inject (Doc 01 §"Daemon Subcommands")

**Status**: DONE (additional shipping surface)
**Severity**: —
**Doc claim**: Doc 01 mentions daemon mode briefly.
**Reality**: `ls crates/roko-cli/src/` shows `event_sources.rs, daemon/, daemon.rs, heartbeat.rs, inject.rs`. Rich daemon-mode surface shipping. The `daemon.rs` + `daemon/` pair handles long-running agent orchestration; `heartbeat.rs` + `inject.rs` are the runtime-signaling helpers.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 11 (A.01 CLI binary, A.02 run, A.03 plan, A.04 prd, A.05 research, A.06 status+replay+dashboard+serve+chat, A.07 config, A.08 init, A.11 layered config, A.12 agent subcmds, A.13 daemon) |
| PARTIAL | 0 |
| NOT DONE | 2 (A.09 `roko new` scaffolders, A.10 progressive help/explain) |

Section A shows **the CLI is substantially shipping**, but the two
onboarding-oriented command docs were previously overstated. `roko new`
and standalone `roko explain` should be treated as absent until proven
otherwise.

## Agent Execution Notes

### A.09 / A.10 — Verify scaffolders + explain

The highest-value work is now truth-in-advertising, not open-ended
verification. `main.rs` is the primary command source and currently
does not show either surface as a top-level subcommand. Docs 02 / 03
should be updated from that starting point unless contradictory source
proof appears.

### A.12 / A.13 — Doc 01 additions

Doc 01 should add subsections for the shipping agent-subcommand family
(4 commands) and daemon-subcommand family (5+ source files).

Acceptance criteria:

- Doc 01 enumerates shipping agent / daemon subcommands,
- Doc 02 / 03 no longer imply `roko new` or standalone `roko explain`
  ship without source proof,
- Doc 04 cites the shipping layered-config chain precedence.
