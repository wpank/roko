Status: stub · budget 550 words · owner gap-353d57

# 1 Introduction

[[TODO: Write this section to gap-353d57's plan, in about 550 words. Every number has a footnote naming its source, and the numbers match §7. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| IN1 | lit | Frontier models are capable but expensive; cheap models are unreliable on their own, and success falls as tasks get longer. | draft §1.1 "Capable or cheap"; C1 | [@kwa2025measuring]; [@sinha2025illusion] |
| IN2 | lit | The bet: on decomposable, checkable work, most of the dependability comes from the harness (precise specs, executable checks, retries, escalation, integration gates, feedback). The literature supports about frontier quality at 2–6× lower cost there, mostly measured on single-function, QA and document tasks, and not on sequential or integrative work. | tldr/01 "The bet"; C1 TL;DR | [@kim2025towards], plus the studies behind the range (`narayan2025minions`, `chen2024frugalgpt`, `ong2025routellm`; add them after refcheck). The 2–6× range is C1's synthesis: cite the studies, not the range |
| IN3 | status | Built: the Graph engine, durable checkpoints and resume, and honest per-task verdicts since the 09-28 fix. | tldr/00 point 1; tldr/03 "Execution", "Verification and QA" | WIRED tags from the appendix; `crates/roko-graph/src/engine.rs`; N4 |
| IN4 | status | Designed, not built: the tier ladder, escalation, integration on the Graph path, and audits. The text says "is designed to". | tldr/04 steps 4, 8 and 9; tldr/03 | Tags from the appendix; spec-98f76d (E5), spec-a0e40a (E6), spec-6ac537 (E17) |
| IN5 | number | What Roko has built: the portal (N1, N2), on one pinned model (N3). | B7 | Footnotes as for N1–N3, the same figures as §7 |
| IN6 | case | What the supervising session did: a frontier Claude Code session with about 20 subagents wrote and audited the plans, fixed about 25 engine defects, set up isolation, merged and checked the whole product. Its cost is outside Roko's record (N6). | tldr/01 "Where it stands"; B7; W12 | Frozen CASE-005 and CASE-007; N6, labelled an estimate |
| IN7 | design | The contributions, and a map of §2–§10. | This README's outline | Section cross-references |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- draft §N: `tmp/cybernetic-harness/paper/sections/`. The research draft's sections; outline in `paper/OUTLINE.md`, conventions in `paper/00-README.md`.
- tldr/00: `tmp/cybernetic-harness/tldr/00-README.md`. The ten things to know; the status-tag vocabulary.
- tldr/01: `tmp/cybernetic-harness/tldr/01-WHAT-AND-WHY.md`. The idea, the bet, where it stands, selling points, positioning.
- tldr/03: `tmp/cybernetic-harness/tldr/03-MECHANISMS.md`. Every mechanism with basis, status and verdict.
- tldr/04: `tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md`. The 11-step loop, the eight design rules, the real-run numbers, the three-arm test.
- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- C1: `tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md`. Literature: planning, decomposition, cascades, integration.
- W9: `tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md`. Rules for honest ideal-state writing; the phantom identifiers.
- W12: `tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md`. The operator-loop cost estimate.
- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
