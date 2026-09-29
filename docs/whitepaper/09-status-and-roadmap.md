Status: stub · budget 550 words · owner gap-c19902

# 9 Status, limitations and roadmap

[[TODO: Write this section to gap-c19902's plan, in about 550 words. Waits for: gap-35a614. It links to `appendix-status-matrix.md`, and every limitation names its evidence or item. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| SR1 | status | The tag counts from the appendix, at its commit, and the ten vision claims (V1–V10) re-scored against them. | tldr/05 §1; the appendix | A link to `appendix-status-matrix.md` |
| SR2 | status | What moved after the tldr's `d9e79e9d8`: ready-queue scheduling, adaptive retry budgets, the cost of timed-out attempts, the child-environment allowlist, and re-wired learning loops. | README "Rows that moved" | Commits |
| SR3 | scope | Limitation: one harness, and one pinned model on every portal task (N3). | B7 | N3 |
| SR4 | scope | Limitation: the field evidence is observational, and supervising sessions did much of the work (N6). | B7; W12 | §7 |
| SR5 | scope | Limitation: code-first in practice; shell-command verifiers are the only domain-neutral check. | tldr/06; B8 | The tag from the appendix |
| SR6 | scope | Limitation: safety. There is no OS sandbox, and the git guard has gaps; the child-environment fix merged in `1d923e377` (bug-7d7200 closed). | tldr/03 "Isolation and integration"; B4 | Tags from the appendix; spec-ba7bea (E3) |
| SR7 | scope | Limitation: no learning loop has a measured benefit. | B5; tldr/00 point 6 | An UNPROVEN tag |
| SR8 | design | The roadmap by goal, in order: truth (E2–E4), golden path (E5–E11), proof (E12–E13), cybernetic core (E17), each with its epic ids and exit check. Give the order, not dates. | PLAN §2–§4; `work/goals.toml` | The epic items |
| SR9 | status | Proposed parking: the chain and marketplace code, offline batch consolidation, the conductor, affect and the extra knowledge-store modules, about 94k LOC by tldr/05's estimate (N13). Mark it as proposed. | tldr/05 §3 | N13, recomputed at a commit or labelled an estimate |
| SR10 | figure | Figure 3: the status matrix; gap-d1d92c draws it. | gap-d1d92c | `figures/fig3-status-matrix.svg` |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- tldr/03: `tmp/cybernetic-harness/tldr/03-MECHANISMS.md`. Every mechanism with basis, status and verdict.
- tldr/06: `tmp/cybernetic-harness/tldr/06-DOMAINS-AND-ASSISTANT.md`. Non-code domains and the assistant.
- B4: `tmp/cybernetic-harness/tldr/research/B4-gates-qa-safety.md`. Gates, QA and safety in the code.
- B5: `tmp/cybernetic-harness/tldr/research/B5-learning-loops-v2-vs-graph.md`. The 16 learning loops, Runner-v2 against Graph.
- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- B8: `tmp/cybernetic-harness/tldr/research/B8-domain-agnostic-and-steering.md`. Domains and steering.
- W12: `tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md`. The operator-loop cost estimate.
- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
