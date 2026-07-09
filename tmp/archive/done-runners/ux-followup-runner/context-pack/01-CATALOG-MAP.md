# Catalog Map — Batch ↔ `tmp/ux-followup/` item

Every batch closes at least one open item from the post-PR-13 catalogue.
Use this map to:

- confirm your batch's scope matches the catalog entry before coding
- cite the catalog item in follow-up notes
- cross-check dependencies (items in files 15 ⇄ 05 ⇄ 12 frequently cross-ref)

## Forward map (batch → catalog items)

| Batch | Catalog refs | Files | Severity | Headline |
|-------|--------------|-------|----------|----------|
| UX01 | 06, 89       | 02, 15 | **P0** | Gate-failure → PlanRevision + plan-generator feedback |
| UX02 | 05, 51, 90   | 02, 07, 15 | **P0** | PrdPublished event + orchestrator subscriber |
| UX03 | 60, 26, 43   | 09, 04 | P1 | E2E self-hosting smoke test |
| UX04 | 12           | 02 | P1 | `roko plan validate` command |
| UX05 | 68           | 12 | **P0** | Standalone TUI subscribes to in-process StateHub |
| UX06 | 69, 76       | 12 | **P0** | Notify file-watcher replaces 500 ms polling |
| UX07 | 71, 73, 74   | 12 | P1 | Incremental JSONL tailers |
| UX08 | 72           | 12 | P1 | Task-output directory watcher |
| UX09 | 70           | 12 | P1 | Agent /stream WS consumer |
| UX10 | 75           | 12 | P2 | Git fs-watch replaces 3 s polling |
| UX11 | 77, 78       | 12 | P1 | Channel backpressure + durable gen counter |
| UX12 | 79, 81, 60d  | 13, 09 | P1 | Snapshot schema_version + migrate |
| UX13 | 82           | 13 | P1 | Resume validation vs plan discovery |
| UX14 | 80, 60e, 18  | 13, 09, 03 | P1 | ProcessSupervisor escalation + Drop |
| UX15 | 35c, 83, 87  | 05, 14 | P1 | Verdicts reader + gate trend widget |
| UX16 | 10, 31, 84   | 02, 05, 14 | P1 | Conductor diagnosis panel + endpoint |
| UX17 | 85           | 14 | P1 | Efficiency trend aggregator |
| UX18 | 35, 86       | 05, 14 | P1 | Metrics schema alignment |
| UX19 | 20, 88       | 03, 14 | P1 | Prompt experiments widget |
| UX20 | 14           | 02 | P1 | Agent topology widget |
| UX21 | 13           | 02 | P1 | Sidecar /logs + proxy |
| UX22 | 09           | 02 | P1 | C-factor trend endpoint + widget |
| UX23 | 35a          | 05 | P1 | Gate rungs: wire remaining 4 |
| UX24 | 35b, 94      | 05, 15 | P1 | Playbook store query seam |
| UX25 | 11, 30, 93   | 02, 05, 15 | P1 | HDC fingerprint per episode |
| UX26 | 35d, 91      | 05, 15 | P1 | Safety contract enforcement |
| UX27 | 35e, 92      | 05, 15 | P1 | Role-based tool whitelist |
| UX28 | 29, 95       | 05, 15 | P1 | Enrichment Enriching-phase wiring |
| UX29 | 32, 33, 34   | 05 | P1 | Phase-2 build-surface reality + MCP audit |
| UX30 | 36           | 06 | P1 | Codex conformance harness |
| UX31 | 37           | 06 | P1 | Cursor streaming path |
| UX32 | 38           | 06 | P1 | Cross-backend test parity |
| UX33 | 39, 40       | 06 | P1 | ExecAgent consolidation + dir cleanup |
| UX34 | 40a, 60c     | 06, 09 | P1 | Cascade router integration tests |
| UX35 | 08, 48a      | 02, 07 | P1 | Adaptive threshold load path |
| UX36 | 48b          | 07 | P1 | roko.toml unused keys |
| UX37 | 19           | 03 | P1 | SystemPromptBuilder snapshot tests |
| UX38 | 55           | 09 | P1 | Top-10 unwrap cleanup |
| UX39 | 60a          | 09 | P1 | HTTP validation + OpenAPI |
| UX40 | 60b          | 09 | P1 | Episode backend field |
| UX41 | 59           | 09 | P1 | Coverage scaffold |
| UX42 | 56, 58       | 09 | P1 | clippy + timeout flake audit |
| UX43 | 46           | 07 | P1 | MORI parity mechanical regen |
| UX44 | 45           | 07 | P1 | CLAUDE.md smoke tests |
| UX45 | 47, 64, 65, 66 | 07, 10 | P1 | Terminology + stale-snapshot sidecar |
| UX46 | 67, 67a      | 10 | P1 | implementation-plans + MORI paths |
| UX47 | 27, 28, 27a, 28a | 04 | P1 | tui-parity runner hardening |

## Reverse map (catalog item → batch)

| Item | File | Batch | Title |
|------|------|-------|-------|
| 05  | 02 | UX02 | auto-plan (CLI half already DONE; UX02 closes orchestrator half) |
| 06  | 02 | UX01 | gate-failure feedback loop |
| 08  | 02 | UX35 | adaptive threshold load path |
| 09  | 02 | UX22 | c-factor trend |
| 10  | 02 | UX16 | diagnosis panel |
| 11  | 02 | UX25 | HDC fingerprint (canonical: UX25) |
| 12  | 02 | UX04 | plan validate CLI |
| 13  | 02 | UX21 | sidecar /logs |
| 14  | 02 | UX20 | topology widget |
| 18  | 03 | UX14 | CancellationToken plumbing |
| 19  | 03 | UX37 | SystemPromptBuilder snapshot tests |
| 20  | 03 | UX19 | experiments widget |
| 26  | 04 | UX03 | e2e smoke (via gate re-verify) |
| 27  | 04 | UX47 | runner hardening (BATCHES.md drift) |
| 27a | 04 | UX47 | runner log retention |
| 28  | 04 | UX47 | runner env knobs |
| 28a | 04 | UX47 | runner CI dry-run |
| 29  | 05 | UX28 | Enriching-phase enrichment wiring |
| 30  | 05 | UX25 | HDC per episode |
| 31  | 05 | UX16 | conductor diagnosis |
| 32  | 05 | UX29 | roko-dreams build-surface reality |
| 33  | 05 | UX29 | roko-daimon/chain build-surface reality |
| 34  | 05 | UX29 | MCP servers audit |
| 35  | 05 | UX18 | obs metrics coverage |
| 35a | 05 | UX23 | gate rungs |
| 35b | 05 | UX24 | playbook query |
| 35c | 05 | UX15 | verdicts reader |
| 35d | 05 | UX26 | safety contract |
| 35e | 05 | UX27 | role whitelist |
| 36  | 06 | UX30 | Codex conformance |
| 37  | 06 | UX31 | Cursor streaming |
| 38  | 06 | UX32 | backend test parity |
| 39  | 06 | UX33 | ExecAgent consolidation |
| 40  | 06 | UX33 | Gemini/Perplexity/Ollama dir unification |
| 40a | 06 | UX34 | cascade router tests |
| 45  | 07 | UX44 | CLAUDE.md smoke tests |
| 46  | 07 | UX43 | MORI parity regen |
| 47  | 07 | UX45 | stale-snapshot sidecar |
| 48a | 07 | UX35 | adaptive threshold load |
| 48b | 07 | UX36 | roko.toml unused keys |
| 48c | 07 | (UX01+UX02) | self-hosting 10-11 explicit ack |
| 51  | 07 | UX02 | auto-plan orchestrator ack |
| 55  | 09 | UX38 | unwrap cleanup |
| 56  | 09 | UX42 | clippy missing_* |
| 58  | 09 | UX42 | timeout flake audit |
| 59  | 09 | UX41 | coverage scaffold |
| 60  | 09 | UX03 | e2e smoke test |
| 60a | 09 | UX39 | HTTP validation |
| 60b | 09 | UX40 | episode backend field |
| 60c | 09 | UX34 | cascade router tests |
| 60d | 09 | UX12 | snapshot schema_version |
| 60e | 09 | UX14 | supervisor escalation |
| 64  | 10 | UX45 | stale-snapshot sidecar |
| 65  | 10 | UX45 | terminology sweep |
| 66  | 10 | UX45 | death concept removal |
| 67  | 10 | UX46 | MORI paths |
| 67a | 10 | UX46 | implementation-plans refresh |
| 68  | 12 | UX05 | unconditional StateHub |
| 69  | 12 | UX06 | file-watcher |
| 70  | 12 | UX09 | agent WS consumer |
| 71  | 12 | UX07 | signal tail reader |
| 72  | 12 | UX08 | task-output watcher |
| 73  | 12 | UX07 | episode tail reader |
| 74  | 12 | UX07 | event log push |
| 75  | 12 | UX10 | git watch |
| 76  | 12 | UX06 | learning file watch |
| 77  | 12 | UX11 | channel backpressure |
| 78  | 12 | UX11 | durable gen counter |
| 79  | 13 | UX12 | snapshot version |
| 80  | 13 | UX14 | zombie cleanup |
| 81  | 13 | UX12 | migration framework |
| 82  | 13 | UX13 | resume validation |
| 83  | 14 | UX15 | verdicts widget |
| 84  | 14 | UX16 | diagnosis widget |
| 85  | 14 | UX17 | efficiency aggregator |
| 86  | 14 | UX18 | metrics schema |
| 87  | 14 | UX15 | gate timeline |
| 88  | 14 | UX19 | experiments widget |
| 89  | 15 | UX01 | gate-feedback loop |
| 90  | 15 | UX02 | PRD publish event |
| 91  | 15 | UX26 | safety contracts |
| 92  | 15 | UX27 | role whitelist |
| 93  | 15 | UX25 | HDC per-episode (canonical home) |
| 94  | 15 | UX24 | playbook query (canonical home) |
| 95  | 15 | UX28 | Enriching-phase enrichment wiring (canonical home) |

## Coverage summary

- 82 open non-P2 items → 47 batches (≈ 1.7 items / batch average)
- 6 P2 items from file 08 — **not covered**; parked until P0/P1 work is green
- 24 items marked `[DONE]` in the catalogue — **not touched**; they remain
  as evidence of PR #13 completion
