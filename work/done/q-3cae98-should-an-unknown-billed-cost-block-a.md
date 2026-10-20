+++
id = "q-3cae98"
kind = "question"
title = "Should an unknown billed cost block a showcase bundle, or should cost_usd allow null with a marker?"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b883db4e6"
source = "wave-14 follow-up reports 2026-10-04 (gap-dbe8e7, gate 14b)"
discovered_from = "gap-dbe8e7 (in flight, work/gap-dbe8e7; added the hard refusal to satisfy the numeric contract)"
anchors = ["benchmarks/viabilitybench/showcase/build_bundle.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:30:32Z"
commit = "b883db4e6"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = true
evidence = "2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): Option A: keep the hard refusal. A bundle states what its runs cost, so a run with an unknown billed_usd is fixed upstream before the bundle is built. Re-ask if, after the CLI providers record billing = \"subscription\" (decision 3, dec-55532d), a pilot bundle is still blocked only by an unknown cost. Why: build_bundle.py refuses any run record whose costs.billed_usd is None (benchmarks/viabilitybench/showcase/build_bundle.py:576-578), and the contract declares cost_usd: number (demo/demo-app/src/showcase/contracts.ts:133,369). A public showcase that shows 'cost unknown' would weaken the claim it exists to make, and the refusal is the fail-closed choice. The main source of unknown costs is being removed: Will accepted billing = \"subscription\" for the Claude and Codex CLI providers so their attempts stop recording billed_usd as unknown (decision 3). The showcase itself waits on the pilots (9302/9303), so the refusal blocks nothing today. If vetoed: With option B, a new item changes the contract to cost_usd: number | null with a cost_usd_known flag, the builder allows null, the UI shows 'cost unknown', and a regression test covers the null path."
+++

## Problem

`build_bundle.py` now refuses to build a bundle at all if any run record has an unknown
`billed_usd`, because the showcase view contract declares `cost_usd: number` with no null
allowance. On `work/gap-dbe8e7` (not yet merged):
`benchmarks/viabilitybench/showcase/build_bundle.py:488-491` —
```
unbilled = [record["record_id"] for record in records if record["costs"]["billed_usd"] is None]
if unbilled:
    raise BuildError(f"{len(unbilled)} run record(s) have an unknown billed cost, and a bundle
    states what its runs cost (S10 §5.1): ...")
```
`demo/demo-app/src/showcase/contracts.ts:133,369` both declare `cost_usd: number;` (no `| null`).
Separately, `EfficiencySummaryRecord.cost_usd` (a different struct, `bug-9a6799`, done) already
established the pattern of keeping an unknown cost `Option<f64>`/null rather than forcing a
number — but that record has no reader that sums it, so nothing there had to decide what a
*consumer* does with a null cost. The showcase bundle is exactly that consumer.

## Why it matters

Goal: proof, release — the showcase pipeline's failure mode for incomplete billing data. An
experiment that otherwise ran cleanly but has one run with an unknown cost (e.g. a provider
that doesn't report billed_usd, per `gap-f548c1`'s note that it "stays null when the usage
source is unknown") currently can't be bundled or shown at all, for a reason unrelated to
whether the experiment's actual results are trustworthy. Whether that's the right trade-off
(force clean billing data before anything ships) or too strict (let the showcase degrade
gracefully) is a product/UX call, not a code question.

## Where

- `benchmarks/viabilitybench/showcase/build_bundle.py:488-491` (the refusal).
- `demo/demo-app/src/showcase/contracts.ts:133,369` (`cost_usd: number`, the schema the refusal
  exists to satisfy).

## Plan (decision needed)

- **Option A — keep the hard refusal.** A bundle's cost figure is a trust signal (S10 §5.1); an
  unknown-cost run is treated as a data-quality problem to fix upstream (provider billing,
  `gap-f548c1`-adjacent work) before anything ships, not something the showcase should paper
  over.
- **Option B — allow `cost_usd: number | null`** in the contract, with an explicit marker (e.g.
  a sibling `cost_usd_known: boolean`, or a `"redaction"`-style note) so a bundle can still be
  built and the UI can show "cost unknown" instead of blocking entirely.

## Done when

Will picks an option; if B, the contract and builder change together with a regression test for
the null/marker path.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-dbe8e7, gate 14b not yet merged): confirmed on
  `work/gap-dbe8e7`. Filed as `kind = "question"` per the instruction; no `[[verify]]` command
  since this is a decision, not yet a coded fix.
