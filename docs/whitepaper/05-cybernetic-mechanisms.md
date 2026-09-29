Status: stub · budget 850 words · owner gap-e8cb4d

# 5 Cybernetic mechanisms

[[TODO: Write this section to gap-e8cb4d's plan, in about 850 words. Waits for: gap-35a614. It contains "guarded commit", every mechanism has a tag that matches the appendix, and every designed identifier is marked (designed). Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| CM1 | design | The essential variables are the verified pass rate, cost per verified task, the false-green rate and latency. | tldr/02; tldr/05 proposal 22 | §2 (DP9) |
| CM2 | status | The learning loops (routing, retry feedback, failure memory, playbooks, prompt experiments, adaptive thresholds): what each senses, what it changes, and its tag. | tldr/03 "Learning and memory"; B5; README "Rows that moved" | Tags from the appendix; merges `ce3bdcbb8`, `33e107da1` |
| CM3 | number | Runner-v2 had 16 loops, and none was fully wired on Graph at `d9e79e9d8`; several were re-wired on 09-29. No loop has a measured benefit in either era. | B5; tldr/00 point 6; N8 | Commits; gap-fdd27f, reg-3f5969, gap-5fb9a7, reg-06ae9f; an UNPROVEN tag |
| CM4 | design | M1, a bounded controller over the essential variables, judged by disturbance tests: what it regulates, its sensor, actuator and bounds. | tldr/05 proposal 22; draft §4.6 and Table T1; S06 | A MISSING tag; spec-6ac537 (E17) |
| CM5 | design | M2, the loop-liveness audit: every loop shows exposure, influence and benefit against a withheld control, or it is demoted. | tldr/05 proposal 20; draft §4.5; S03 | A MISSING tag; spec-6ac537 (E17) |
| CM6 | design | M3, a calibrated self-model: it forecasts the pass probability per task and model, and runs in shadow until it is calibrated. | tldr/05 proposal 18; draft §4.3; S04 | A MISSING tag; spec-6ac537 (E17) |
| CM7 | design | M4, random deep audits: hidden tests written by another model family, an audit lottery, and a Hájek estimate of the false-green rate. | tldr/05 proposal 17; draft §4.4; S05 | A MISSING tag; spec-6ac537 (E17) |
| CM8 | design | Guarded commit: every self-modification (router, memory, controller) is accepted only after held-out and anchor checks, with rollback. | tldr/05 proposal 21; tldr/03 "Regulation and audits"; S06 | A MISSING tag; spec-6ac537 (E17) |
| CM9 | lit | The claim is measured, guarded improvement: in the literature, gains fail to compound, depend on task order and weaken under distribution shift. | tldr/01; tldr/05 §5; C4 | `wang2026compound`, `ye2026fragility`, `lin2026evopath`, `wang2026rethinking` (add them after refcheck) |
| CM10 | status | The analogy mechanisms appear in one sentence as proposals to park, not as features: affect, offline batch consolidation, HDC similarity and the conductor. | tldr/05 §3; B6; C4 | Tags from the appendix |
| CM11 | status | M1 does not revive the conductor's old tick, which caused restart storms. | tldr/05 proposal 22; A5 | The conductor's tag from the appendix |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/02: `tmp/cybernetic-harness/tldr/02-HOW-IT-WORKS.md`. Architecture, the control stack, what `.roko/` records.
- tldr/03: `tmp/cybernetic-harness/tldr/03-MECHANISMS.md`. Every mechanism with basis, status and verdict.
- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- B5: `tmp/cybernetic-harness/tldr/research/B5-learning-loops-v2-vs-graph.md`. The 16 learning loops, Runner-v2 against Graph.
- B6: `tmp/cybernetic-harness/tldr/research/B6-cognitive-subsystems.md`. The analogy subsystems.
- C4: `tmp/cybernetic-harness/tldr/research/C4-research-basis-cybernetic-loops.md`. Literature behind the loops; claims it contradicts.
- A5: `tmp/cybernetic-harness/tldr/research/A5-regulation-observability-docs.md`. Regulation and observability against docs/v3.
- draft §N: `tmp/cybernetic-harness/paper/sections/`. The research draft's sections; outline in `paper/OUTLINE.md`, conventions in `paper/00-README.md`.
- S01–S11: `tmp/cybernetic-harness/specs/`. Programme specs; cite the tracked epics that carry them.
- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
