# Self-Developing UX Crosswalk

Source: `tmp/solutions/self-developing/00-INDEX.md`, dated 2026-05-06 (23 UX issues + a "Fixes Applied This Session" log + a `roko develop` core thesis). These are product-direction issues for making `roko develop "..."` feel like the default self-development loop.

> Status-quo audit · re-verified against code at HEAD `5852c93c05` on 2026-07-08. Prior version (2026-07-07) treated `roko develop` as unbuilt product direction; that is now **stale** — see the headline change below.

## Headline change since last audit: `roko develop` shipped

Issue 07 (`roko develop` — "the one command that does everything") is **no longer open product direction; the command exists.**

- Registered in `crates/roko-cli/src/main.rs:384-387` (help text: `roko develop "Add user auth"`, `--dry-run`, `--yes`, `--continue`) and dispatched at `main.rs:2399` → `commands::develop::cmd_develop`.
- Implementation: `crates/roko-cli/src/commands/develop.rs` (211 lines). It is a **thin wrapper over `do_cmd` with `--plan` forced**: dry-run preview → interactive task-table approval (`show_plan_approval`) → execute → `roko dashboard` hint on TTY success. `--continue` delegates to `do_cmd --continue`.
- **Load-bearing caveat**: `develop.rs` does not pin an engine; it inherits whatever `do_cmd`/`plan run` resolves. Because the clap default is `--engine graph` (`main.rs:1361`) whose `TaskExecutorCell` live dispatch is still a dry-run stub (`crates/roko-graph/src/cells/task_executor.rs:80-92`), `roko develop` can appear to "succeed" while dispatching no real agent work on the graph path. **The crosswalk guidance "do not implement `roko develop` until plan default is honest" was inverted in practice: it shipped first.**

So the correct disposition for issue 07 is now **Shipped-but-rides-stubbed-path**, and the P0 is no longer "spec it" — it is "make the engine `roko develop` inherits honest" (i.e. the same P0 as 21-TMP-MAY-BATCH's TaskExecutorCell item).

## Issue Map (re-verified)

| # | Source issue | Source status | Current classification | Owner |
|---|---|---|---|---|
| 01 | Model config UX | Open | Open. Config/env/provider discovery remains complex. | [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md) |
| 02 | Plan generation UX | Open | Open/partial. Plan generation exists; validation/default engine semantics still risky (graph-default stub). | [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md) |
| 03 | CLI noise | Open | Open/partial. Raw output/log noise still a cleanup item. | [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md) |
| 04 | Zero-knowledge onboarding | Open | Product strategy, not migration blocker. `roko develop` narrows the gap but onboarding still assumes config knowledge. | [65-DOCS-CONVERGENCE-PLAN.md](65-DOCS-CONVERGENCE-PLAN.md) |
| 05 | Idea-to-execution flow | Open | Open. `roko develop` collapses the manual steps into one command, but honest execution still depends on the engine fix. | [27-IMPLEMENTATION-BACKLOG.md](27-IMPLEMENTATION-BACKLOG.md) |
| 06 | Error recovery | Open | Open/partial. Recovery classifiers exist in places but are not the default UX loop. | [36-ORCHESTRATION-RUNNERS.md](36-ORCHESTRATION-RUNNERS.md) |
| 07 | `roko develop` spec | Open | **SHIPPED (rides stubbed graph path).** `develop.rs` exists; wraps `do_cmd --plan` + approval + dashboard hint. Real-work honesty gated on TaskExecutorCell. | [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md), [21-TMP-MAY-BATCH.md](21-TMP-MAY-BATCH.md) |
| 08 | Model discovery ergonomics | Open | Open/partial. Needs model list/fuzzy/completion proof. | [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md) |
| 09 | Unified CLI UX (`note`/`plan`/`do`) | Open | Partial. `roko do`/`develop` exist as the unified verbs; command proliferation not yet reversed. | [62-CLI-COMMAND-LEDGER.md](62-CLI-COMMAND-LEDGER.md) |
| 10 | Terminal output corruption | Open | Open/partial. TUI/terminal surfaces exist but output contracts need proof. | [43-SURFACES-DEMO-UX.md](43-SURFACES-DEMO-UX.md) |
| 11 | Context sources/editor integration | Open | Partial. ACP exists; context source UX remains open. | [51-ACP.md](51-ACP.md) |
| 12 | ACP Zed errors | Partial | Partial. ACP is real; capability/error truth still open. | [51-ACP.md](51-ACP.md) |
| 13 | Config should not exist | Open | Product direction. Current system still requires explicit config for many paths. | [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md) |
| 14 | Image support | Partial | Partial. `resource_link`+`Image` added to ContentBlock; ACP reports `image: true` (`handler.rs:287`). Needs provider/surface test matrix. | [38-AGENT-PROVIDERS-TOOLS.md](38-AGENT-PROVIDERS-TOOLS.md) |
| 15 | Bare mode kills commands | Fixed | Treat fixed unless regression tests fail. | [51-ACP.md](51-ACP.md) |
| 16 | Resource link crash | Fixed | Treat fixed unless ACP protocol tests fail. | [51-ACP.md](51-ACP.md) |
| 17 | Decision provenance noise | Fixed | Treat fixed; watch UX regressions. | [51-ACP.md](51-ACP.md) |
| 18 | Slash command no streaming | Fixed | Source says fixed; current streaming parity still needs tests. | [51-ACP.md](51-ACP.md) |
| 19 | ACP model has no tools | Open | **Likely mostly-addressed.** `crates/roko-acp/src/builtin_tools.rs` (810 LOC, 84 fn/Tool defs) exists. Re-classify to Partial: tool *execution* + permission parity still needs live proof. | [51-ACP.md](51-ACP.md), [33-AGENT-SAFETY.md](33-AGENT-SAFETY.md) |
| 20 | Learning not wired in ACP | Open | Partial. `roko-acp/src/{session,runner,bridge_events}.rs` reference CascadeRouter/EpisodeLogger/dream/experiment symbols; parity depth (does ACP actually persist episodes + route?) unproven. | [40-LEARNING-TELEMETRY.md](40-LEARNING-TELEMETRY.md) |
| 21 | Cross-provider cascade error | Open | Open/needs proof. Model-not-forwarded-to-CLI bug not confirmed fixed. | [38-AGENT-PROVIDERS-TOOLS.md](38-AGENT-PROVIDERS-TOOLS.md) |
| 22 | Plan-run TUI broken | Partial | Still tied to Graph default/stale snapshot/hidden stderr issues — **directly caused by the graph-default stub**; this is the user-visible face of the issue-07 caveat. | [37-RUNNER-V2-AND-GRAPH.md](37-RUNNER-V2-AND-GRAPH.md), [43-SURFACES-DEMO-UX.md](43-SURFACES-DEMO-UX.md) |
| 23 | TUI plan list scroll | Open | UI polish; not migration-critical. | [43-SURFACES-DEMO-UX.md](43-SURFACES-DEMO-UX.md) |

## Design-intent adoption ledger (source thesis → code)

The source doc's "Core Thesis" is that self-development should feel like talking to a colleague. Mapping the concrete asks:

| Design intent (00-INDEX thesis / fixes) | Adopted? | Evidence |
|---|---|---|
| One command does everything (`roko develop`) | **Yes (surface)** | `commands/develop.rs`; `main.rs:2399` |
| Plan-first with human approval gate | **Yes** | `show_plan_approval()` in `develop.rs:155` |
| Auto-launch/hint dashboard on success | **Yes** | `hint_tui_dashboard()` `develop.rs:208` |
| `--dry-run` plan preview before executing | **Yes** | `develop.rs:52-69` |
| `--continue` resume from last snapshot | **Yes (delegated)** | `develop.rs:26-43` → `do_cmd --continue` |
| ACP gains real tools (not pure chat) | **Partial** | `roko-acp/src/builtin_tools.rs` 810 LOC |
| ACP image capability | **Yes** | ContentBlock Image + `handler.rs:287` |
| Model discovery / fuzzy / `roko models` | **Unverified** | no proof gathered |
| "Config should not exist" auto-inference | **No** | explicit-provider doctrine still requires config |
| Honest execution on the default path | **No — P0** | TaskExecutorCell stub `task_executor.rs:80-92` |

## Adoption Guidance (updated)

- **`roko develop` is now real.** Stop treating it as spec-only. The remaining work is not the command — it is making the engine it inherits (graph default) actually dispatch agents, OR pinning `roko develop`/`do` to `runner-v2` until `TaskExecutorCell` live dispatch lands. This is the single highest-leverage fix for issues 05, 07, and 22 simultaneously.
- Treat UX/gateway designs beyond `develop` as product strategy unless a roadmap item explicitly adopts them.
- Promote issues 01, 08, 09, 13, and 19-tool-execution into the roadmap after the P0 engine fix.
- Keep fixed ACP issues (15-18) covered by regression tests, not memory.
- Re-run issue 19/20 as live proofs: builtin_tools *existing* is not the same as ACP tool *execution + permission enforcement* working end-to-end.

## Cross-cutting drift for the navigation layer

- CLAUDE.md's self-hosting workflow (steps 1-8) still lists the old manual `prd idea → prd draft → research → prd plan → plan run` chain and never mentions `roko develop`, which now collapses most of that into one command. Nav docs should surface `roko develop` as the primary self-dev entry point.
- CLAUDE.md is at "18 crates"; the tree is **31 crates** (roko-acp/demo/plugin/graph + mcp-* not listed). See 21-TMP-MAY-BATCH checklist P2.
