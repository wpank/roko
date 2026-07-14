# CTRL-14 independent evidence review — not ready for terminal status

## Review target

- Candidate: `ed5ab0fed4a820b814d59b398c5c989b5003cfdf`
- Candidate parent/base: `5719b51c03dfde9e2709233d51e422c826ee97a2`
- Candidate change: only `tmp/status-quo/execution-evidence/CTRL-14.md`
- Review scope: factual accuracy of the nonterminal supersession evidence. This is
  not acceptance of CTRL-14 as `DONE`.

I read the complete master checklist and parsed every record in the complete
6,349-line retired manifest. I then traced every declared replacement to its exact
canonical task record and source epic rather than relying on the worker's aggregate
table.

## Independent reproduction

### Retired plan and replacement corpus

An independent `tomllib` traversal of
`tmp/status-quo/backlog/plans/status-quo-authoring-gaps/tasks.toml` and all
`tmp/status-quo/backlog/plans/E*/tasks.toml` files reproduced:

```text
retired_tasks=96 unique_retired_ids=96 skipped=96 executable=0
dependency_edges=79
epic_manifests=48 canonical_tasks=447 statuses=done:6,ready:441
mapped_targets=96 orphan_targets=0 unique_epic_groups=17
titles=exact:4,refined:92
verify=retired:480,canonical_targets:290
acceptance=retired:384,canonical_targets:227
```

The four literal title matches are exactly `E01-T07`, `E06-T05`, `E10-T04`, and
`E12-T05`. Every other mapping is still deterministic: removing only the `GAP-`
prefix produces the unique canonical stable ID, and the retired record itself names
the exact canonical plan path. No title similarity or fuzzy matching is involved.

For all 96 records I independently checked that:

- the declared plan path exists and contains the stable ID exactly once across the
  48-manifest canonical corpus;
- the target's epic prefix matches both its plan directory and declared epic file;
- the declared epic exists and contains that exact stable ID;
- the canonical task has nonempty role/files, read-files/symbol/anti-pattern context,
  non-placeholder verify commands with failure messages, and acceptance coverage;
- every retired dependency resolves within the 96-record retired graph.

The group counts are independently reproduced as
`E01:7,E02:8,E03:4,E04:16,E05:5,E06:6,E07:7,E08:4,E09:6,E10:4,E11:2,E12:6,E13:1,E14:4,E15:3,E17:3,E18:10`.
The absence of E16 is explained by the absence of any `GAP-E16-*` record, not by an
orphan or dropped mapping.

The authoring notes explicitly say the old plan was consumed and the 96 records are
skipped provenance. Their per-epic corrections explain why exact stable identity,
not literal title preservation, is the authoritative mapping for the 92 refined
titles. The coverage ledger, backlog indexes, and July 14 audit all agree that this
plan must remain superseded and must not execute.

Hashes and byte counts also match the candidate:

```text
retired_sha256=4705f6f7d00403f8aa33fb14db89bb5a8353a67468fe3a3d3af613245f7a9d03
epic_manifest_set_sha256=e3cb1eb1a3ec0300aaa07efc652090e1bc492139d90e5cfe892659ba1a1a8607
epic_manifest_files=48 epic_manifest_bytes=1501633
```

### Strict isolation and nonexecution

Using the integrated `target/debug/roko`, I copied only the retired manifest into an
isolated temporary plan directory, linked its referenced `tmp` tree, and ran:

```sh
roko plan validate --strict retired --color never
roko plan run retired --workdir "$fixture" --dry-run --json --no-serve --color never
```

Both exited 0 and reproduced:

```text
0 diagnostics in 1 plan
"plans": []
"total_plans": 0
"total_tasks": 0
```

The production path agrees with that behavior: `TaskDef::is_ready` requires the
literal `ready` status, `task_status_is_terminal` classifies `skipped` as terminal,
and `TaskTracker::new` pre-collects manifest-skipped IDs.

### Independently reproduced nonterminal contradictions

I ran the exact source-coverage program published in
`tmp/status-quo/backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md`. Its current result is:

```text
sources=745
missing_from_ledgers=1
missing_from_doc_tasks=1
tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
```

Direct fixed-string search across every source ledger and every `DOC-*` manifest
found no alternate assignment. The direct `tmp/status-quo/*.md` corpus is now 109,
while the ledger and DOC inventory still claim 108; the published aggregate remains
744 instead of 745.

I also parsed every `E*/tasks.toml`: current reality is 48 manifests and 447 tasks,
including 169 in E01-E18. `tmp/status-quo/backlog/plans/00-INDEX.md` still lists only
E01-E18 as 149 tasks and still claims 744 sources. The canonical audit independently
labels that index materially stale.

Finally, current whole-root validation independently reproduced the candidate's
inventory mismatch:

```text
non-strict: exit 0, 13 diagnostics in 55 plans
strict:     exit 1, 13 diagnostics in 55 plans
```

That contradicts the coverage ledger's current-sounding statement of six expected
`PLAN_031` warnings, while leaving the isolated retired plan's zero-diagnostic result
unchanged. Whole-root validation regenerates top-level `plans/INDEX.md` as a known
working-tree side effect; that unstaged side effect is not part of the reviewed
candidate or this review commit.

## Changed-line and evidence-quality review

The candidate changes no manifest, product code, index, or canonical status. Its
reproduction program, hashes, aggregate counts, mapping table, runtime trace, and two
required reconciliation findings are factually accurate. `git show --check` reports
one blank line at the end of the new Markdown file; this is non-semantic and does not
change the evidence verdict.

No evidence claim weakens, hides, or converts unfinished programme work into a
terminal result. In particular, the candidate correctly refuses to mark CTRL-14
done while the source coverage and plan-index/validation inventories contradict the
integrated tree.

## Verdict

**ACCEPTED — evidence accuracy only. Programme status remains `REVIEW_NOT_READY`.**

Confidence: high. No correction to the factual content of candidate
`ed5ab0fed4a820b814d59b398c5c989b5003cfdf` is required. This review must not be used
as terminal acceptance of CTRL-14. The coordinator must first integrate independently
reviewed repairs for both reproduced control-plane defects, rerun the candidate's
acceptance checks against that integrated commit, and obtain renewed terminal review
before changing CTRL-14 to `DONE`.
