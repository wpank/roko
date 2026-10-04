+++
id = "gap-eabd7f"
kind = "gap"
title = "m4-audits view is no longer written; needs a projection from S05's real audit records"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-14 follow-up reports 2026-10-04 (gap-dbe8e7, gate 14b)"
discovered_from = "gap-dbe8e7 (in flight, work/gap-dbe8e7; dropped the placeholder view rather than ship fake data)"
anchors = ["benchmarks/viabilitybench/showcase/build_bundle.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_build_writes_m4_audits_from_real_audit_records' benchmarks/viabilitybench/showcase/test_bundle.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/showcase/test_bundle.py -q -k test_build_writes_m4_audits_from_real_audit_records"
+++

## Problem

`m4-audits` is no longer written by `build_bundle.py`. On `main`,
`benchmarks/viabilitybench/showcase/build_bundle.py:57` has `VIEWS = ("overview",
"p1-head-to-head", "m4-audits")` and writes an `m4-audits` view (line 249:
`"m4-audits": {"schema": "showcase-view/m4-audits/1", "arms": audits}`). On `work/gap-dbe8e7`
(not yet merged), the `VIEWS` constant and the whole `m4-audits` projection are gone: the
builder no longer produces that view file at all, deliberately — its own comment says "`m4-
audits` waits for S05's audit records" (line 24) — rather than keep shipping one built from
fabricated/placeholder data. The Overview's catalogue still links a tile to `"view":
"m4-audits"` (line 85), so once a bundle is served, that link now points at a view that was
never written.

## Why it matters

Goal: proof, M4 deep audits / the showcase Audits page (S05, S10). The Audits page can't show
anything — not even an honest "not yet measured" placeholder view, since no view file is
written at all — until this projection exists. This item exists so dropping the fabricated view
(the right call, given `gap-2da8ec`'s finding that no real verdict exists yet either) doesn't
quietly become permanent: a real projection from S05's actual audit records is still owed.

## Where

- `benchmarks/viabilitybench/showcase/build_bundle.py` (the view to re-add, this time backed by
  real data; `project_views`, the dropped `VIEWS` constant).
- S05's audit records to project from: `crates/roko-gate/src/audit/policy.rs::Selection` (the
  draw/policy record, `audit.selection`), `crates/roko-cli/src/audit/worker.rs` (`AuditUnit`,
  `CheckOutcome` — the checks a unit ran and what they found), and the `audit.result`/labels
  records `crates/roko-cli/src/audit/labels.rs` writes.

## Current state

No `m4-audits` view is written on `work/gap-dbe8e7`. The source data to build one from (audit
selections, checks, results) exists on the Rust side, under the workspace's audit ledger/vault,
but nothing projects it into the showcase bundle's view format.

## Plan

1. Define what `m4-audits`'s view schema (`showcase-view/m4-audits/1`) needs from S05's real
   records: per-arm audit counts, false-green rate estimates, check pass/fail breakdowns —
   whatever `demo/demo-app`'s Audits page contract (`contracts.ts`) actually reads.
2. Read the relevant audit ledger/vault records for the experiment being bundled (policy/draws
   from `audit.selection`, checks from the audit worker's results) and project them into that
   shape.
3. Re-add `m4-audits` to the builder's written views, backed by this real projection.
4. Regression test: an experiment with recorded audit selections and results produces a bundle
   whose `m4-audits` view carries real, non-placeholder values.

## Done when

- A bundle built from an experiment with real S05 audit records includes a real `m4-audits`
  view, not an absent one.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-dbe8e7, gate 14b not yet merged): confirmed by diffing
  `build_bundle.py` between `main` (writes a placeholder `m4-audits` view) and
  `work/gap-dbe8e7` (writes none). Matches that branch's own Progress note: "m4-audits is left
  out until S05's audit records exist (the page shows 'not yet measured')." Related to
  `gap-2da8ec` (S09 verdict records/writer), filed the same day from the same branch — different
  data source (S05 audit records vs. S09 hypothesis verdicts), so kept separate.
