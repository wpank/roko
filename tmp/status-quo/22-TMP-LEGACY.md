# tmp/ Inventory — Legacy Archive (May 1 batch)

> Status-quo audit · verified 2026-07-07 · **re-verified against HEAD `5852c93c05` on 2026-07-08**
>
> Scope: the 32 directories + 4 loose files all timestamped **2026-05-01 11:24/11:25** (a bulk
> move; content authored 2026-04-16 → 2026-05-01). Everything here predates the v2 refactor
> (`tmp/v2-refactoring/`, May 5) and the later audit waves (`tmp/subsystem-audits/`,
> `tmp/doc-convergence/`, May 16; `tmp/status-quo/`, July 7).
>
> **Re-verification deltas (2026-07-08)** — spot-checks against current code changed the disposition of
> several "still-open" items:
> - **BI_01 (unscrubbed gists) is now FIXED.** `crates/roko-cli/src/share.rs` scrubs all transcript text
>   via `roko_core::obs::LogScrubber` + a secondary ≥32-char hex/base64 redaction pass before upload
>   (`share.rs:10-17,41-45`). The legacy "posts raw unscrubbed transcripts" claim is **stale**.
> - **Unauthenticated share creation** is more nuanced than the legacy note: `roko-serve/src/routes/mod.rs`
>   mounts `shared_runs::auth_routes()` behind `require_api_key` (L184) AND `shared_runs::public_routes()`
>   *intentionally* without auth (L241-243, "Public share-receipt reader … so recipients can open share
>   links without a roko API key"). So public share **reads** are by-design; re-verify that share
>   **creation** (`POST /api/runs/{id}/share`) is inside the auth nest, not the public one.
> - **backlog/removed-learn-modules recovery: 1 of 8 back.** `crates/roko-learn/src/contextual_bandit.rs`
>   now exists; the other 7 (bandit_research, causal, shapley, resonant_patterns, kalman, adversarial,
>   signal_metabolism) are still absent. Update the recovery map to reflect partial reintegration.
> - **visual-gate2 unified evaluation: still NOT built.** No `UnifiedEval`/`EvaluationFramework`/composable
>   gate symbols in `roko-gate/src/`. Confirmed still-relevant/unadopted.
> - **productionizing budget/OTEL/semantic-caching: still NOT meaningfully wired.** Only config-level hits
>   (`roko-serve/src/lib.rs`, `roko-core/src/config/serve.rs`); no `BudgetEnforce`/`SemanticCache`/live OTLP
>   pipeline. Confirmed still-relevant/unverified-against-code.
> - **CLAUDE.md:225 + 307** still contain the broken `tmp/ux-followup/` paths (should be `tmp/ux/ux-followup/`).
>   Cleanup item unchanged.

## Summary

- **Most of the batch is safely historical**: pitch/demo material for the May 6 a16z meeting (passed), Mori-convergence audits executed by the arch runner (2026-04-28), and the unified-migration plans superseded by `tmp/v2-refactoring/`.
- **Seven items still carry open work** that must survive into the new roadmap: `MASTER-TASKS.md`, `mega-parity-audit.md` + `-v3.md`, `binary-issues/` (with its runner tracker at 3/56 batches done, **5 unresolved P0 security items**), `ux/ux-followup/` (40 open items), `prds/impl2/` (46 gap-fix tasks), `productionizing/` (P1–P18 + F1–F7 + D1–D9), and `backlog/` (8 recoverable roko-learn modules).
- **CLAUDE.md has two broken references**: `tmp/ux-followup/…` moved to `tmp/ux/ux-followup/…` (CLAUDE.md lines 225 and 307), and `tmp/MASTER-TASKS.md` exists but is frozen at 2026-04-26 — pre-v2, with an expired demo section.
- Two directories are completely empty: `tmp/unified-depth/`, `tmp/gateway/`.

## Inventory table

| Dir/File | Purpose | Verdict | Successor / Notes |
|---|---|---|---|
| `tmp/workflow/` | Mori-vs-roko workflow comparison: 17 subsystem audits + `UNIFIED-IMPLEMENTATION-PLAN.md` (80+ tasks, 3-runtime convergence) + `ANTI-PATTERNS.md` | SUPERSEDED-BY | `tmp/subsystem-audits/` (carries copies of both plans) + `tmp/v2-refactoring/`. ANTI-PATTERNS still good reading |
| `tmp/visual-gate2/` | 10 PRDs (2026-04-29): unified evaluation architecture replacing 7-rung pipeline / LLM judge / quality_judge / vision loop / PRM with one composable framework | **STILL-RELEVANT** | Designed, never built — gates are still the 7-rung pipeline per CLAUDE.md. Supersedes `tmp/archive/visual-gate-v1/` |
| `tmp/ux/` | 5-layer agent-native architecture design (agent-server, mirage extraction, auth, dashboard) + `implementation-plans/` + **`ux-followup/`** | MIXED | Design docs largely executed (roko-agent-server exists). `ux/ux-followup/00-INDEX.md`: **40 open items (31 P1 + 9 P2)** as of 2026-04-20 — this is the dir CLAUDE.md still calls `tmp/ux-followup` |
| `tmp/unified-migration/` | 4-phase Engram→Signal/Pulse/Cell/Graph migration checklist | SUPERSEDED-BY | `tmp/v2-refactoring/` (00-INDEX…09-GRADUATION) which executed the rename/Cell/Graph work; residue tracked in `.roko/GAPS.md` (tasks 101–103) |
| `tmp/unified-migration-runner/` | Runner harness for the above: MASTER-CHECKLIST (123+ batches), last run 2026-04-26, `POST-REFACTOR-ROADMAP.md` | SUPERSEDED-BY | `tmp/v2-refactoring/`. POST-REFACTOR-ROADMAP ideas worth one skim before archiving |
| `tmp/unified-depth/` | — | HISTORICAL | **Empty dir — delete** |
| `tmp/runners/` | 9+ self-contained parallel batch runners + `parallel-template/` machinery | MIXED | `runners/binary-issues/ISSUE-TRACKER.md`: **3/56 batches done, BI_01–BI_05 security P0s all open** (verified 2026-05-01). Other runners (mega-parity, post-parity ×330, converge, perf, ux-impl) historical |
| `tmp/ressearch2/` [sic] | 7 research dumps on the Casado/a16z May 6 pitch strategy | HISTORICAL | Meeting passed |
| `tmp/research/` | 15 deep-research dumps: agent-OS paradigms (field calculus, CRDTs, stigmergy, market structure) | HISTORICAL | Fed `tmp/prds/` + learnings sets; keep as citations archive |
| `tmp/productionizing/` | Railway/production guide: 12 docs incl. **P1–P18** blockers plan, **F1–F7** frontier wiring (ADAS, research pipeline, math, daimon, novelty, spawning, collusion), **D1–D9** production economics (budget enforcement, semantic caching, cost metrics, compliance, OTEL) | **STILL-RELEVANT** | Task plans unverified against current code; budget/OTEL/caching not confirmed wired anywhere |
| `tmp/prds/` | The Roko+Korai vision corpus: PRD-01…10 (~22K lines) + IMPL-01…10 checklists + `impl/` + **`impl2/` (6 gap-fix PRDs, 46 tasks, audit 2026-04-22)** | **STILL-RELEVANT** | Architecture north star pre-`tmp/unified/`. impl2's 46 wiring tasks need re-triage vs v2 code |
| `tmp/mori-diffs/` | 38 Mori→Roko convergence audits; `29-CURRENT-RUNTIME-GAP-LEDGER.md` was canonical; `23-HANDOFF-OPEN-ITEMS.md` | SUPERSEDED-BY | Arch runner (P0A–P4B, 2026-04-28) built the foundation services; superseded by `tmp/subsystem-audits/` + `tmp/status-quo/` |
| `tmp/learnings2/` | Investor/engineer briefing set v2 + whitepaper-v2 PDF | SUPERSEDED-BY | `tmp/learnings3/` |
| `tmp/learnings3/` | Briefing set v3 (extends v2: Aubakirova corpus, Keycard, benchmarks) + whitepaper PDF | HISTORICAL | Most recent of the series — keep as business/positioning reference |
| `tmp/gateway/` | — | HISTORICAL | **Empty dir — delete** |
| `tmp/dogfood/` | Dogfood findings index (2026-04-26) + May 6 demo/deck/landing checklists + STATE-OF-THE-WORLD | MIXED | Demo items expired; its P1–P3 runtime-bug list (F5 memory leak, S4 signals.jsonl dead path, etc.) flowed into MASTER-TASKS §2 — re-verify those |
| `tmp/design-systems/` | ROSEDUST design system, Three.js/WebGL patterns, site-generation mega-prompt | **STILL-RELEVANT** | Reusable design reference, not work items. Keep |
| `tmp/demo-app-backup/` | Snapshot of the vite demo app (dist + src) | HISTORICAL | Delete (build artifacts) |
| `tmp/demo-current/` | Built vite demo snapshot (`dist/`, tsbuildinfo) | HISTORICAL | Delete (build artifacts) |
| `tmp/demo-new/` | ROSEDUST unified demo-page redesign plan + chain-knowledge/job-market demo scripts | HISTORICAL | Demo era closed |
| `tmp/demo-redesign/` | `AUDIT.md`: 16 roko-serve hang issues (S1 sync AlloyChainClient, S2 chain-watcher hang, S3 block_in_place, S4 StateHub sync I/O) | **STILL-RELEVANT** | Serve startup-blocking bugs never confirmed fixed — re-verify against `crates/roko-serve/src/{state.rs,lib.rs}` |
| `tmp/demo-req/` | Pitch collateral: whitepaper PDFs, decks, landing HTML, `FINAL-GAP-ANALYSIS.md`, seriesAResearch | HISTORICAL | Meeting passed; PDFs are the only copies of some decks — archive, don't delete |
| `tmp/demo-resources/` | `smoke-test.sh`, `run-all.sh` + scripted validations for serve/matchmaking/prd/research/self-hosting | **STILL-RELEVANT** | Working smoke-test scripts; candidates to migrate to `scripts/` and CI |
| `tmp/demo-uis/` | 17 landing/UI iterations (v1–v17) + V8-SPEC + narrative docs | HISTORICAL | Design exploration; `design-systems/06-SITE-CATALOGUE.md` indexes it |
| `tmp/daeji/` | 12-doc deep study: roko × daeji chain integration (precompiles, knowledge layer redesign, coexistence) | **STILL-RELEVANT** | Reference for CLAUDE.md item 16 (chain runtime integration, Phase 2+) |
| `tmp/binary-issues/` | Canonical binary/UX tracker (2026-04-28): **90+ issues by root cause**, MASTER-INDEX + 19 detail files (provider dispatch, chat lifecycle, TUI polish, slash commands, serve security) | **STILL-RELEVANT** | Runner tracker shows 3/56 done. Security section (S5.5–S5.9) unresolved: unscrubbed gists, no terminal allowlist, no PTY caps, `dangerously_skip_permissions` defaults |
| `tmp/backlog/` | `removed-learn-modules.md`: 8 roko-learn modules (4,808 LOC) deleted 2026-04-26 with per-module reintegration recipes; recoverable from `wp-arch2` history | **STILL-RELEVANT (1/8 recovered)** | `contextual_bandit.rs` reintegrated ✓; still missing: bandit_research, causal, shapley, resonant_patterns, kalman, adversarial, signal_metabolism |
| `tmp/audit-patches/` | 10 git `.patch` files from audit branches (incl. `context-sidecar-prd.patch`) | HISTORICAL | Verify each merged, then delete |
| `tmp/archive/` | Archive of archives: depth-v1, workflow-v1, visual-gate-v1, learnings-v1, roko-trustworthy, stale-root (MASTER-REMAINING-WORK, MORI-PARITY-GAP-ANALYSIS, redesign v1/v2…) | HISTORICAL | Already the designated graveyard — move other archived dirs here |
| `tmp/acp-features/` | ACP (Zed/Cursor/JetBrains editor protocol) feature checklist for roko-acp | **STILL-RELEVANT** | roko-acp crate exists; checklist has `[ ]`/`[~]` items never re-verified |
| `tmp/acp-runner/` | Overnight Codex runner that created the roko-acp crate (18 batches) | HISTORICAL | Crate shipped |
| `tmp/agentchain-v2/` | 4 doc sets (01-roko, 02-daeji, 03-isfr, 04-markets): agent-chain v2 architecture | **STILL-RELEVANT** | Phase 2+ chain reference, sibling of `daeji/` |
| `tmp/architecture-archive/` | 21-doc canonical architecture spec (split from roko-architecture-redesign-v2), AC added 2026-04-25 | SUPERSEDED-BY | `tmp/prds/` PRD set + `tmp/unified/` spec. §20 (orchestrator gaps) / §21 (TUI ops) gap lists fed later audits |
| `tmp/scratchpad.md` | 1,020-line pastebin: session prompts, runner invocations, PRD-ingestion transcripts | HISTORICAL | No unique work items |
| `tmp/mega-parity-audit.md` | Deep audit v2 of 113 tasks (2026-04-29, wp-arch2): **16 critical issues** | **STILL-RELEVANT** | Findings target the V2 WorkflowEngine path that is now default — see rescued list below |
| `tmp/mega-parity-audit-v3.md` | v3 quality report on 130 tasks: 90 SOLID / 37 PARTIAL / **3 HOLLOW**, 4 security findings, 8 structural anti-patterns | **STILL-RELEVANT** | Same — each finding needs re-verification against current tree |
| `tmp/MASTER-TASKS.md` | Consolidated open-work list, 7 sections, updated 2026-04-26 | **STILL-RELEVANT (STALE)** | CLAUDE.md's "Master task list". §1 (May 6 demo) expired; §5 superseded by v2-refactoring; §2/§3/§4/§6/§7 carry live items. Regenerate from `tmp/status-quo/12-ROADMAP.md` |

## Still-open work rescued from legacy docs

Items below were open as of the batch date and are **not** confirmed closed anywhere newer; each needs a verify-or-carry decision in the new roadmap.

### A. Security (P0)
- **[RE-VERIFY, not closed]** Unauthenticated share creation: `POST /api/runs/{id}/share`. `routes/mod.rs` now has a deliberate `auth_routes()`/`public_routes()` split (L184 auth-nested, L241-243 public reader). Confirm the *create* route is in `auth_routes()`; public *read* is by-design — `mega-parity-audit-v3.md` §Security; `crates/roko-serve/src/routes/shared_runs.rs`
- **[FIXED 2026-07-08]** CLI `roko run --share` no longer posts raw transcripts — `share.rs` scrubs via `LogScrubber` + ≥32-char hex/base64 redaction (`share.rs:10-17,41-45`). Was `mega-parity-audit.md` #1 / BI_01; **close this row.**
- **[OPEN]** Terminal command allowlist, PTY session caps/TTL, `dangerously_skip_permissions` default-to-false, secret-leak violations promoted to Block — ISSUE-TRACKER BI_02–BI_05 (not re-verified; carry forward)
- `acknowledge_public_risk = true` silently bypasses terminal auth on public binds — v3 (`routes/mod.rs`)
- Auth opt-in: `roko init --cloud` / `PORT` env produce 0.0.0.0 binds with no auto-provisioned auth — v3

### B. V2 execution-path correctness (mega-parity v2 + v3 — high value since V2 engine is now the default)
- `[[task.verify]]` commands silently discarded in the V2 WorkflowEngine path (only PlanRunner honors them) — v2 audit #2
- `[[gate]]` TOML arrays from `roko init --profile rust` silently ignored by `roko plan run` (config schema split `[[gate]]` vs `[gates]`) — v2 #3, v3 anti-pattern 4
- V2 path: no adaptive gate thresholds (`service_factory.rs:195`) and gate verdicts not logged to `episodes.jsonl` — v2 #8, #9
- Skipped verdicts trigger `GateFailed` in EffectDriver — v2 #6 (`roko-gate/src/gate_service.rs`)
- Episode path mismatch: `learn.rs:394` reads `.roko/learn/episodes.jsonl`, logger writes `.roko/episodes.jsonl` → `roko learn episodes` always empty — v2 #4
- Double-write to `episodes.jsonl` (EpisodeSink zeroed data + legacy `emit_feedback` rich data) — v2 #5; zeroed `AgentOutcome` also no-ops RoutingObservationSink — v2 #12
- Two parallel model-selection paths: `EffectiveModelSelection` vs the inline 8-step pipeline in the `plan run` hot path — v2 §Structural, v3 anti-pattern 1
- Playbooks built from planned TOML, not actual episodes (`extract_playbook_from_episode` never called from plan-run) — v2 §Structural
- `roko config mcp` panics (`unreachable!`) — v3 HOLLOW 1; plan regenerate blind to validation diagnostics — v3 HOLLOW 3; `roko bench demo --real` is a simulation stub — v2 #16
- Streaming events silently drained in chat Session mode (spinner until turn completes) — v3 anti-pattern 2; context pack not wired into plan generation — v3 anti-pattern 3

### C. Runtime bugs (MASTER-TASKS §2 ← dogfood; demo-redesign)
- **F5**: unbounded `efficiency_events: Vec` never drained → 9.5 GB RSS memory leak
- **S4**: `signals.jsonl` dead path — conductor writes `engrams.jsonl` instead
- **M1/M2**: no streaming in non-approval path; TUI model column shows "-" (`model: String::new()`)
- **#9** 120s enrichment-timeout hardcode in gate judge; **#12** knowledge endpoint uses `/neuro/` not `/knowledge/`; **S7** `learn/` files (cascade-router.json, gate-thresholds.json) stale in runner v2
- roko-serve startup hangs S1–S4 (sync `AlloyChainClient::http`, chain-watcher await, `block_in_place`, sync StateHub bootstrap) — `demo-redesign/AUDIT.md`

### D. Runner-v2 / engine completion (MASTER-TASKS §3; continued in `.roko/GAPS.md` tasks 101–103)
- Phase C (runner v2 default for all `plan run`), Phase D (deprecate orchestrate.rs), Phase E (unified-spec alignment)
- CascadeRouter + AdaptiveThresholds persistence and replan-on-gate-failure not wired in runner v2

### E. UX followup (`tmp/ux/ux-followup/00-INDEX.md` — 40 open: 31 P1 + 9 P2)
- VCG auction still dominated by greedy path (matches CLAUDE.md "Partial" today); ExtensionChain formalization; hardcoded `tmp/`/absolute paths in source; Codex/Cursor backend parity (6); TUI event parity (7, incl. incremental tail-read + learning-data watcher); stale-doc sweeps (5); session-state migration framework; per-gate timeline widget; runner hardening (3); MCP coverage audit / Phase-2 crates (4)

### F. Deferred/blocked (MASTER-TASKS §7 — all still open per CLAUDE.md items 13–16 + roko-dreams row)
- Chain runtime integration; dreams cron trigger; cold substrate archival; knowledge-informed model routing; UX34 force_backend override learning

### G. Task inventories needing wholesale re-triage (not individually verified here)
- `prds/impl2/`: 46 gap-fix tasks across 6 PRDs (chain 7, config unification 12, event bridge 6, gates/safety/supervisor 7, learning/neuro 5, dead code/backends 9)
- `productionizing/`: P1–P18, F1–F7, D1–D9 (34 tasks)
- `binary-issues/MASTER-INDEX.md`: 90+ issues; runner closed only 3/56 batches
- `visual-gate2/`: 10-PRD unified evaluation architecture — designed, unbuilt
- `backlog/removed-learn-modules.md`: 8 modules / 4,808 LOC recoverable from `wp-arch2`

## Stale references

1. **CLAUDE.md:225** → `tmp/ux-followup/05-partially-wired-subsystems.md` — **dir no longer exists**; file is at `tmp/ux/ux-followup/05-partially-wired-subsystems.md`
2. **CLAUDE.md:307** → `tmp/ux-followup/00-INDEX.md` — same move: `tmp/ux/ux-followup/00-INDEX.md`
3. **CLAUDE.md:244** → `tmp/MASTER-TASKS.md` billed as "Master task list" — exists but frozen 2026-04-26 (pre-v2); §1 demo tasks expired, §5 superseded
4. `tmp/unified-migration/00-INDEX.md` → `tmp/roko-trustworthy/AUDIT.md` — moved to `tmp/archive/roko-trustworthy/`
5. `tmp/unified-migration/00-INDEX.md` → `tmp/unified-depth/` "(in progress)" — dir is empty; depth material lives in `tmp/archive/depth-v1/` and the later `tmp/unified/` spec (2026-05-16)
6. `tmp/acp-features/00-ACP-FEATURES.md` → `nunchi-dashboard/tmp/ux-refresh-context/` (external repo, unverifiable here) and `tmp/archive/workflow-v1/`
7. `tmp/runners/binary-issues/ISSUE-TRACKER.md` scope note: items inside `#[cfg(feature = "legacy-orchestrate")]` were declared obsolete — that cfg boundary itself needs re-checking post-v2

## Cleanup checklist

- [ ] **[P2]** Fix CLAUDE.md lines 225 + 307: `tmp/ux-followup/` → `tmp/ux/ux-followup/` — verify: `grep -n 'tmp/ux-followup' CLAUDE.md` returns nothing
- [ ] **[P2]** Regenerate `tmp/MASTER-TASKS.md` from `tmp/status-quo/12-ROADMAP.md` (or repoint CLAUDE.md:244 at the roadmap) — verify: file header date ≥ 2026-07
- [ ] **[P2]** Re-verify the 5 BI_SEC items + v3 security findings against current `roko-serve`/`share.rs`; carry unfixed ones into the new roadmap before archiving `binary-issues/` — verify: each has a code-cited closure or a new tracker row
- [ ] **[P2]** Re-verify §B V2-path findings (task.verify, [[gate]] arrays, episode logging, adaptive thresholds, model-selection split) against the current engine — verify: `roko plan run` with a `[[task.verify]]` task actually executes the verify command
- [ ] **[P2]** Triage `prds/impl2/` (46 tasks) + `productionizing/` (P/F/D plans) vs v2 code; mark each done/obsolete/carried — verify: annotated copy or roadmap rows exist
- [ ] **[P3]** Delete empty dirs `tmp/unified-depth/`, `tmp/gateway/` — verify: `ls -A` empty before removal
- [ ] **[P3]** Delete build artifacts `tmp/demo-current/` (dist/.vite/tsbuildinfo) and `tmp/demo-app-backup/` — verify: nothing references them (`grep -rn 'demo-current\|demo-app-backup' tmp/*.md CLAUDE.md`)
- [ ] **[P3]** Move to `tmp/archive/`: `workflow/`, `unified-migration/`, `unified-migration-runner/`, `mori-diffs/`, `learnings2/`, `ressearch2/`, `research/`, `dogfood/`, `demo-new/`, `demo-req/`, `demo-uis/`, `demo-redesign/` (after §C re-verify), `acp-runner/`, `architecture-archive/`, `scratchpad.md`, spent `runners/` subdirs — verify: `grep -rn 'tmp/<name>' CLAUDE.md docs/ tmp/status-quo/` shows no live inbound refs
- [ ] **[P3]** Confirm all 10 `tmp/audit-patches/*.patch` are merged, then delete — verify: `git apply --check` fails (already applied) or matching commits found
- [ ] **[P3]** Keep in place as living reference: `design-systems/`, `learnings3/`, `visual-gate2/`, `daeji/`, `agentchain-v2/`, `acp-features/`, `backlog/`, `binary-issues/` (until migrated), `ux/ux-followup/` (until migrated); consider promoting `demo-resources/` smoke scripts to `scripts/`

## Cross-cutting drift for the navigation layer

- **This document is the only live pointer** to ~130 open work items (40 `ux/ux-followup` + 46 `impl2` + 34 `productionizing` P/F/D + 90+ `binary-issues` + 10 `visual-gate2` PRDs + 8→7 `backlog` modules). None are tracked in `.roko/GAPS.md` (frozen 2026-05-05) or CLAUDE.md. If the roadmap doesn't absorb them, they vanish when tmp/ is archived.
- CLAUDE.md's two broken `tmp/ux-followup/` refs (L225, L307) and stale "Master task list" pointer (L244 → frozen 2026-04-26 file) remain the concrete nav-layer bugs.
- The legacy security P0s split cleanly now: 1 fixed (BI_01/gists), 1 needs re-verify (share-create auth), 3 carry forward (BI_02-05). The nav layer should track these as a live security row, not bury them in an archive doc.

## Open questions

- Did the May 6 a16z pitch outcome get recorded anywhere? Disposal of `ressearch2/`, `demo-req/`, `dogfood/` deck items depends on whether that material is still live for later raises.
- How much of unified-migration Phase 3 (Economy: demurrage, tiers, VCG, on-chain registries) did `tmp/v2-refactoring/` actually execute? GAPS.md covers Graph-engine residue but is silent on Economy.
- Are the 40 `ux/ux-followup` open items and 46 `impl2` tasks tracked anywhere post-v2, or is this document now the only live pointer to them?
- Is `visual-gate2/` still the intended gate evolution, or did the v2 `Verify` protocol absorb/obsolete it?
- The binary-issues runner stopped at 3/56 batches — deliberate abandonment in favor of the v2 refactor, or interrupted work that should resume?
- `tmp/prds/` PRD-01…10 vs the newer `tmp/unified/` spec (May 16): which is canonical for the roadmap's architecture targets?
