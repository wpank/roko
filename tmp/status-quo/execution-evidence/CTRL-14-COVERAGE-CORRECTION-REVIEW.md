# CTRL-14 coverage correction — independent review

## Candidate and requirement

- Verdict: **REJECTED**
- Candidate: `beac93874e448a3e069f23978eabd23e5d9f8383`
- Implementation chain: `ad908c5af` followed by `beac93874`
- Base: `a4278ced0`
- Reviewed requirement: close the two nonterminal inventory findings recorded in `ed5ab0fed` without rewriting historical audits: include the canonical master in the source/DOC coverage layers, reconcile current 108→109 and 744→745 coverage claims, and make the executable plan index exactly match all 48 E manifests (169 E01-E18 tasks, 447 total).

The candidate changes exactly the declared five paths: four control files plus its evidence record. No audit file, product file, source document, generated top-level index, task status, or unrelated manifest changed. That scope check passes, but the correction is incomplete because a current navigation claim immediately above these canonical layers was left contradictory.

## Blocking finding

### F1 — Current backlog navigation still advertises 149 tasks and 744 sources

Severity: **medium; acceptance-blocking control-plane contradiction**.

`tmp/status-quo/backlog/00-INDEX.md` is the live “Navigation layer for the roko executable backlog,” not a July audit record. It has no historical/supersession fence and mixes current 447-task content with stale current descriptions:

- line 36 describes `05-MASTER-CHECKLIST.md` as 149 tasks;
- line 37 describes canonical ledger 06 as 149 materialized tasks;
- line 39 describes canonical ledger 08 as covering 744 sources;
- lines 167–171 say the current DOC source-corpus layer covers 744 sources.

The candidate's own evidence says current roll-ups now carry 745 and that 744 was preserved only in the historical July audit. The unfenced current index therefore disproves that statement and violates the original required reconciliation: change 744→745 everywhere it is presented as current coverage.

Reproduction:

```sh
rg -n '149 materialized|all 744|covers all \*\*744\*\*' \
  tmp/status-quo/backlog/00-INDEX.md
```

Actual: lines 37, 39, and 168 still match; line 36 separately retains the stale 149 checklist count. Expected: current navigation agrees with the parsed 169/447/745 truth, or the whole document is explicitly fenced as a historical baseline rather than mixing old and current claims.

Smallest correction:

1. Add `tmp/status-quo/backlog/00-INDEX.md` to the correction scope.
2. Reconcile its current navigation descriptions to 447 implementation tasks (169 for E01-E18 where relevant) and 745 source documents, including the source-corpus layer paragraph.
3. Update the worker evidence and its scope proof to include this file and explicitly say that ledger 06's warning-count paragraph is not repaired by this coverage candidate.
4. Rerun the same census, strict focused plan validation, sealed-index check, and independent review on the new immutable candidate.

## Passing independent checks

- Read the complete 1,164-line master, the prior nonterminal CTRL-14 finding (`ed5ab0fed`), both implementation commits/diffs, worker evidence, ledgers 06 and 08, the affected DOC manifest/ledger, and the complete 48-manifest set through structured TOML inspection.
- Exact diff scope from `a4278ced0..beac93874`: five expected paths only. The chain contains exactly two commits; audits are untouched; `git diff --check` passes.
- Parsed all 48 E manifests: every `meta.total` equals its task array, every `meta.done` equals parsed done statuses, IDs are globally unique, E01-E18 totals 169, and E01-E48 totals 447. Current status census is 7 done / 440 ready.
- Parsed every row of `backlog/plans/00-INDEX.md`: 48 unique E01-E48 rows, every directory/count matches its manifest, and the total is 447/0 gaps.
- Parsed all six DOC manifests: metadata matches arrays, 71 unique DOC tasks remain, and the modified DOC plan retains 12 tasks with unchanged metadata, IDs, and statuses; only DOC-SQ-01 changed from the base.
- Enumerated the published source glob: 745 sources, including 109 direct `tmp/status-quo/*.md`; zero sources are missing from source ledgers or DOC task manifests. The master occurs once in the source set, once in the status-quo ledger, once in DOC-SQ-01, and in no other DOC-SQ task.
- Executed DOC-SQ-01's structural verify command successfully.
- Ran the integrated validator (`git d4749f9c7`) from a disposable repository-shaped root: strict `DOC-status-quo-corpus` validation exits 0 with `0 diagnostics in 1 plan`.
- Source `plans/INDEX.md` remained byte-identical at SHA-256 `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`; the review worktree remained clean after validation.

## Explicit CTRL-07 fence

Ledger 06's “six expected PLAN_031 warnings” paragraph is independently stale: the same integrated validator reports 12 prerequisite diagnostics in 55 plans at this candidate. That count is assigned to CTRL-07 and is **not** claimed fixed or accepted by this coverage review. It is not a second coverage-candidate finding; this review accepts only ledger 06's manifest/task-count tables as matching 169+243+35=447 and leaves its validation paragraph to the separate CTRL-07 correction.

## Verdict

**REJECTED** with high confidence. The generated census, manifest/index mapping, master assignment, focused strict validation, and five-path hygiene all pass. One unfenced current navigation source still contradicts the corrected totals, so the prior coverage reconciliation contract is not yet complete. Required next action: make the four-line `backlog/00-INDEX.md` reconciliation above, update evidence/scope and the CTRL-07 fence, then submit a new immutable candidate for review.
