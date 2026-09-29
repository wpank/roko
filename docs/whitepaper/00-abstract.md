Status: stub · budget 150 words · owner gap-353d57

# Abstract

[[TODO: Write this section to gap-353d57's plan, in about 150 words. Waits for: §1–§9 drafted. Its numbers match §1 and §7. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| AB1 | design | Roko runs spec'd agent work: a frontier model writes a plan of small tasks, each with its own executable check, and Roko is designed to run them in parallel on the cheapest model that passes, verify each task and the whole plan, and learn from verified outcomes. | PLAN §1 (thesis); tldr/01 "The idea" | §4 and §5; "is designed to" for anything not WIRED |
| AB2 | status | What runs today: the Graph engine with checkpoints and resume, and honest per-task verdicts. What is designed only: the tier ladder, escalation, integration and audits. | tldr/00 points 1 and 3–6 | Tags from the appendix, at its commit |
| AB3 | number | The portal build: 16 plans and 173 tasks (168 gate-verified) for $174.87 of recorded agent spend, excluding the supervising session. | B7 | Footnote as for N1 and N2 |
| AB4 | number | The cheap-model half of the thesis is untested: all 210 portal attempts pinned `claude-sonnet-4-6`. | B7; tldr/04 | Footnote as for N3; an UNPROVEN tag |
| AB5 | design | The evaluation tests "cheaper at equal quality": cost per verified task against Claude Code on Opus, on hidden-test tasks. | PLAN §1; §8 | §8; spec-567e52 |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- tldr/00: `tmp/cybernetic-harness/tldr/00-README.md`. The ten things to know; the status-tag vocabulary.
- tldr/01: `tmp/cybernetic-harness/tldr/01-WHAT-AND-WHY.md`. The idea, the bet, where it stands, selling points, positioning.
- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
