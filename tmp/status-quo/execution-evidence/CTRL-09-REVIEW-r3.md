# CTRL-09 r3 independent review

> [!CAUTION]
> **SUPERSEDED FULL-GRAPH CLAIM.** This historical acceptance correctly proved
> 160 unique task-level runtime edges, but repeated the r2 parser omission of
> the two `meta.depends_on_plan` edges and mislabeled 160 as the complete graph.
> The complete declared graph has 162 unique edges. Preserve the review for its
> manifest and source findings, but use `CTRL-09-POSTMERGE-CORRECTION.md` and its
> fresh review as the corrected graph authority.

- **Verdict:** `ACCEPTED`
- **Candidate:** `3ac488dde191ae9f0dbf32f861cd160153bf263a`
- **Rejected parent:** `abfa50fb8ff50226e6ade3e00e1e13aa3de9c338`
- **Integration base for the cumulative candidate:**
  `bb5048f4c1d0e3f34155e89d39ccef46109c3b59`
- **Review branch:** `review/CTRL-09-r3-3ac488dde`
- **Review date:** 2026-07-14

## Independent reconstruction and scope

I reviewed the complete cumulative candidate from the stated base, not only the r3
evidence correction. I read the master acceptance/review contract, all 581 lines of
the candidate DOC-v2-core manifest, the complete coverage ledger and worker evidence,
the prior r2 rejection, the live `TasksFile` parser, the combined plan roots, and the
three-path cumulative diff.

The cumulative candidate changes exactly:

1. `tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml`
2. `tmp/status-quo/backlog/source-coverage/docs-v2-core.md`
3. `tmp/status-quo/execution-evidence/CTRL-09.md`

It changes no product source, test, master, shared index, lockfile, or top-level plan.
The manifest and coverage ledger at r3 are byte-for-byte unchanged from rejected r2:

```text
DOC-v2-core/tasks.toml SHA-256:
35c2b939da233fc79c83ee79bc3c7875dbc509c10f25416c69c8ab7aa0b4976a
docs-v2-core.md SHA-256:
7722cd5384c164dc57005f8102ae54c39b2c145d078af461d83616e56cc20625
r2..r3 diff: CTRL-09.md only, 2 insertions / 1 deletion
```

`git diff --check` passes. A merge-tree preflight against current integration HEAD
`5ca5a09ae73c9d7b2838668d1c8ffaced9d73f43` succeeds with result tree
`009ead1a2322b613ff9ac88c44349ce5a36b0116`.

## Parser, ownership, and source proof

I added a temporary integration test which invoked the public
`roko_cli::task_parser::TasksFile::parse` API on the candidate manifest. It asserted
the exact ordered IDs, preserved meta and statuses, `scribe` role, `docs` domain,
task-level acceptance and executable verify steps, scheduler-visible product-plan
prerequisites, parser definition order, all 34 unique docs-only write paths, complete
context coverage, and E19-E45 coverage. A raw TOML assertion separately checked the
control-plane-only `ownership` marker, which `TasksFile` intentionally ignores as an
unknown field.

```text
CARGO_TARGET_DIR=.../integration/target \
  cargo test -p roko-cli --test ctrl09_review_semantics -- --nocapture

running 1 test
test candidate_is_ten_docs_only_acceptance_rollups_with_complete_ownership ... ok
test result: ok. 1 passed; 0 failed
```

Independent Python comparisons additionally proved:

```text
preserved meta except estimate=yes; estimate=4200->1800
IDs/order/status=unchanged; task-level acceptance=10/10
sources=34; unique writers=34; context sources=34; exact ledger pairs=34
all writers are docs/v2 Markdown; roles/domains/ownership=10/10
E19-E45 prerequisite coverage=27/27
```

The temporary test source and graph script were removed after the checks. The
disposable archive was removed by its exit trap; no isolated Cargo target was
created.

## Combined graph and TOML proof

I independently parsed the selected execution universe: 32 top-level, 55 backlog,
and 6 self-heal manifests. I resolved each local task dependency within its owning
plan, each plan dependency against the 93-plan ID set, and ran Tarjan SCC detection
over the unique plan dependency pairs. I computed the base and candidate edge sets
separately from Git objects:

```text
plans=93; tasks=881
statuses={'done': 33, 'ready': 752, 'skipped': 96}
base unique plan edges=133
candidate additions=27 (DOC-v2-core -> each E19 through E45)
candidate removals=0
candidate unique plan edges=160
unresolved local references=0
unresolved plan references=0
cyclic strongly connected components=0
```

This reproduces the corrected r3 claim exactly as `133 + 27 - 0 = 160` and fully
disposes the sole r2 rejection. All 193 tracked TOML files parse with `tomllib` and
zero errors.

## Disposable strict validation and sealing

I validated a fresh `git archive` of the immutable candidate with the integrated
repository binary, so validator-generated index output could not touch the review
worktree:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans

roko plan validate --strict tmp/status-quo/backlog/plans/DOC-v2-core
0 diagnostics in 1 plan
```

The source `plans/INDEX.md` remains sealed at SHA-256
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
The review worktree was clean before this review record was authored.

## Verdict

`ACCEPTED`

The cumulative candidate converts DOC-v2-core into ten dependency-gated,
file-disjoint documentation acceptance roll-ups without retaining product
implementation ownership, while preserving all ten task IDs and all 34 v2 sources.
Its semantic manifest, coverage mapping, combined dependency graph, strict roots,
scope, and evidence claims are internally consistent. The prior graph-count defect
is corrected and independently reproduced. No required next action remains before
integration and post-merge verification.
