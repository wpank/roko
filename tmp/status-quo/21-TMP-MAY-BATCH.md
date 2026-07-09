# tmp/ Inventory — May 4-6 Design Batch (v2 refactor era)
> Status-quo audit · verified 2026-07-07 · **re-verified against HEAD `5852c93c05` on 2026-07-08**
>
> **Re-verification result (2026-07-08)**: every load-bearing claim below still holds byte-for-byte.
> `TaskExecutorCell` live dispatch is still a dry-run stub (`task_executor.rs:80-92`, warning text intact);
> `Signal` is still a re-export alias with `pub struct Engram` canonical (`signal.rs:6`, `engram.rs:63`);
> zero light-clients/MPP symbols exist in `crates/`; `.roko/GAPS.md` mtime is still 2026-05-05 (frozen);
> `PlanEngine::#[default]` is `RunnerV2` (`main.rs:1301`) while the clap arg default is `"graph"`
> (`main.rs:1361`) — confirming Open Question 1's "landmine." **New since this doc was written**:
> `roko develop` shipped (`commands/develop.rs`, `main.rs:2399`) as a thin `do_cmd --plan` wrapper — and it
> inherits the `--engine graph` default, so the stub boundary is now user-facing via the headline
> self-dev command. See [68-SELF-DEVELOPING-CROSSWALK.md](68-SELF-DEVELOPING-CROSSWALK.md).

## Summary

This batch is the paper trail of the **v1→v2 refactor sprint executed on branch `wp-arch2` between 2026-05-03 and 2026-05-06**, merged to `main` as PR #53. The chain of documents is causal, not independent:

1. **Audits** (May 4): `infrastructure-audit.md`, `model-provider-audit.md` catalogued anti-patterns.
2. **Plan** (May 4-5): `redesign-plan.md` (10 phases of hardening) and `tmp/v2-refactoring/` (58-item, 5-phase Cell/Signal/Graph/Engine/Feed migration — **this is THE v2 migration plan**).
3. **Execution harness** (May 5-6): `tmp/taskrunner/` ran 100 tasks through ~20 parallel worktree agents in waves; `tmp/isfr/` was a follow-on feature plan executed the same way.
4. **Record** (May 5): `pr-body.md` / `github-pr-53-body.md` / `wp-arch2-vs-main.md` document what shipped (941 commits, +437K/-53K, roko-graph crate added).

**Reconciliation verdict**: the v2-refactoring plan is ~85% implemented in code (roko-graph exists with engine/loader/registry/hot/cells; Cell::execute()+CellContext+TypeSchema; Observe/Connect/Trigger traits; graduation; feed CLI; plan→graph converter; `--engine graph` is the CLI default). The **load-bearing exception**: `TaskExecutorCell` live dispatch is *still* a dry-run stub as of today (`crates/roko-graph/src/cells/task_executor.rs:81-89`), so **Runner v2 remains the only engine that actually dispatches agents** — "Graph Engine as default" is nominal. The 7 cognitive-loop cells are still `PassthroughCell` stubs. `.roko/GAPS.md` contains only the Tasks 101-103 section and hasn't been updated since ~May 6.

One plan in this batch was **never implemented at all**: `tmp/light-clients/` (Verified Chain Layer, 22 work units — ConsensusVerifier, VerifiedState<T>, Tempo BLS, MPP payments). Zero of its types exist in `crates/`; roko-chain instead grew `x402.rs` as the payment rail. `Untitled-1.md` is a prompt paste buffer (scratch).

## Inventory table

| Item | Date | Size | Purpose | Adopted? | Verdict |
|---|---|---|---|---|---|
| `tmp/v2-refactoring/` (12 docs) | May 5 | ~75KB | **The v2 migration plan**: 5 phases, 58 checklist items (Cell execute, Signal rename, protocols, Graph+Engine, Feeds, Runner-v2→Engine migration) | ~85% — roko-graph shipped; live dispatch + hot-loop cells still stubs | **IMPLEMENTED** (residual gaps = GAPS.md 101-103) |
| `tmp/taskrunner/` | May 6 | ~100KB+ | Parallel-agent execution harness: dag.toml, waves 0-6, 100 task files, claim/gate/audit scripts | Harness did its job; 100/100 "implemented", 0 marked "verified" — batch-4 audit never recorded | **IMPLEMENTED** (historical record; stub list still open) |
| `tmp/isfr/` (6 docs + 20 tasks) | May 5 | ~100KB | ISFR keeper agent: relay pub/sub, IsfrFeed, ISFRSource trait, chain tools, contracts, serve/UI | Rust side yes: `roko-chain/src/isfr_keeper.rs`, `isfr_sources/{aave_v3,compound_v3,ethena,lido,mock}`, `chain_profile.rs`, `roko-core/src/isfr_feed.rs`, `roko-cli/src/commands/isfr.rs`, `roko-serve/src/feed_agents/keeper.rs` | **IMPLEMENTED** (F3-F5 demo-app UI unverified) |
| `tmp/light-clients/` (28 docs) | May 4 | ~460KB | Verified Chain Layer: 22 WUs — ConsensusVerifier trait, VerifiedState<T>, PlaybackVerifier, Tempo threshold-BLS, eth_getProof, MPP client/budget/ledger/discovery/server | **No.** Zero hits for `ConsensusVerifier|VerifiedChainClient|TrustedHeader|MppClient|mpp_pay|verified_balance` in crates/ | **AUTHORITATIVE-UNIMPLEMENTED** (MPP portion possibly superseded by `roko-chain/src/x402.rs`) |
| `tmp/redesign-plan.md` | May 4 | 148KB | Master hardening redesign: Phases 0-10 (boot, foundations, providers, tools, orchestration, serve, learning, frontend, deploy/CI, concurrency, ACP) + Batches 5-47 implementation log | Yes — batches all dated done May 3-4; spot-checks: root `Dockerfile` ✓, `commands/dev.rs` ✓, `/metrics` ✓, workspace registry ✓ | **IMPLEMENTED** (self-logging plan; a few residuals) |
| `tmp/infrastructure-audit.md` | May 4 | 156KB | Infra/anti-pattern audit §1-14 with in-place FIXED annotations | Most §12/§13 items marked FIXED May 3-4 | **IMPLEMENTED** (residuals: §12.2, §13.7, §13.10, §14.1, §14.3) |
| `tmp/model-provider-audit.md` | May 4 | 58KB | Provider anti-pattern audit (no slug-synthesis doctrine) + Batches 5-30 log + issues 1-19 | Yes — explicit-provider-config doctrine landed per batch log | **IMPLEMENTED** (historical) |
| `tmp/pr-body.md` | May 5 | 74KB | PR #53 body + rolling updates; final two sections = best narrative of Runner-v2 wiring (101 commits) and Graph Engine (36 commits) | Branch merged (git: `main`, post-merge commits present) | **IMPLEMENTED** (best what-shipped record) |
| `tmp/github-pr-53-body.md` | May 5 | 55KB | Earlier snapshot of same PR body (lacks the two final May 5 updates) | — | **SUPERSEDED** by pr-body.md |
| `tmp/wp-arch2-vs-main.md` | May 5 | 409KB | Reviewer packet: themes, reading order, GitHub API snapshot, verbatim PR body, full commit log vs main | — | **SUPERSEDED** (historical; useful commit-log appendix) |
| `tmp/Untitled-1.md` | May 6 | 118KB | Paste buffer: agent prompts, `run-agents.sh` invocations, fix plans referencing legacy `orchestrate.rs` line numbers | — | **SCRATCH** |
| `tmp/DEV-WORKFLOW.md` | May 1 | 10KB | Dev workflow reference: cargo-watch, nextest, Docker dev loop, Zed, pre-commit | `roko dev` exists; commands still valid | **KEEP** (current reference; pre-dates graph engine) |

## Per-item notes

### tmp/v2-refactoring/ — the v2 migration plan (READ `00-INDEX.md` + `CHECKLIST.md` first)
- **Structure**: `00-INDEX` (strategy: "Build New, Wire Immediately, Delete Old"; nothing built without a CLI command), `01-CURRENT-STATE` (wired/dead/floating census — notes orchestrate.rs 23,331 lines already feature-gated dead, ~15K LOC floating), `02-WIRING-STRATEGY`, `03-QUICK-WINS` (QW-1..8), `04-CELL-EXECUTE`, `05-SIGNAL-RENAME`, `06-NEW-PROTOCOLS` (Observe/Connect/Trigger), `07-GRAPH-ENGINE` (full type design for Graph/Node/Edge/CellRef/Engine/Flow + `roko graph run` wiring plan), `08-FEEDS`, `09-GRADUATION` (Pulse→Signal promotion policies), `10-DEAD-CODE-AUDIT` (DELETE 2 / WIRE-NOW 10 / KEEP-tag 18), `CHECKLIST.md` (58 items P0→P4).
- **Key decisions**: target active paths (`roko run`→WorkflowEngine, `plan run`→Runner v2, `serve`), not dead orchestrate.rs; build roko-graph alongside Runner v2 then migrate (Phase 4); Graphs are Cells (fractal); Workflow/Activity execution classes; graduation as the only path from ephemeral Pulse to durable Signal.
- **Adoption evidence**: `crates/roko-graph/src/{engine,loader,registry,topo,condition,budget,convert,hot,cells/}.rs` all exist; `roko-core/src/cell.rs:126` has `async fn execute(&self, input: Vec<Engram>, ctx: &CellContext)`; `roko-core/src/traits.rs:400,408,420` has Observe/Connect/Trigger; `roko-core/src/pulse.rs:138` has `graduate()`; `roko-core/src/config/graduation.rs` + `roko-graph/src/cells/graduation.rs` exist; `commands/{graph,feed}.rs` exist; `PlanCmd::Run` has `--engine` with `default_value = "graph"` (`roko-cli/src/main.rs:~1361`).
- **Deviations from plan**: (a) **Signal rename half-done the other way** — plan said make `pub struct Signal` canonical with deprecated `Engram` alias (P1-6/QW-1); code today still has `pub struct Engram` (`engram.rs:63`) and `signal.rs:6: pub use crate::engram::{Engram as Signal, ...}` — crates import `Signal` but the struct never flipped. (b) **FileWatchFeed / ProviderHealthFeed (P3-2/P3-3) never built** — the concrete feed became `IsfrFeed` + a separate `FeedAgent` trait in `roko-serve/src/feed_agents/mod.rs:54`; `roko-core/src/feed.rs` is a *different* Feed concept (FeedRegistry for agent data streams, with its own migration note to docs/v2 M037). (c) Checklist checkboxes were never ticked ("Phase 0..4: Not started") — tracking moved to taskrunner STATUS.toml; treat CHECKLIST.md status fields as stale.

### tmp/taskrunner/ — the execution machine
- `README.md`: wave model (wave 0 foundation → 1 parallel fixes → audit → 2 v2 core → 3 graph+engine → 4 feeds/graduation/calibration → 5 migration+hot → 6 cleanup), status progression `pending→claimed→implemented→tested→wired→verified→done`, rule "no task without a CLI wire target".
- `PROGRESS.md` (May 6): 100/100 implemented across 4 agent batches; explicitly lists intentional stubs (TaskExecutorCell dry-run only; 7 PassthroughCells; OTLP layer placeholder) and flags "All 28 batch-4 tasks need auditing".
- `STATUS.toml`: every task `status = "implemented"`, **none** `verified`/`done` → the audit wave for batch 4 never closed the loop. `dag.toml` (40KB) holds the full task DAG; `tasks/` has ~100 task files; `scripts/` has spawn/claim/gate/merge/audit automation.
- Batch-4 highlights (map to v2 checklist): 036 gate `Cell::execute()` impls, 042 protocol-trait integration tests, 066-071 graph foundation/engine/fanout/conditions/budget, 082 streaming-first backend (`stream_turn()`), 092 WAL, 094 checksummed snapshots, 095 Prometheus `/metrics`, 098 feed CLI, 099 graduation, 101 plan→graph converter, 102 `--engine` flag, 103 hot graphs, 104 StateHub moved roko-core→roko-runtime.

### tmp/isfr/ — ISFR keeper (interest-rate oracle feeds)
- 6 design docs (relay pub/sub upgrade; Feed-trait alignment with taskrunner task 097; ISFRSource trait + keeper; chain tools; contract deployment via ChainProfile with mirage-rs default; end-to-end integration) + `tasks/` A1-F5 (~2,700 LOC planned). Ties to `tmp/prds/IMPL-06-ISFR.md` (7-phase program — these docs are its Phase 1).
- Adopted (verified in code): keeper + 5 sources + bootstrap + oracle-submit + chain_profile in roko-chain; `isfr_feed.rs` in roko-core; `roko isfr` CLI; serve `feed_agents/keeper.rs`, events, health; agent-server `relay_subscriber.rs`. The relay itself (`agent-relay`, phases A1-A6) lives outside this repo — see `tmp/relay-bus/` (May 8) for the successor work.

### tmp/light-clients/ — Verified Chain Layer (NOT built)
- `00-INDEX.md`: 22 WUs in 6 dependency layers, ~50-68h estimate; WU-1..14 = verification stack (ConsensusVerifier trait, TrustLevel, PlaybackVerifier, RpcOnlyVerifier, MPT state proofs via alloy-trie, ThresholdBlsVerifier for Tempo chain-id 4217/42431, VerifiedChainClient, BackendPool config, watcher, sidecar routes, orchestrator wiring); WU-15..22 = demo/dashboard + MPP (Machine Payments Protocol client, tools, budget policy, `.roko/payments.jsonl` ledger, discovery, agent-as-MPP-server with 402 middleware). Contains a "Corrections from Prior Docs" section (QMDB claim wrong; MPT proofs; real chain IDs).
- **Adoption: none.** No verifier/MPP symbol exists anywhere under `crates/`. `roko-chain/src/` today: isfr*, x402.rs, witness.rs, block_watcher.rs, observer.rs, marketplace.rs, futures_market.rs, identity_economy_*, reputation_registry.rs, korai_token.rs, etc. If chain verification/payments becomes a priority again, this folder is the most complete design on file — but re-baseline against current roko-chain (which has grown ~30 modules; CLAUDE.md's "roko-chain: Phase 2+" row is badly stale) and decide MPP-vs-x402 first.

### tmp/redesign-plan.md + tmp/infrastructure-audit.md + tmp/model-provider-audit.md — the audit→fix trio
- `redesign-plan.md`: top = "Implementation Progress" batches 5-47 (all logged done 2026-05-03/04); body = Phases 0-10 with per-item designs (RAII terminal guard, error enums, atomic I/O, unified config loader, RetryPolicy, streaming-first trait, token budgets, tool schemas/safety, progress event bus, smart exit codes, workspace registry, health/metrics, `roko dev`, episode compaction, SSE frontend, 3-stage Dockerfile, CI, bounded channels); "Execution Order" at L2360.
- `infrastructure-audit.md`: §11 priority table; §12 tool-system and §13 orchestration items mostly annotated FIXED. **Unannotated residuals worth re-checking**: §12.2 (bash tool process confinement), §13.7 (config loaded from worktree exec_dir not project root), §13.10 (three gate rungs always skipped), §13.12 (raw-int rung config), §14.1 (PRD promote write-then-delete), §14.3 (frontmatter line-scanner not YAML).
- `model-provider-audit.md`: source of the "no provider inference from model slug / no runtime config synthesis / explicit providers only" doctrine, plus "twenty config loaders" finding (#12) → unified loader work. Batches 5-30 logged done.
- These three overlap heavily (redesign-plan embeds both audits' batch logs). Treat `redesign-plan.md` as the umbrella.

### PR trio + scratch
- `pr-body.md` §"Update 2026-05-05: Runner v2 wiring" and §"Update 2026-05-05: Graph execution engine" (L1086-1415) are the two most information-dense summaries of the sprint — including the claim "Engram→Signal rename... now fully propagated" (true only for import sites, see above) and the roko-graph feature table (75 tests, 7 built-in cells, `roko graph run/validate/show`).
- `wp-arch2-vs-main.md` adds reviewer reading order + full `git log main..HEAD` appendix — useful for archaeology only.
- `Untitled-1.md`: prompts/transcripts; references legacy `orchestrate.rs` internals (`handle_enriching` ~L8382, `skip_enrichment`) that are now feature-gated dead code. No design content that isn't better captured elsewhere.

## The v2 refactor plan vs reality

| Planned (v2-refactoring CHECKLIST) | Shipped? | Evidence |
|---|---|---|
| **P0 quick wins**: TopicFilter And/Or/Not; balance/demurrage on Signal; delete roko-calc; STATUS tags; wire calibration_policy, demurrage_consumer, run_ledger, error_enrichment, post_gate_reflection, section_outcome | Yes (jsonl_rotation unconfirmed) | pr-body L1187-1207, L1277-1312; roko-calc absent from `crates/` |
| **P1**: Cell::execute() + CellContext + TypeSchema; execute() on 4 gates; Observe/Connect/Trigger | Yes | `cell.rs:126`; `traits.rs:400-420`; taskrunner 036/042 |
| **P1**: Engram→Signal rename (Signal canonical) | **Partial** | struct still `Engram` (`engram.rs:63`); `Signal` is a re-export alias (`signal.rs:6`) |
| **P2**: roko-graph crate — types, loader, registry, topo, engine, `roko graph run`, default cells, examples, fan-out/fan-in, conditional edges, budget, deadline, AgentCell, ComposeCell | Yes (75 tests; `Condition` renamed from `EdgeCondition`) | crate tree; pr-body L1247-1269; taskrunner 066-071 |
| P2-11 sub-graphs, P2-12 flow snapshots/resume | **No** — graph path has no `--resume-plan` support | GAPS.md Task 101 |
| **P3**: Feed trait + FileWatchFeed + ProviderHealthFeed + feed CLI | **Morphed** — feed CLI ✓ (task 098), concrete feeds became IsfrFeed + serve `FeedAgent`; the two planned example feeds don't exist | grep: no `FileWatchFeed|ProviderHealthFeed` in crates/ |
| **P3**: graduation (Pulse::graduate, policy config, GraduationCell); predict-publish-correct calibration | Yes | `pulse.rs:138`; `config/graduation.rs`; `cells/graduation.rs`; learning subscriber (pr-body L1277) |
| **P4-1..3**: plan→graph converter; `--engine graph` flag; comparison runs | Converter + flag yes (tasks 101/102); comparison runs not evidenced | `convert.rs`; `main.rs` PlanEngine |
| **P4-4/5**: Engine default; Runner v2 feature-gated | **Nominal only.** CLI `--engine` defaults to `graph`, but `TaskExecutorCell` live dispatch still falls back to dry-run with a warning (`task_executor.rs:81-89`) → real agent work still requires Runner v2. Gate coverage partial (only `PlanCmd::Run` gated; runner internals compiled unconditionally) | GAPS.md Tasks 101/102; code re-verified 2026-07-07 |
| **P4-6/7**: Hot Graphs + declarative cognitive loop | Skeleton only — HotPolicy/start_hot exist; all 7 loop cells are PassthroughCell stubs; `persist_tick_state` unimplemented; `[graph.policy.hot]` not parsed | GAPS.md Task 103; `hot.rs`, `cells/stubs.rs` |

**Net**: Phases 0-3 substantially shipped; Phase 4 (the actual migration) stalled at the stub boundary and nothing has moved since May 6 (GAPS.md unchanged; TaskExecutorCell comment intact). The repo currently runs a hybrid: Runner v2 does the real work; Graph Engine is a parallel, mostly-real-but-not-agent-dispatching path that is confusingly the CLI default.

## Checklist

- [ ] **[P0]** Implement `TaskExecutorCell` live dispatch (or flip the clap `--engine` default to `runner-v2` until it exists). **This now also fixes `roko develop`/`roko do`**, which inherit the graph default via `develop.rs`→`do_cmd`. Verify: `roko develop "<real task>"` produces real agent output, not `task-output:stub:*` engrams; grep `task_executor.rs:80-92` for the dry-run fallback warning
- [ ] **[P0]** Decide fate of `tmp/light-clients/`: adopt (re-baseline 22 WUs against current roko-chain), or mark superseded by x402 — verify: decision recorded; grep `ConsensusVerifier` in crates/ after any adoption work
- [ ] **[P1]** Graph Engine parity gaps from GAPS.md Tasks 101-103: parallel node dispatch (`max_parallel` stored but unused), `--resume-plan` on graph path, conditional-edge evaluation in cognitive loop, `[graph.policy.hot]` TOML parsing, `persist_tick_state` — verify: each against `roko-graph/src/{engine,hot,loader}.rs`
- [ ] **[P1]** Replace the 7 `PassthroughCell` cognitive-loop stubs with real cells — verify: `roko-graph/src/cells/stubs.rs` shrinks; hot-graph run produces non-passthrough output
- [ ] **[P1]** Close the taskrunner audit loop: batch-4's 28 tasks were never audited (`STATUS.toml` has zero `verified`) — verify: either run the audit scripts or fold residuals into GAPS.md and archive tmp/taskrunner
- [ ] **[P2]** Finish the Signal rename in the planned direction or document the alias as final — verify: `grep 'pub struct Signal' crates/roko-core/src/` or an ADR note
- [ ] **[P2]** Re-check unfixed infrastructure-audit residuals: §12.2 bash confinement, §13.7 worktree config root, §13.10 skipped gate rungs, §14.1 PRD promote atomicity, §14.3 frontmatter parser — verify: each section in `tmp/infrastructure-audit.md` against current code
- [ ] **[P2]** Update CLAUDE.md: it still describes the orchestrate.rs/PlanRunner era ("orchestrate.rs Wired", "roko-chain Phase 2+", 18 crates) — reality is Runner v2 + roko-graph + 31 crates incl. roko-acp/roko-demo/roko-plugin — verify: diff CLAUDE.md tables against `ls crates/` and `main.rs` command tree
- [ ] **[P2]** GAPS.md is stale (single May-6 section) despite CLAUDE.md calling it the canonical tracker — verify: append post-May gaps or link this audit
- [ ] **[P3]** Verify tmp/isfr F3-F5 (demo-app DataHub slice, dashboard page, nav) landed — verify: grep isfr in demo app TS sources
- [ ] **[P3]** Archive scratch/superseded: `Untitled-1.md`, `github-pr-53-body.md`, `wp-arch2-vs-main.md` → `tmp/archive/` — verify: files moved, nothing links to them
- [ ] **[P3]** Confirm DCA-4 jsonl_rotation actually wired to episodes/efficiency logs — verify: grep `JsonlRotation` usage outside roko-learn

## Open questions

1. **Is the graph-default CLI flag a landmine?** **PARTLY RESOLVED (2026-07-08).** Confirmed the mismatch: `PlanEngine::#[default] = RunnerV2` (`main.rs:1301`) but the clap arg carries `default_value = "graph"` (`main.rs:1361`), and `commands/plan.rs:258` branches on `PlanEngine::Graph`. So when a user runs `roko plan run` with no `--engine`, clap supplies `Graph` (the arg default wins over the enum `#[default]`, which only applies to programmatic `PlanEngine::default()` construction). The graph path then hits `TaskExecutorCell` → dry-run stub. **Still open**: does any wrapper (`do_cmd`, `develop`) override the engine before dispatch? `develop.rs` does NOT — it inherits the default. So `roko develop "..."` on a real plan can emit `task-output:stub:*` engrams and report success. This is now a **P0** (was P0 for `plan run`; now also the headline self-dev command).
2. **Was PR #53's batch-4 code ever independently audited?** PROGRESS.md demanded it after batch 3 found 3 P0s; STATUS.toml suggests it never happened before merge.
3. **MPP vs x402**: was the light-clients MPP design consciously replaced by x402 (dated later in roko-chain), or did it just fall off? Affects whether WU-17..22 are dead or dormant.
4. **Where does the runtime Feed story land?** Three coexisting concepts: `roko-core/feed.rs` FeedRegistry (with its own migration note to Bus Pulse streams, "M037"), `roko-serve` FeedAgent, and `IsfrFeed`. The v2-refactoring `08-FEEDS.md` design matches none exactly.
5. **Does `tmp/relay-bus/` (May 8) supersede isfr Phase A** (relay pub/sub) — i.e., is the agent-relay upgrade complete in its own repo?
6. Which of the redesign-plan Phase 7 (frontend SSE) and Phase 8 (CI pipeline) items are verifiable here? Root `Dockerfile` exists but `.github/workflows/` couldn't be confirmed from this audit.
