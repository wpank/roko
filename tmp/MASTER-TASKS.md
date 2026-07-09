# Master Task List

> Single source of truth for all open work. ~150 lines, 7 sections.
> Each item cites its source file. Updated 2026-04-26.

---

## 1. Demo & Pitch (deadline: May 6)

Source: [`dogfood/09-MAY6-DEMO-BUILD.md`](dogfood/09-MAY6-DEMO-BUILD.md),
[`dogfood/11-LANDING-PAGE-UPDATES.md`](dogfood/11-LANDING-PAGE-UPDATES.md),
[`dogfood/12-DECK-AND-MEMO.md`](dogfood/12-DECK-AND-MEMO.md)

**P0 — Demo-critical (must work May 5)**
- [ ] P0-1: `nunchi` CLI wrapper binary (shell script or symlink)
- [ ] P0-2: `nunchi agents list` with identity display (Clack-style output)
- [ ] P0-3: `nunchi audit` command — scripted demo showing identity/predict/gates/knowledge
- [ ] P0-4: `nunchi resume` command — crash recovery from checkpoint
- [ ] P0-5: `nunchi replay` command — JSON audit trail stream
- [ ] P0-6: Pre-warm LLM cache for deterministic demo (cached responses or demo-magic)
- [ ] P0-7: Demo backup tiers (asciinema recording, MP4, screenshots)

**P1 — Strong-to-have**
- [ ] P1-1: TUI streaming (agent output visible during execution)
- [ ] P1-2: TOML markdown fence stripping in enrichment path
- [ ] P1-3: Memory leak investigation (9.5GB RSS after 17 min)

**P2 — Polish (meeting week)**
- [ ] P2-1: 13-slide deck as PDF (Figma/Keynote → PDF, send May 1)
- [ ] P2-2: Pre-read memo (2,000 words, 10-section Kirwin template, send May 1)
- [ ] P2-3: Design partner outreach (Hebbia, Harvey, Decagon)
- [ ] P2-4: Landing page updates (remove mock data, add /changelog, add /docs, update hero)

---

## 2. Runtime Bugs (from dogfood)

Source: [`dogfood/00-INDEX.md`](dogfood/00-INDEX.md) — P1/P2/P3 open items

- [ ] **#9** Enrichment timeout hardcode — one 120s hardcode remains in gate judge call
- [ ] **M1** No streaming in non-approval path — runner v2 fixes this but only covers `--approval`
- [ ] **M2** Model shows "-" in TUI — runner v2 passes `model: String::new()` instead of resolved model
- [ ] **F5** Memory leak — unbounded `efficiency_events: Vec` never drained (9.5GB RSS)
- [ ] **#12** Knowledge endpoint URL mismatch — uses `/neuro/` path, not `/knowledge/`
- [ ] **#15** Enrichment artifacts empty — moot with `skip_enrichment` but unresolved
- [ ] **S4** signals.jsonl dead path — conductor writes to `engrams.jsonl` instead
- [ ] **S7** learn/ files stale in runner v2 — cascade-router.json and gate-thresholds.json not updated

---

## 3. Runner v2 Completion

Source: [`dogfood/00-INDEX.md`](dogfood/00-INDEX.md) — "Rewrite: Plan Runner v2" section

**Phases**
- [ ] Phase C: Make runner v2 the default for all `plan run` (non-approval path still uses orchestrate.rs)
- [ ] Phase D: Deprecate orchestrate.rs → `orchestrate_legacy.rs`
- [ ] Phase E: Align with unified spec (type renames, Activity recording)

**Wiring gaps** (runner v2 vs orchestrate.rs)
- [ ] CascadeRouter persistence — does not update `cascade-router.json`
- [ ] AdaptiveThresholds persistence — does not update `gate-thresholds.json`
- [ ] Replan-on-gate-failure — not wired in runner v2
- [ ] Model field not forwarded to `tui.agent_spawned()` despite being resolved

---

## 4. UX / Wiring (40 open items)

Source: [`ux/ux-followup/00-INDEX.md`](ux/ux-followup/00-INDEX.md)
— 112 entries total, 72 DONE, 40 open (31 P1 + 9 P2)

**High-value items**
- [ ] CognitiveWorkspace VCG auction — `vcg_allocate` built but greedy path dominates
- [ ] ExtensionChain — formalize 8 layers, wire into orchestrate.rs
- [ ] Hardcoded paths — several `tmp/` and absolute paths in source need fixing

**By catalog file** (see source for full details)
- `03` non-batch-followups: 1 open (snapshot tests)
- `04` t9-t19-residuals: 3 open (runner hardening)
- `05` partially-wired: 4 open (Phase 2+ crates, MCP audit)
- `06` agent-backends: 6 open (Codex/Cursor/cascade-router parity)
- `07` spec-code-drift: 3 open (docs sweeps)
- `08` phase-2-vision: 6 open (chain/dreams/full-TUI/HTTP roadmap)
- `09` hygiene: 3 open (clippy docs, flaky tests, cascade router e2e)
- `10` stale-docs: 5 open (terminology renames, old paths, banners)
- `12` tui-event-parity: 7 open (incremental tail-read, learning-data watcher)
- `13` session-state-mgmt: 1 open (migration framework)
- `14` observability: 1 open (per-gate timeline widget)

---

## 5. Spec Migration (reference only)

Source: [`unified-migration-runner/MASTER-CHECKLIST.md`](unified-migration-runner/MASTER-CHECKLIST.md)
— 95 batches across 4 phases (0 done, 78 pending, 17 blocked)

- Phase 0 (Prep): 10 batches — baseline, stubs, wiring
- Phase 1 (Kernel): 27 batches — type renames, Pulse/Bus, React, Cell, demurrage
- Phase 2 (Engine): 27 batches — Graph executor, agent runtime, CognitiveWorkspace, surfaces
- Phase 3 (Economy): 31 batches — CaMeL IFC, corrigibility, on-chain registries, arena, brain export

4-agent parallel execution plan with crate partitioning. Full details in source file.

---

## 6. Gap-Fix PRDs (46 tasks, reference only)

Source: [`prds/impl2/00-INDEX.md`](prds/impl2/00-INDEX.md)
— 6 PRDs, 46 tasks, audit date 2026-04-22

| # | PRD | Tasks | Priority |
|---|-----|-------|----------|
| 01 | Chain integration | 7 | 1 (root) |
| 02 | Config unification | 12 | 1 (root) |
| 03 | Event bridge + serve gaps | 6 | 2 |
| 04 | Gates / safety / supervisor | 7 | 2 |
| 05 | Learning / neuro corrections | 5 | 3 |
| 07 | Dead code + backend gaps | 9 | 3 |

PRDs 01+02 are roots; 03+04 depend on 02; 05+07 are independent.

---

## 7. Deferred / Blocked

- [ ] **Chain runtime integration** — blocked on chain backend (Phase 3+)
- [ ] **Dreams cron trigger** — dream consolidation built but no automatic scheduling
- [ ] **Cold substrate archival** — built but not instantiated at runtime
- [ ] **Knowledge-informed model routing** — neuro store not consulted for CascadeRouter selection
- [ ] **UX34: force_backend override learning** — cascade router doesn't learn from manual overrides
