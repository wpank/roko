+++
id = "spec-f8d196"
kind = "spec"
title = "Epic: research paper, companion report and TL;DR upkeep"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "L"
subsystem = ["paper", "companion", "tldr"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/", "tmp/cybernetic-harness/companion-audit/", "tmp/cybernetic-harness/tldr/"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "paper"
links = { depends_on = ["gap-aad48c", "gap-e08b4d", "gap-ac4ce8", "gap-56a1b4", "gap-04b0ff", "gap-2abf34", "gap-c4d630", "gap-cdd5f4", "gap-652d05", "gap-b64fba", "gap-b409fa", "gap-420202", "gap-0ef29d", "gap-856053", "gap-987510", "gap-4d516a", "gap-b605cf", "gap-184da5", "gap-a3031b", "gap-3986d0", "gap-bb619d", "dec-536bbd", "gap-f3be74", "gap-cb86e4", "gap-9899b0", "gap-e2d092"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --report tmp/cybernetic-harness/paper/sections/*.md"
+++

## Problem

The research paper, the companion report and the tldr all live in untracked `tmp/cybernetic-harness/`, and none of
them had work items. The research draft (about 65k words) needs aligning with the golden-path thesis, trimming and
tightening before any data arrives (assessment W9). The companion report needs its E1 re-derivation and its main text.
The tldr needs refreshing after today's merges.

## Why it matters

Will asked on 2026-09-29 for the whitepaper and the research papers to be written in parallel with the Roko fixes.
The research paper follows the whitepaper and fills in data as the benchmark lands.

## Where

- `tmp/cybernetic-harness/paper/` (OUTLINE, sections, bibliography, CLAIMS-EVIDENCE);
- `companion-audit/`;
- `tldr/`.

## Current state

The draft is 1.9× over budget, with 231 `[[RESULT]]` and 56 `[[AS-BUILT]]` markers. No marker lint exists; the
whitepaper's paperlint (gap-af0b57) is built to serve both papers.

## Plan

This is the implementation plan.

1. **Align, trim and tag,** one agent per file group: the framing (outline, abstract, introduction, conclusion), §4,
   §5, §3, §8/§9, and appendices E/F.
2. **Tooling:** the claims aggregator. paperlint comes from gap-af0b57.
3. **Companion:** E1 re-derivation, then the E10 draft. Recruit the E2 and E3 raters (Will).
4. **tldr:** refresh it against today's code.
5. **Later, when data exists:** the notation table, the figure dry run, pre-registration, and filling results (W9
   PW08, PW10–PW14).

## Done when

- [x] gap-aad48c: Research paper: align the outline, abstract, introduction and conclusion with the golden-path thesis
- [x] gap-e08b4d: Research paper §4: rebuild §4.1 around the control stack and the golden-path loop, and tag designed identifiers
- [x] gap-ac4ce8: Research paper §5: record the decided evaluation scope and trim the protocol to budget
- [x] gap-56a1b4: Research paper §3: trim the related work to budget
- [x] gap-04b0ff: Research paper §8 and §9: the operator loop, field-evidence threats, and trims
- [x] gap-2abf34: Research paper appendices E and F: reproducibility formats and the cost of the supervising sessions
- [x] gap-c4d630: Research paper claims aggregator: regenerate CLAIMS-EVIDENCE.md from the section ledgers
- [x] gap-cdd5f4: Companion report: re-derive every number at the audit/baseline-2026-09-28 tag
- [x] gap-652d05: Companion report: draft the E10 main text with E1 number markers
- [x] gap-b64fba: Refresh the TL;DR against the 2026-09-29 merges and fix three known errors
- [x] gap-b409fa: Companion report: fill the re-derived numbers into the E10 draft and fix the flagged source errors
- [x] gap-420202: Research paper §6: align the economics results template with the thesis, add the plan-level template, and trim
- [x] gap-0ef29d: Research paper §7: fix the dormant-loop claim in §7.3, align with the thesis, and trim
- [x] gap-856053: Research paper appendices A and B: match the benchmark and the spec standard as built
- [x] gap-987510: Research paper appendices C and D: align the audit protocol and metrics with S05, S09 and the ViabilityBench analysis
- [x] gap-4d516a: TL;DR: fix four statements left stale after the 2026-09-29 refresh
- [x] gap-b605cf: Research paper: notation table, figure and table map, budget-line names and bibliography venues
- [x] gap-184da5: Research paper: cite the frozen rollup, B7, W12 and the field cases from docs/whitepaper/evidence/
- [x] gap-a3031b: Companion report E12: internal review and bibliography QA
- [x] gap-3986d0: Freeze a tarball of .roko state with each audit tag
- [x] gap-bb619d: Research paper: the final prior-art kill-search (Track D13) before submission
- [ ] dec-536bbd: Decide how paperlint --strict treats numbers in the claims-ledger rows
- [x] gap-f3be74: TL;DR: fix the stale statements found by the second refresh pass
- [ ] gap-cb86e4: Companion report: make the draft pass paperlint --strict apart from the rater and author markers
- [x] gap-9899b0: Research paper: cite the frozen evidence copies in §1.2, §4.1, §7 and Appendix F
- [x] gap-e2d092: Research paper: pick up S09 v1.2 and S01 v1.2 in §5, Appendices A, D and E, and fix S01 §4.5
- [ ] The `[[verify]]` command passes (paperlint reports on every section).

## Notes

- Human raters for companion E2 and E3 are still needed (Will).
- Status tags carry a commit. Results never replace a `[[RESULT]]` slot without an S09 bundle.
