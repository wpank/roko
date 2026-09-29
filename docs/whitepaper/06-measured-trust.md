Status: stub · budget 450 words · owner gap-424bf8

# 6 Measured trust

[[TODO: Write this section to gap-424bf8's plan, in about 450 words. It contains "false-green rate", and each measure is defined with its denominator and tagged. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| MT1 | design | Routing learned from your own verified outcomes, across vendors: what it counts and its denominator. Today the router is PARTIAL (its LinUCB stage learns from successes only), and M3 routing on verified labels is MISSING. | tldr/00 point 9; tldr/01 selling point 3; B3 | Tags from the appendix; bug-f68404; bug-8da8ba; spec-6ac537 (E17) |
| MT2 | design | A false-green rate estimated from random audits with hidden tests, reported with a confidence interval (M4). | tldr/01 selling point 5; tldr/05 proposal 17; S05 | A MISSING tag; spec-6ac537 (E17) |
| MT3 | design | Per-loop evidence that learning helps: exposure, influence and benefit for each loop, against a withheld control (M2). | tldr/01 selling point 6; tldr/05 proposal 20; S03 | A MISSING tag; spec-6ac537 (E17) |
| MT4 | number | What exists today: honest per-task verdicts since the 09-28 fix, with 0 false greens in 151 passes, from the visible verify only. That is not an audited false-green rate. | B7 | Footnote as for N4 |
| MT5 | lit | As documented on 2026-09-29, we found no product that documents these three measures; several pair a frontier planner with a cheaper executor. No claim of firstness. | C3 (vendor docs fetched 2026-09-29) | An `@online` entry with `urldate` for each vendor page |
| MT6 | design | Pointers: §5 for the mechanisms, §8 for how they will be evaluated. | This README | Section cross-references |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/00: `tmp/cybernetic-harness/tldr/00-README.md`. The ten things to know; the status-tag vocabulary.
- tldr/01: `tmp/cybernetic-harness/tldr/01-WHAT-AND-WHY.md`. The idea, the bet, where it stands, selling points, positioning.
- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- B3: `tmp/cybernetic-harness/tldr/research/B3-routing-cost.md`. Routing and cost in the code.
- B7: `tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md`. Real-run evidence: the portal build, false greens, concurrency.
- C3: `tmp/cybernetic-harness/tldr/research/C3-competitive-landscape.md`. Competitors, from vendor docs fetched 2026-09-29.
- S01–S11: `tmp/cybernetic-harness/specs/`. Programme specs; cite the tracked epics that carry them.
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
