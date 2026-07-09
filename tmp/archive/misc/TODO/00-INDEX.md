# TODO Index — tmp/ Directory Audit

Audited 2026-04-20. **Complete coverage** of all `tmp/` contents (25 directories + 27 loose files).

## Status Legend

- **DONE** — Completed, no remaining action
- **STALE** — Outdated, checkboxes behind code reality
- **ACTIVE** — Has genuine remaining work
- **ARCHIVE** — Historical artifact, keep for reference

## Directory Status

| # | Directory | Status | Remaining Items | File |
|---|-----------|--------|-----------------|------|
| 1 | `docs-gaps/` | STALE | ~253 unchecked (many false; ~50 checkbox updates + ~200 genuine) | [01-docs-gaps.md](01-docs-gaps.md) |
| 2 | `demo/` | ACTIVE | End-to-end validation needed | [02-demo.md](02-demo.md) |
| 3 | `agent-registry/` | DONE | 1 minor docs gap (AR07 runbook) | [03-agent-registry.md](03-agent-registry.md) |
| 4 | `docs-parity/` | DONE | Reference artifact, accurate | [04-docs-parity.md](04-docs-parity.md) |
| 5 | `docs-parity-meta/` | DONE | Generator tool, reusable | [04-docs-parity.md](04-docs-parity.md) |
| 6 | `docs-parity2/` | DONE | 21/21 batches merged | [04-docs-parity.md](04-docs-parity.md) |
| 7 | `implementation-plans/` | ACTIVE | Critical wiring gaps in plans 01-03, 06 | [05-implementation-plans.md](05-implementation-plans.md) |
| 8 | `integrate-prds/` | ACTIVE | Phase B 85%, Phase C 15% | [06-integrate-prds.md](06-integrate-prds.md) |
| 9 | `new-docs/` | DONE | 554 files, accurate, current (2026-04-19) | [07-new-docs.md](07-new-docs.md) |
| 10 | `new-docs-section-00/` | DONE | 549 files, spec-grade reference | [07-new-docs.md](07-new-docs.md) |
| 11 | `prd-enhance-logs/` | ARCHIVE | Execution logs from Apr 13 enhancement run | [08-prd-enhance-logs.md](08-prd-enhance-logs.md) |
| 12 | `prd-migration/` | DONE | 22/22 topics, 422 files, 190K lines | [09-prd-migration.md](09-prd-migration.md) |
| 13 | `refinements/` | ACTIVE | 35 proposals, design complete, implementation blocked | [10-refinement-system.md](10-refinement-system.md) |
| 14 | `refinements-audit/` | DONE | Audit of 35 proposals; "ship now" list identified | [10-refinement-system.md](10-refinement-system.md) |
| 15 | `refinements-runner/` | DONE | 35/35 batches executed; branch pending merge | [10-refinement-system.md](10-refinement-system.md) |
| 16 | `refinement-audit-runner/` | ACTIVE | Phase 1 done, Phase 2 stuck on verify failures | [10-refinement-system.md](10-refinement-system.md) |
| 17 | `run-anywhere/` | DONE | 24-doc design encyclopedia, reference only | [11-run-anywhere.md](11-run-anywhere.md) |
| 18 | `sdb-spec/` | DONE | 10/10 dashboard specs implemented | [12-sdb-spec.md](12-sdb-spec.md) |
| 19 | `tui/` | DONE | 270 gap items audited; 243 fixed, 27 partial | [13-tui-system.md](13-tui-system.md) |
| 20 | `tui-parity/` | DONE | 19/19 batches merged | [13-tui-system.md](13-tui-system.md) |
| 21 | `ux/` | DONE | Architecture spec, ~90% of Phase 1 implemented | [14-ux-system.md](14-ux-system.md) |
| 22 | `ux-followup/` | ACTIVE | 72/112 done; 40 open (31 P1, 9 P2) | [14-ux-system.md](14-ux-system.md) |
| 23 | `ux-followup-runner/` | DONE | 47/47 batches executed | [14-ux-system.md](14-ux-system.md) |
| 24 | `ux-refactoring/` | DONE | 12/12 batches merged (~100 tasks) | [14-ux-system.md](14-ux-system.md) |
| 25 | Loose files (27) | ARCHIVE | Historical artifacts, runner scripts, PR drafts | [15-loose-files.md](15-loose-files.md) |

## Priority Actions

### P0 — Blocking self-hosting

1. Wire `SystemPromptBuilder` into `orchestrate.rs` (replace inline `build_system_prompt()`) — [05](05-implementation-plans.md)
2. Wire `ToolDispatcher` + `SafetyLayer` into orchestrate.rs agent dispatch — [05](05-implementation-plans.md)
3. Wire `ProcessSupervisor` into orchestrate.rs for PID tracking — [05](05-implementation-plans.md)

### P1 — Quality / correctness

4. Update ~50 stale checkboxes in `docs-gaps/` where code exists but wasn't marked — [01](01-docs-gaps.md)
5. Verify remaining ~200 genuinely unchecked `docs-gaps/` items — [01](01-docs-gaps.md)
6. Run demo end-to-end on live chain (T1.3 Solidity, TUI event loop) — [02](02-demo.md)
7. Persist AR07 remote demo runbook — [03](03-agent-registry.md)

### P1 — Refinement system

8. Ship "5 now" items from refinements-audit: HDC fingerprint on Engram, unified RokoEvent, Bus trait, Signal cleanup, INDEX fix — [10](10-refinement-system.md)
9. Review + rebase + merge refinements branch (`codex/refinements-run-20260416-221511`) — [10](10-refinement-system.md)
10. Unblock refinement-audit-runner Phase 2 (PU00-01 verify gate failures) — [10](10-refinement-system.md)

### P1 — UX follow-up (40 open items)

11. Wire remaining 4 gate rungs (FactCheck, Symbol, GeneratedTest, PropertyTest) — [14](14-ux-system.md)
12. TUI polling→push migration (7 items, Phase E) — [14](14-ux-system.md)
13. Stale docs: terminology, paths, banners (5 items) — [14](14-ux-system.md)
14. Advanced agent backends: Codex/Cursor/streaming parity (6 items) — [14](14-ux-system.md)

### P1 — TUI polish (27 partial items)

15. PostFX toggle via roko.toml + error logging completeness — [13](13-tui-system.md)

### P2 — Deferred / Phase 2+

16. Implementation plan 11 phases 3-8 (daemon, multi-repo, PRD workflow) — [05](05-implementation-plans.md)
17. Implementation plan 12a cognitive layer (distillation pipeline) — [05](05-implementation-plans.md)
18. Implementation plan 12b chain layer (deferred until Tier 1) — [05](05-implementation-plans.md)
19. `integrate-prds` Phase C new features (dreams, heartbeat, pheromones, mesh) — [06](06-integrate-prds.md)
20. `sdb-spec` post-demo items (stream CRUD, chat persistence, IPFS artifacts) — [12](12-sdb-spec.md)
21. `ux-followup` Phase 2 vision (6 items: chain, dreams, full TUI, HTTP auth, plugins) — [14](14-ux-system.md)
