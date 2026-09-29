Status: stub · budget 700 words · owner gap-29a64e

# 7 Field evidence

[[TODO: Write this section to gap-29a64e's plan, in about 700 words. Waits for: bug-7b37c4 (E13.1). It contains "observational", and every number traces to a frozen rollup, a snapshot or a commit, with its scope. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| FE1 | number | The portal build: plans, tasks, tests, attempts, first-try passes, spend and concurrency, all from one frozen source cited by its generation time and sha256. | B7; ROLLUP | A file in `evidence/`; N1, N2, N5, N10 |
| FE2 | number | One model: all 210 portal attempts pinned `claude-sonnet-4-6`. | B7 | N3 |
| FE3 | case | CASE-001: a failed task logged as passed, then honest verdicts (N4). | CASES; B7 | The frozen case; the fix items bug-82d47b, bug-521f08, bug-06e2d1 |
| FE4 | case | CASE-005: parallel lanes, worktrees and merges made by hand. | CASES; B7 | The frozen case; resolve N12 first |
| FE5 | case | CASE-006: gates green, product unusable: the browser-pass loop (N11). | CASES; B7 | The frozen case |
| FE6 | case | CASE-007: plan defects that only frontier audits caught. | CASES; B7 | The frozen case |
| FE7 | number | The operator loop and its cost: about $2.7–3.4k API-equivalent for 09-25 to 09-29, about 16–20× Roko's recorded $172.80 (N6). Use E13's harvested numbers instead if gap-263de5 and gap-ccb87e have landed. | W12; spec-f2463d (E13) | The frozen W12 table, labelled an estimate |
| FE8 | number | The rollup over every captured run: 42 runs, $192.92 recorded, $1.22 per verified task. Its autonomy index is overstated until bug-7b37c4 lands (N7). | ROLLUP | The frozen rollup and its sha256 |
| FE9 | scope | Everything in the section is observational: no causal claim, and no comparison with the benchmark. | The field README; gap-29a64e | The word "observational" in the text |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- ROLLUP: `tmp/cybernetic-harness/evidence/field/ROLLUP.md`. Field rollup (with `rollup.json`), generated 2026-09-29T11:14:31.
- CASES: `tmp/cybernetic-harness/evidence/field/CASES.md`. Field cases CASE-001 to CASE-008.
- W12: `tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md`. The operator-loop cost estimate.
- tldr/01: `tmp/cybernetic-harness/tldr/01-WHAT-AND-WHY.md`. The idea, the bet, where it stands, selling points, positioning.
- tldr/04: `tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md`. The 11-step loop, the eight design rules, the real-run numbers, the three-arm test.
