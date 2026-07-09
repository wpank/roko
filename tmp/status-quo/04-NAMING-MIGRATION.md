# Naming & Convention Migration

> Status-quo audit · **re-verified 2026-07-08 @ git HEAD `5852c93c05` (main)** · supersedes earlier draft (which declared the migration "essentially complete" — it is not) · sources: fresh grep census over `crates/` (31 crates), `apps/` (3), `docs/`, `.roko/`; cross-checked against 30-CORE-SIGNAL, 18-V2-DEPTH-COVERAGE, 35-GATES-VERIFICATION, 44-AGENT-SERVER, 48-MCP-CRATES, 26-CANONICAL-DECISIONS (D3/D5/D6). All paths relative to `/Users/will/dev/nunchi/roko/roko/`.
>
> **2026-07-08 re-verification note**: re-ran the full `rg --type rust` census at HEAD `5852c93c05`. Per-crate `Engram` counts (crates only, excluding target/): **roko-core 268, roko-cli 195, roko-conductor 128, roko-serve 75, roko-std 60, roko-chain 43, roko-fs 29, roko-graph 24, roko-agent 17, roko-plugin 14, roko-neuro 12, roko-dreams 12, roko-acp 6, roko-runtime 2**; apps: **mirage-rs 48, roko-chain-watcher 1**. That's the authoritative per-crate table for the flip's blast radius (supersedes the ~885 aggregate — the sum across these is now **~945 in crates + 49 in apps**). `Signal`-type vocabulary is inverse: **roko-agent 251, roko-gate 178, roko-compose 56, roko-learn 46, roko-cli 37, roko-core 17, roko-orchestrator 9, roko-serve 8**. Two live surfaces spot-confirmed unchanged: agent-server `"engram_id": format!("engram-{}", Uuid::new_v4())` (`messaging.rs:57` — fabricated, not persisted), and `roko init` still renames `signals.jsonl`→`engrams.jsonl` (`util.rs:135-140`). Ground truth in `.roko/`: `signals.jsonl` 467 lines (raw gate-verdict JSON), `engrams.jsonl` 10 lines (Engram-shaped), plus a **44 MB `events.jsonl` firehose** that neither the noun-flip nor the storage doc previously tracked — see §Data-file addendum.

## Summary

The v1→v2 naming migration finished the **traits** and stalled on the **noun and the storage layer**. The six verb traits are truly renamed — `Store, Score, Verify, Route, Compose, React` are the only trait definitions; zero `trait Scorer/Gate/Router/Composer/Policy` remain (only relic: blanket alias `pub trait Substrate: Store {}`, `crates/roko-core/src/traits.rs:428`). But the base noun is still `Engram`: **885 occurrences across 116 rust files in crates/ + 49 across 8 files in apps/**, with `Signal` a pure re-export alias (`crates/roko-core/src/signal.rs:6`) whose header promises "the full Engram→Signal rename happens in Phase 1" — a phase that never ran. The ecosystem split cleanly along the alias: roko-agent/-gate/-compose/-learn/-orchestrator speak Signal; roko-cli/-serve/-std/-fs/-graph/-neuro/-conductor speak Engram (per-crate split in 30-CORE-SIGNAL §Old-paradigm #1).

Worse, the storage layer renamed **in the opposite direction from the spec**: v2 declares `.roko/signals.jsonl` primary (`docs/v2/01-SIGNAL.md:1371`), but code labels it "legacy" (`crates/roko-fs/src/layout.rs:217`), `roko init` migrates `signals.jsonl` → `engrams.jsonl` (`crates/roko-cli/src/commands/util.rs:135-150`), and the two files have split writers/readers: the live plan runner appends raw gate-verdict JSON to signals.jsonl (`crates/roko-cli/src/runner/event_loop.rs:1147`, 467 lines on disk) while roko-serve's `/gates/*` and the TUI read engrams.jsonl (`crates/roko-serve/src/routes/status/gates.rs:84`, 10 stale lines) — dashboards silently see zero verdicts.

Old names also leak through live surfaces: the agent-server returns fabricated `"engram_id": "engram-<uuid>"` (`crates/roko-agent-server/src/features/messaging.rs:57`), the WS event filter accepts prefix `engram-stream:` (`crates/roko-serve/src/routes/ws.rs:240`), `roko new` scaffolds **non-compiling v0-era code** (`Kind::Signal` variant that doesn't exist, wrong Engram field names, v1 `Gate::check` API — `crates/roko-cli/src/scaffold.rs:129-204`), a live PRD prompt still says "the Grimoire" (`crates/roko-cli/src/prd_prompt.rs:16`), and roko-mcp-slack mixes `slack.`/`slack_` tool names (`crates/roko-mcp-slack/src/main.rs:385-393`). Meanwhile chain-era names (korai/DAEJI/ISFR) and bio names (pulse/pheromone/stigmergy/rung) are **v2-canonical, not legacy** — no action. Structurally, error handling splits thiserror-libs vs anyhow-libs with 5 mixed crates, and module layout splits mod.rs-style (56 `mod.rs` files) vs the flat style of the newest crates (roko-acp: 15 flat files, zero mod.rs).

## Term census table

Era: **v1** = legacy, rename/remove · **v2** = canonical, keep · **amb** = ambiguous/split. Counts are `--type rust` in `crates/` unless noted; "top" lists heaviest files/crates.

| Term | Era | Hits (top locations) | Canonical name | Action |
|---|---|---|---|---|
| `Engram` | v1 | **~945× crates + 49× apps** (2026-07-08 recount); per-crate: core 268, cli 195, conductor 128, serve 75, std 60, **chain 43** (incl. duplicate struct — R9), fs 29, graph 24, agent 17, plugin 14, neuro 12, dreams 12, acp 6, runtime 2 | `Signal` | Execute the flip (§Engram→Signal) — P1 |
| `Signal` (type) | v2 | Alias-only import path; Signal-vocabulary crates: roko-agent 335, roko-gate 147, roko-compose 78, roko-learn 53, roko-orchestrator 15 (30-CORE §1) | `Signal` | Keep; make it the struct |
| `engrams.jsonl` | v1 name, **live main file** | Writers: orchestrate.rs:6027,7154,8515,19536; file_substrate.rs:48,72; serve state.rs:429; TUI dashboard.rs (12+ hardcoded joins). Readers: serve status/{gates.rs:84,episodes.rs:35,metrics.rs:119}, status.rs:194, dreams cycle.rs:52 (`ENGRAMS_LOG_FILE`), dashboard_snapshot.rs:1269,2891; e2e tests | `signals.jsonl` | P0 canonicalize + migrate (§Data-file) |
| `signals.jsonl` | v2 name, labeled "legacy" in code | layout.rs:217-221; runner/event_loop.rs:1147 (writer, raw JSON); chat_inline.rs:3201, acp session.rs:1474, agent_serve.rs:1388 (readers); deprecated `Workspace::signals_path` workspace.rs:165-168 | `signals.jsonl` | Keep name; fix labels + writers |
| `engram_id` | v1 | 4 sites: agent-server messaging.rs:57 (**fake UUID**, + README.md:48), roko-chain identity_economy_markets.rs:454, roko-runtime heartbeat.rs:1828 | `signal_id` | P2 (couple to flip; agent-server needs real id — 44 P2 item) |
| `engram-stream:` (WS filter prefix) | v1 | serve routes/ws.rs:212,240 | `signal-stream:` | P2, accept both during window |
| `Substrate` (trait) | v1 | 18× / 7 files; def traits.rs:428 (kept only for `CellContext`), stale doc cell.rs:87-88, runtime_feedback/mod.rs | `Store` | P2 retire blanket trait + fix docs |
| `*Substrate` (concrete types) | amb | `FileSubstrate` (roko-fs), `MemorySubstrate` (roko-std/src/memory.rs:75), `ArchiveColdSubstrate`, mirage-rs `chain_substrate.rs`/`hdc_substrate.rs` | `*Store` | P3 cosmetic |
| `Scorer` | v1 | ~9 sites: cell.rs:87 (doc), lib.rs:310 (historical note, fine), **scaffold.rs:164 codegen**, roko-std/src/scorer.rs (filename), serve feed_agents display strings, roko-graph cell.rs:86 doc example | `Score` | P1 scaffold+kernel docs; P3 filenames |
| `Gate` | **v2-domain** | Pervasive in roko-gate as domain noun (GatePipeline, CompileGate…); 0 trait defs; all gates `impl Verify` (gate_service.rs:183 … gate_pipeline.rs:208, 15+) | `Verify` (protocol) / `Gate` (cell noun) | Keep. Fix stale `pub trait Gate` snippet in docs/v2-depth/02-block/verify-cells-and-pipeline.md:11-19 |
| `Router`/`Composer`/`Policy` (struct suffixes) | amb | `FirstRouter/HighestScoreRouter/RoundRobinRouter` impl Route (roko-std/src/router.rs:16,42,75), `NoOpGate/NoOpRouter/NoOpComposer/NoOpPolicy` (roko-std/src/noop.rs:46-99), `CascadeRouter` (roko-learn) | Route/Compose/React traits | P3 optional; decide suffix policy (Open Q3) |
| `rung` | **v2** | 258+× / 74+ files (roko-gate rung_selector.rs, config/gates.rs, orchestrate.rs, serve); v2-depth blesses "7-rung pipeline" (02-block/verify-cells-and-pipeline.md:3,33) | `rung` | Keep. Fix **semantic split**: pipeline rungs 0-6 (Compile,Lint,Test,Symbol,GenTest,PropTest,Integration) vs `GateService::rung_for_name` compile0/clippy1/test2/**diff3/fmt4/custom5** (gate_service.rs:51,392-397) — D6 |
| `verify-cell` | v2 (docs-only) | docs/v2-depth/02-block/verify-cells-and-pipeline.md | — | No code action; it's the doc name for gates |
| `mori`/`bardo`/`golem` | v1 provenance | **161× / 74 rust files** — nearly all doc-comments/ported-from citations (roko-gate, roko-compose/enrichment, roko-core phase.rs/verdict.rs/agent.rs); runtime-adjacent: phase.rs:20-21 (`.mori/state/*.json` read-compat), integration_gate.rs:4-25 (golem lifecycle), role_prompts.rs:922,1012 (tests **forbidding** mori/bardo in prompts — good) | `roko` | Keep as provenance; P3 sweep runtime-visible strings; docs/v2 leakage below |
| `grimoire` | v1 | 2 code sites: config/learning.rs:19 (doc-comment on `knowledge_warnings`), **prd_prompt.rs:16 (live agent prompt text)** | neuro / knowledge store | P2 fix prompt, P3 doc-comment |
| `styx` | removed | 0 hits | Korai | Done ✅ |
| `clade` | v1 | 1 code hit (roko-demo/src/scenarios/job_board.rs:1 comment); docs/v2/28-ROADMAP.md:304 ("Clade-Metaproductivity" — research-term, arguably intentional) | fleet (53×/10 files — adopted) | Trivial comment fix |
| `korai` / `DAEJI` / `isfr` | **v2 (chain domain)** | korai 135×/13 files (roko-chain + serve routes/jobs.rs); DAEJI ≈30× (korai_token.rs:151-152, chain_profile.rs, demo); isfr 160×/25 files (isfr_keeper/bootstrap/sources, serve routes/isfr.rs:27, std builtin isfr.rs) | keep | None — chain-era ≠ legacy. Feature-gating is 18-V2 Q4, not naming |
| `pulse` | **v2** | `Pulse` type pulse.rs:75-88 + PulseBus/pulse_bus (runtime, serve) | keep | None (v2 gaps on Pulse fields are 30-CORE's P2) |
| `pheromone` / `stigmergy` | **v2** | pheromone 409+×/25+ files (coordination.rs 180, context_provider.rs 56, system_prompt_builder.rs 32, `Kind::Pheromone`); stigmergy 13×/4 files | keep | None — v2-depth 11-memory canon |
| `Runner` / `Engine` | **v2 (both)** | `--engine runner-v2|graph` (main.rs:1302-1303 `RunnerV2`), GraphEngine (commands/graph.rs:8,62), "runner-v2" template string event_loop.rs:2806 | per engine decision | None now; revisit names after 18-V2 Open Q1 (which engine wins) |
| `dispatch_direct` | removed (chat path) | Strays: chat_inline.rs:728,735,742,4810-4818 — error strings *refusing* the "deprecated dispatch_direct path" + guard test (intentional tombstones); `dispatch_direct_hire` (roko-chain markets.rs:1045) is an unrelated marketplace fn | — | Confirmed no live path ✅; P3 reword tombstone strings once trust is established |
| `block` (v2 abstraction) | v1-of-v2 (docs dir only) | Zero code (`Block` in roko-chain/mirage = blockchain blocks, legit); dir `docs/v2-depth/02-block/` fronts for `02-CELL.md` (verify-cells-and-pipeline.md:3 links "Depth for 02-CELL.md") | `cell` | P3 rename docs dir `02-block/`→`02-cell/` or add INDEX note |
| `Kind::Signal` | **never existed** | scaffold.rs:129,200,271,331,429 — 5 codegen templates reference a nonexistent Kind variant | real `Kind::*` | P1 (scaffold emits non-compiling code) |
| `SignalKind` / `signal_kinds` | amb (parallel vocab) | `enum SignalKind` roko-agent/src/lifecycle.rs:940; `signal_kinds` string consts roko-core/src/signal_kinds.rs (lib.rs:156,273) used by serve webhooks.rs:226-277 | unify with core `Kind` | P3 (30-CORE checklist item) |
| slack `slack.` vs `slack_` | v1-drift | 4 dot-tools vs 5 underscore-tools, main.rs:385-393; plus `github.github.*` double-prefix (48-MCP §notes) | `slack.*` (`{server}.{tool}`) | P2 (48-MCP P2 item) |

**Docs leakage (v1 terms inside v2 docs)**: `Engram` — 64×/7 files in docs/v2 (**ARCHITECTURE-GUIDE.md 48**, API-REFERENCE.md 4 — including "`engrams.jsonl`" at :1080,:1141) and 11×/6 files in docs/v2-depth; `mori/bardo/golem/clade` — ≈24× in docs/v2 (27-ORCHESTRATOR.md 19 — parity-spec references, arguably intentional; 19-CONFIG.md:388 "Legacy Mori format"), ≥100×/40+ files across docs/ overall. docs/v2/CLI-REFERENCE.md:2296 documents `signals.jsonl` as the live log while API-REFERENCE.md:1141 documents `engrams.jsonl` — the spec contradicts itself file-to-file.

## Rename targets (file/type/config-key/route level)

| # | Target | Current → Canonical | Impact |
|---|---|---|---|
| R1 | `pub struct Engram`, `EngramBuilder` (`crates/roko-core/src/engram.rs:63`) | → `Signal`, `SignalBuilder`; keep `pub type Engram = Signal` deprecated alias; swap module names engram.rs/signal.rs | 885+49 refs; mechanical but wide (§flip) |
| R2 | `.roko/engrams.jsonl` | → `.roko/signals.jsonl` canonical | Touches roko-fs layout.rs:202-221, file_substrate.rs:48,72; util.rs:135-150 (init); ~25 hardcoded `join("engrams.jsonl")` in roko-cli/serve/core; e2e tests (e2e.rs:103, e2e_self_host.rs:115, ollama_e2e.rs:116-132); dreams cycle.rs:52 |
| R3 | `roko init` migration direction (util.rs:138-144 renames signals→engrams) | Invert: engrams→signals | One function + tests |
| R4 | `Layout::engrams_path_legacy`/`signals_path` (layout.rs:208-221), deprecated `workspace::Workspace` (roko-core lib.rs:314-315, workspace.rs:165-168) | Collapse to one `signals_path()`; delete Workspace | Small; unblocks D5-style path centralization |
| R5 | scaffold templates (`crates/roko-cli/src/scaffold.rs:129-204,271,331,429`) | Fix: real field names (`id/lineage/created_at_ms/Score`), drop `Kind::Signal`, v2 APIs (`Verify::verify`→Verdict, not `Gate::check`→bool), emit `Signal` | `roko new gate/scorer/...` currently generates non-compiling code |
| R6 | agent-server `"engram_id": "engram-<uuid>"` (messaging.rs:57; README.md:48) | → real persisted signal id, field `signal_id` (compat: emit both) | API consumers; pairs with 44-AGENT-SERVER P2 "persist /message turns" |
| R7 | WS filter prefix `engram-stream:` (serve ws.rs:212,240) | → `signal-stream:`, accept both ≥1 release | Frontend contract (66-FRONTEND-API-PARITY) |
| R8 | Blanket `trait Substrate: Store {}` (traits.rs:428) + stale kernel doc listing v1 trait names (cell.rs:87-88) | Delete alias (inline `Store` bound into `CellContext`); rewrite doc to Store/Score/Verify/Route/Compose/React | 7 files |
| R9 | roko-chain duplicate `Engram` struct (identity_economy_markets.rs:653, per 18-V2; `engram_id: Blake3Hash` :454) | Delete; use roko-core Signal | Divergence bomb — two structs, one name |
| R10 | v1 struct/file names in roko-std: scorer.rs, router.rs, noop.rs (`NoOpGate/Router/Composer/Policy`), memory.rs `MemorySubstrate` | → score.rs/route.rs; `NoOpVerify/NoOpRoute/NoOpCompose/NoOpReact`; `MemoryStore` | P3; public API of roko-std, semver-noise only |
| R11 | slack tool names (main.rs:385-393) + server-side pre-prefixing → `github.github.*` (48-MCP) | Uniform `slack.x`; single prefixing layer | External tool-name contract for agents |
| R12 | Config/prompt text: `prd_prompt.rs:16` "the Grimoire", `config/learning.rs:19` doc, explain.rs:147 + surface_inventory.rs:1550 (engrams.jsonl in user-facing help) | neuro/knowledge, signals.jsonl | User-visible strings |
| R13 | Docs: ARCHITECTURE-GUIDE.md (48 Engram), API-REFERENCE.md:1080,1141 (engrams.jsonl), v2-depth verify-cells `pub trait Gate` snippet, `docs/v2-depth/02-block/` dir name, docs' "roko-relay crate" vs actual `apps/agent-relay/` (18-V2 §12) | Signal / signals.jsonl / Verify impl / 02-cell / agent-relay | Doc-only |
| R14 | `Rung` semantics (gate_service.rs:51 vs 7-rung pipeline) | One canonical rung table; namespace GateService's diff/fmt/custom outside numeric rungs | D6; affects adaptive-threshold keys in `.roko/learn/gate-thresholds.json` |
| R15 | Kind vocab triple: core `Kind`, roko-agent `SignalKind` (lifecycle.rs:940), `signal_kinds.rs` consts | Single Kind lattice + string-const namespace | P3, design work (30-CORE) |

Non-targets (explicitly keep): `Gate*` cell names, `rung`, `pulse`, `pheromone`, `stigmergy`, `korai/DAEJI/ISFR`, `CascadeRouter` et al. struct suffixes (pending Q3), mori/bardo *provenance comments* in ported code.

## The Engram→Signal flip

The one rename that touches everything. Three sources currently disagree (30-CORE Open Q1): v2 spec says the struct *stays* Engram with a `type Signal = Engram` bridge (`docs/v2/01-SIGNAL.md:69`); signal.rs:1-4 promises a full Phase-1 rename; the storage layer renamed files *toward* Engram (util.rs:138). CLAUDE.md and the HTTP surface (`GET /api/signals`, serve routes/status/mod.rs:38) already speak Signal. **Recommendation (= D3): Signal wins everywhere; Engram survives one release as a deprecated alias.** Inventory of what the flip touches, in execution order:

1. **Decide D3 first** (record in 26-CANONICAL-DECISIONS) and amend `docs/v2/01-SIGNAL.md:69` so spec, code comment, and CLAUDE.md agree — otherwise the next agent re-litigates it.
2. **Storage before types** (independent, fixes live split-brain — see §Data-file): canonical file becomes signals.jsonl; serve/TUI readers repointed; runner verdict writes become Signal-shaped.
3. **Kernel rename** (roko-core, ~265 refs): `engram.rs`→`signal.rs` (struct `Signal`, `SignalBuilder`); new `engram.rs` holds `#[deprecated] pub type Engram = Signal;` + builder alias; flip lib.rs re-exports (:154-155) and crate docs lib.rs:1-19 ("universal Engram type"); rename `benches/engram_bench.rs`; update traits.rs/datum.rs/pulse.rs/loop_tick.rs/attestation.rs/prediction.rs signatures mentioning Engram (mostly doc text — the types flow through).
4. **Engram-vocabulary crates, most-isolated first** (2026-07-08 recount): roko-fs (29) → roko-std (60) → roko-plugin (14) → roko-graph (24) → roko-neuro (12) → roko-dreams (12) → roko-runtime (2) → roko-chain (43, incl. the R9 duplicate — do R9 first) → roko-conductor (128, mixed) → roko-serve (75) → roko-cli (195) → apps (mirage-rs 48, chain-watcher 1). Signal-vocabulary crates (agent 251 / gate 178 / compose 56 / learn 46 / orchestrator 9) need zero changes — proof the alias path works. roko-acp (6) rides mostly on re-exports.
5. **Wire-visible names with compat windows**: `engram_id`→`signal_id` (R6, emit both), `engram-stream:`→`signal-stream:` (R7, accept both), heartbeat.rs:1828 + chain markets.rs:454 field renames (serde alias for old field).
6. **Kill the name-collision hazards**: roko-chain duplicate `Engram` (R9) and scaffold.rs templates (R5) *before* the mechanical sed, or they'll silently survive it.
7. **Tests + docs last**: e2e asserts (e2e.rs:103,464; e2e_self_host.rs:115-121; ollama_e2e.rs:116-132), explain.rs:147, ARCHITECTURE-GUIDE/API-REFERENCE, tui/dashboard.rs source labels ("source: …/engrams.jsonl" :3890,:4391).
8. **One release later**: delete the `Engram` alias, `TaintInfo`, `Workspace`, `engrams_path_legacy` (30-CORE P3 retirement list).

Sizing: ~934 textual refs, but ≥90% are mechanical identifier swaps inside 10 crates + 1 app; the risky 10% are serde field names, JSONL file paths, and wire contracts — all enumerated above.

## Structural convention drift

**Error handling** (from `^(anyhow|thiserror)` in `crates/*/Cargo.toml`):
- thiserror-only (7, the protocol spine — target convention for libs): roko-agent, roko-compose, roko-gate, roko-graph, roko-learn, roko-orchestrator, roko-runtime.
- anyhow-only (11): roko-serve, roko-agent-server, roko-neuro, roko-index, roko-daimon, roko-dreams, roko-mcp-{code,github,scripts,slack,stdio}. Fine for bin-shaped crates (mcp-*, agent-server); **drift** for library crates roko-neuro/-index/-daimon/-dreams whose fallible APIs leak `anyhow::Result`.
- Both (5): roko-core (has typed `error/mod.rs` + anyhow), roko-cli (correct — binary), roko-chain, roko-demo, roko-acp.
- Neither (8): roko-fs, roko-std, roko-primitives, roko-conductor, roko-plugin, roko-lang-{rust,typescript,go} — ride on `roko_core::Result`/custom enums.
- **roko-acp pins `thiserror = "2"` / `anyhow = "1"` directly instead of `{ workspace = true }`** (`crates/roko-acp/Cargo.toml:17-18`) — the "new reference" crate violates workspace-dep convention.

**Module layout**: 56 `foo/mod.rs` files across crates (old-style): roko-agent 15, roko-cli 15 (incl. tests/common), roko-core 5, roko-serve 5, rest scattered. New-style flat layout (`foo.rs` + `pub mod foo;`): **roko-acp — 15 flat files, zero mod.rs** — and roko-graph (single `cells/mod.rs`). No enforced rule; new crates should follow roko-acp/roko-graph, and mod.rs trees should not grow.

**Doc comments**: strong — all 28 crates that have `src/lib.rs` open with `//!` crate docs (28/28; roko-mcp-slack/-github/-scripts are bin-only, no lib.rs). But kernel doc *content* drifts: cell.rs:87-88 still narrates v1 trait names; roko-graph/src/cell.rs:86 doc example says `["Gate", "Scorer"]`.

**Test organization**: mixed inline `#[cfg(test)]` + `tests/` dirs, no norm. Heavyweights: roko-agent (25+ integration files + fixtures/ tree), roko-cli (e2e.rs, e2e_self_host.rs, ollama_e2e.rs, smoke.rs, phase0_wiring.rs + tests/common/mod.rs), roko-core (phase1_integration, property_tests, cell_execute), roko-compose (snapshot tests). Thin end: roko-mcp-slack has 2 trivial tests for 9 networked tools (48-MCP). Convention gap, not blocker.

**Naming of files vs traits**: module files still carry v1 verb names for v2 traits — roko-std/src/{scorer.rs,router.rs}, roko-cli/src/gate_runner.rs (pre-Signal `GateRunner`/`GateReport` foundation vocabulary — 35-GATES 🕰️ note), roko-fs/src/{file_substrate,cold_substrate}.rs, mirage-rs roko_bridge/{chain,hdc}_substrate.rs.

## Data-file renames & migration strategy

Ground truth in `.roko/` today: **both** `engrams.jsonl` (10 lines, stale 2026-05-06, zero GateVerdicts) **and** `signals.jsonl` (467 lines, 2026-05-08, all the verdicts), plus `events.jsonl`, and a triple of episode files (`episodes.jsonl`, `learn/episodes.jsonl`, `memory/episodes.jsonl` — D5, same disease). The split:

| | engrams.jsonl | signals.jsonl |
|---|---|---|
| Label in code | "main" (layout.rs:202) | "legacy" (layout.rs:217) |
| Label in spec | absent from v2 | "primary Signal log" (01-SIGNAL.md:1371) |
| Writers | FileSubstrate (file_substrate.rs:48,72) via orchestrate.rs:8515 + serve state.rs:429 + TUI inject (tui/app.rs:1413-1414) | plan-runner raw JSON append (event_loop.rs:1147-1160 — PascalCase `{"kind":"GateVerdict"…}`, **not deserializable as Engram**) |
| Readers | serve /gates+/api/signals+metrics (gates.rs:84,92; episodes.rs:35; metrics.rs:119), TUI dashboard.rs:423+, status.rs:194, dreams cycle.rs:52 | chat_inline.rs:3201 (count), acp session.rs:1474 (existence), agent_serve.rs:1388 (archive) |
| `roko init` | creates; **migrates signals→engrams** (util.rs:135-150) | renamed away |

So records written by the live engine are invisible to every dashboard reader, and vice versa (35-GATES Open Q1 — confirmed here; note 35's phrasing "orchestrate writes verdicts to signals.jsonl" — the writer is precisely `runner/event_loop.rs:1147`, the runner-v2 path; orchestrate.rs's substrate path lands in engrams.jsonl).

**Strategy (engrams.jsonl → signals.jsonl, per v2 spec):**
1. **Schema first**: make the runner's verdict append go through FileSubstrate as a real Signal (30-CORE P0) so the canonical file has one schema. Keep `events.jsonl` for non-Signal runtime events.
2. **Flip the canonical name in one commit**: layout.rs (`signals_path()` main, `signals_path_legacy()` = engrams.jsonl), FileSubstrate default filename, all ~25 hardcoded `join("engrams.jsonl")` call sites replaced with `RokoLayout` accessors (centralization is the real fix — hardcoding is why the split happened).
3. **Migration on `roko init` / first open** (invert util.rs:135-150): if only engrams.jsonl exists → rename to signals.jsonl; if **both** exist (today's state) → append engrams.jsonl records into signals.jsonl deduped by `id` content-hash, then move engrams.jsonl to `.roko/backup/engrams.pre-v2.jsonl`. A dedicated `roko migrate data` command is warranted since both-files is the observed state, not an edge case.
4. **Dual-read window ≥1 release**: FileSubstrate reads legacy file when present (log a deprecation warning); writers only ever touch signals.jsonl.
5. **Flip tests + docs**: e2e.rs:103/464, e2e_self_host.rs:115, ollama_e2e.rs:116-132, dreams `ENGRAMS_LOG_FILE`, API-REFERENCE.md:1080,1141; CLI-REFERENCE.md:2296 is already correct.
6. Same recipe later for the episodes triple (D5) — do not fork the mechanism.

**Data-file addendum (2026-07-08 ground truth)**: `.roko/` now holds a **third, much larger log the noun-flip must not ignore**: `events.jsonl` at **44 MB** (2026-05-09, the newest write) — dwarfing both signals.jsonl (80 KB) and engrams.jsonl (10 KB). This is the runtime event firehose (non-Signal records), and it's the *only* file being actively grown. Its existence reframes the migration: the canonical-Signal-log question (signals vs engrams) is about two nearly-dead files, while the real write volume goes to `events.jsonl`. **Confirm before the flip**: (a) is `events.jsonl` meant to stay the non-Signal sidecar (then the runner's raw gate-verdict append at `event_loop.rs:1147` should move *there*, not into a Signal log)? (b) does any reader treat `events.jsonl` as a Signal source (it should not — it's not Engram-shaped)? This is a candidate **undocumented subsystem** the navigation layer should surface: an events-bus persistence path distinct from the Store/Signal path, unmentioned in v2 §Persistence.

## Migration checklist

- [ ] **[P0]** Record D3 verdict (Signal is the noun) + amend `docs/v2/01-SIGNAL.md:69` and signal.rs header to match — verify: `grep -n 'Signal' docs/v2/01-SIGNAL.md | head -5` and 26-CANONICAL-DECISIONS D3 marked decided
- [ ] **[P0]** Canonicalize `.roko/signals.jsonl`: flip layout.rs labels + FileSubstrate filename, centralize hardcoded paths through `RokoLayout` — verify: `grep -rn '"engrams.jsonl"\|join("engrams' crates/ --include='*.rs' | grep -v target | wc -l` → 0
- [ ] **[P0]** Invert init migration + handle both-files merge (dedup by id; backup old file) — verify: `cargo run -p roko-cli -- init /tmp/m && ls /tmp/m/.roko/signals.jsonl`; run in a dir seeded with both files, count lines
- [ ] **[P0]** Runner verdict writes become Signal-shaped via substrate (kill raw JSON at event_loop.rs:1147) so serve `/gates/*` sees them — verify: `cargo run -p roko-cli -- plan run plans/ && curl :6677/api/gates/recent | jq length` > 0
- [ ] **[P1]** Execute Engram→Signal struct rename in roko-core with deprecated alias — verify: `grep -n 'pub struct Signal' crates/roko-core/src/ -r` = 1 hit; workspace builds with `-D warnings` except deprecation allows
- [ ] **[P1]** Sweep Engram-vocabulary crates (fs→std→graph→neuro→plugin→dreams→conductor→serve→cli→apps) — verify: `grep -rn '\bEngram\b' crates/ apps/ --include='*.rs' | grep -v target | grep -v deprecated | wc -l` trends 885+49 → ~0
- [ ] **[P1]** Fix `roko new` scaffold templates (Kind::Signal, field names, Verify API) — verify: `cargo run -p roko-cli -- new gate foo --output /tmp/s && rustc --edition 2021 --crate-type lib /tmp/s/foo_gate.rs` (or paste into a test crate and `cargo check`)
- [ ] **[P1]** Delete roko-chain duplicate `Engram` (identity_economy_markets.rs:653) — verify: `grep -rn 'struct Engram' crates/ --include='*.rs' | grep -v target` = 0 (post-flip) or 1 (roko-core, pre-flip)
- [ ] **[P2]** agent-server: real persisted signal id, `signal_id` field (+`engram_id` compat) — verify: `curl :7777/message -d '…' | jq .signal_id` then `rg <id> .roko/signals.jsonl`
- [ ] **[P2]** WS filter `signal-stream:` (accept `engram-stream:` for one release) — verify: `grep -n 'signal-stream' crates/roko-serve/src/routes/ws.rs`
- [ ] **[P2]** Normalize slack tools to `slack.*`, kill `github.github.*` double-prefix — verify: `echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-slack | jq -r '.result.tools[].name' | grep -c '^slack\.'` = 9
- [ ] **[P2]** Retire `trait Substrate` blanket alias + fix cell.rs:87-88 / roko-graph cell.rs:86 doc vocabulary — verify: `grep -rn 'trait Substrate' crates/ --include='*.rs' | grep -v target` = 0
- [ ] **[P2]** Unify rung numbering (D6): one canonical table for pipeline + GateService + `.roko/learn/gate-thresholds.json` keys — verify: `grep -n 'rung_for_name' crates/roko-gate/src/gate_service.rs` matches pipeline docs
- [ ] **[P2]** Purge runtime-visible v1 lore: prd_prompt.rs:16 grimoire, explain.rs:147, surface_inventory.rs:1550, learning.rs:19 doc — verify: `grep -rni 'grimoire' crates/ --include='*.rs' | grep -v target` = 0
- [ ] **[P2]** Docs pass: ARCHITECTURE-GUIDE.md Engram→Signal (48 hits), API-REFERENCE.md engrams.jsonl:1080,1141, v2-depth verify-cells `pub trait Gate` snippet — verify: `grep -c Engram docs/v2/ARCHITECTURE-GUIDE.md` = 0 (or alias-note only)
- [ ] **[P3]** Error-handling convention: thiserror for lib crates (roko-neuro/-index/-daimon/-dreams), anyhow at bins; roko-acp deps → `{ workspace = true }` — verify: `grep -n 'thiserror = "2"' crates/roko-acp/Cargo.toml` = 0
- [ ] **[P3]** roko-std v1 names: scorer.rs/router.rs/noop.rs structs, `MemorySubstrate`→`MemoryStore`; roko-fs `*Substrate` types — verify: `grep -rn 'NoOpComposer\|NoOpPolicy\|MemorySubstrate' crates/ --include='*.rs' | grep -v target` = 0
- [ ] **[P3]** Reconcile Kind vs `SignalKind` (lifecycle.rs:940) vs signal_kinds consts — verify: `grep -rn 'enum SignalKind' crates/` = 0
- [ ] **[P3]** docs dir `02-block/`→`02-cell/` (fix inbound links); note apps/agent-relay vs "roko-relay" in 12-connectivity docs — verify: `ls docs/v2-depth/ | grep 02-`
- [ ] **[P3]** After one release: delete `Engram` alias, `TaintInfo`, `workspace::Workspace`, `engrams_path_legacy`, dual-read shim, `engram_id`/`engram-stream:` compat — verify: `grep -rn 'engram' crates/ apps/ --include='*.rs' -i | grep -v target | wc -l` ≈ 0

## Open questions

1. **Who ratifies D3?** Spec (`01-SIGNAL.md:69`), code comment (signal.rs:3), and storage direction (util.rs:138) each answer differently. This audit recommends Signal; nothing proceeds safely until it's written into 26-CANONICAL-DECISIONS.
2. Is the runner's raw-JSON append to signals.jsonl an intentional lightweight audit sidecar (→ move it to events.jsonl) or a missed migration (→ make it Signal-shaped)? (30-CORE Q2; answer determines checklist P0 #4 shape.)
3. Struct-suffix policy: are `*Gate/*Router/*Scorer` blessed as *cell-noun* vocabulary atop v2 verb traits (Gate clearly is, per v2-depth), or should NoOp*/std impls adopt verb names? Affects R10 and every future cell.
4. `engram-stream:` / `engram_id` compat window length — who are the external consumers (dashboard frontend per 66-FRONTEND-API-PARITY, relay clients per 70-RELAY-PROTOCOL-FREEZE)?
5. Mori/bardo provenance comments cite absolute paths under `/Users/will/dev/uniswap/bardo/…` (161 refs) — keep as-is, rewrite to `bardo-backup/` relative refs, or strip once parity is declared done?
6. Should `roko doctor` gain a naming-drift check (both jsonl files present, legacy WS filters, scaffold health) so this class of split-brain is detected instead of audited?
7. Does anything still *read* Mori state for real (`phase.rs:20-21` claims `.mori/state/*.json` compat)? If not, that compat guarantee should be dropped with the naming sweep.
