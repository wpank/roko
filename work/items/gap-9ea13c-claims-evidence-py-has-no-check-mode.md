+++
id = "gap-9ea13c"
kind = "gap"
title = "claims_evidence.py has no --check mode, so the design paper's claims aggregate has no automated staleness check"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "reg-3621b1 investigation (worker, 2026-10-16)"
discovered_from = "reg-3621b1"
anchors = ["tmp/cybernetic-harness/paper/tools/claims_evidence.py", "tmp/cybernetic-harness/paper/tools/claims.py"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["reg-3621b1", "gap-aed6f1", "gap-c4d630"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tmp/cybernetic-harness/paper/tools/claims_evidence.py --check"
+++

## Problem

`tmp/cybernetic-harness/paper/tools/claims_evidence.py` — the research paper's real claims aggregator since the
2026-10-02 design-paper rewrite — can only write `CLAIMS-EVIDENCE.md`; it has no `--check` mode. `python3
tmp/cybernetic-harness/paper/tools/claims_evidence.py --check` exits 2 (`unrecognized arguments: --check`). So
nothing can mechanically tell whether the live `CLAIMS-EVIDENCE.md` still matches the section ledgers; the only way
to know is to regenerate and diff by hand.

The paper also still carries `tools/claims.py` (and `test_claims.py`), a pre-rewrite tool with a working `--check`
that reg-3621b1 asked to make pass. It parses the current ledgers fine (608 claims, matching claims_evidence.py's
count) but renders a different, heavier format — `maturity@commit`, `blocks_on`, hypothesis and spec maps — built
for the archived empirical draft. `claims_evidence.py`'s own header says so explicitly: "`tools/claims.py` was not
used. It emits an older format ... that this design paper doesn't use." That line has been in the file since
2026-10-02 (`claims_evidence.py`'s mtime), and the 2026-10-16 independent review (gap-aed6f1, done) used
`claims_evidence.py`, not `claims.py`, to regenerate the ledger after rewriting 198 rows and adding/deleting 14/16
(`tmp/cybernetic-harness/paper/REVIEW.md`, "How the review was done").

reg-3621b1 (regression, filed from status-quo-2026-10-15#finding-35) asked to make `claims.py --check` pass. Doing
that would mean regenerating `CLAIMS-EVIDENCE.md` in the old format, undoing gap-aed6f1's just-merged, reviewed
output and contradicting what `REVIEW.md` says was checked. reg-3621b1 was closed `superseded` instead (evidence
there): verified that `claims_evidence.py`'s output is already byte-identical to the live `CLAIMS-EVIDENCE.md`
except the non-deterministic "Generated <time>" line, so the aggregate is not actually stale — only `claims.py`
(the wrong tool) says it is.

## Why it matters

Goal `whitepaper`. gap-c4d630 built `claims.py` specifically so a reader (and a check) could tell when the claims
aggregate drifted from the section ledgers. Since the 2026-10-02 rewrite moved the paper to `claims_evidence.py`'s
leaner format, that safety net has been silently missing: the paper has no command that fails when
`CLAIMS-EVIDENCE.md` goes stale.

## Where

- `tmp/cybernetic-harness/paper/tools/claims_evidence.py` (untracked; add `--check` here).
- `tmp/cybernetic-harness/paper/tools/claims.py`, `test_claims.py` (untracked; superseded — candidates for deletion
  once nothing points at them; `tools/paperlint.py:41`, tracked, has a comment calling `claims.py` the consumer of
  the ledger tables and would need its wording updated too, as a separate small edit since it is tracked code).
- `tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md` (untracked; the `--out` default for both tools).

## Current state

`claims_evidence.py main()` always does `out.write_text(text, ...)` — no comparison, no exit code signalling
staleness. `claims.py` already has a `--check` implementation (temp file, `filecmp.cmp`, unified diff on stderr,
`sys.exit(1)`) that is a reasonable model to port, minus the derived-column logic that doesn't apply here.

## Plan

1. Add `--check` to `claims_evidence.py`: write the regenerated text to a temp file, compare byte for byte against
   `--out`'s target (default `CLAIMS-EVIDENCE.md`), print a short diff and exit 1 if different, print "up to date"
   and exit 0 otherwise (mirror `claims.py --check`'s shape, not its derived-column format).
2. Decide what to do with `claims.py` / `test_claims.py`: delete them (nothing else depends on them per a repo-wide
   grep for `claims.py` outside `work/` and the paper itself, which found only the `tools/paperlint.py:41` comment),
   or keep them parked with a note that they are for the archived empirical-draft format only. Either way, update
   that `paperlint.py` comment.
3. Re-run the review's checks list (`REVIEW.md`, "How the review was done") with the new `--check` to confirm it
   passes on the current ledger.

## Done when

The `[[verify]]` command passes: `claims_evidence.py --check` exists and reports the aggregate current.

## Notes

- Both files are untracked (`tmp/` is gitignored): work in the main checkout, or in a worktree with a copy of
  `tmp/cybernetic-harness/`.
- `tools/paperlint.py` is tracked code outside the paper; touch it only as its own small, separate edit.
