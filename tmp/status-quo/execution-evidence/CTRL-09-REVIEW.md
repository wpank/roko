# CTRL-09 r2 independent review

- **Verdict:** `REJECTED`
- **Candidate:** `abfa50fb8ff50226e6ade3e00e1e13aa3de9c338`
- **Base:** `bb5048f4c1d0e3f34155e89d39ccef46109c3b59`
- **Review branch:** `review/CTRL-09-r2-abfa50fb8`
- **Review date:** 2026-07-14

## Independent scope and method

I read the complete master checklist, the candidate's three-path diff and worker
evidence, the complete DOC-v2-core manifest and coverage ledger, all 34 `docs/v2`
sources, all E19-E45 manifests and their task contracts, the canonical CTRL-08
ownership record/evidence/final review, the live `TasksFile` parser, and relevant Git
history. I recreated the preservation, parser-semantic, source/writer, product-owner,
combined-graph, TOML, strict-validation, index, scope, and merge checks without using
worker scripts or archives.

The candidate is the direct child of the stated base and changes exactly:

1. `tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml`
2. `tmp/status-quo/backlog/source-coverage/docs-v2-core.md`
3. `tmp/status-quo/execution-evidence/CTRL-09.md`

It changes no product source, tests, master, shared index, lockfile, or top-level plan
index. `git diff --check` passes. `git merge-tree --write-tree` from the stated base
produces tree `5a28eabe0f1065791876844348fad390d0551a0d`, exactly the candidate tree.

## Independently reproduced passing controls

- The ten ordered IDs, ten `ready` statuses, and meta
  `plan/total/done/status/max_parallel/skip_enrichment` values are unchanged from the
  base. The estimate alone changes from 4,200 to 1,800 minutes.
- All ten records are `scribe`/`docs` documentation acceptance roll-ups, have
  nonempty task-level acceptance before their context/verify tables, have at least
  one scheduler-visible product-plan dependency, and write only `docs/v2/*.md`.
- The 34 source files are exactly the 34 write paths, each occurs once, and the
  coverage-ledger table contains exactly the same 34 `(source, writer)` pairs with no
  duplicate writer or omitted source.
- Every canonical E19-E45 plan occurs in at least one `depends_on_plan` list. Local
  ordering makes T10 the final roll-up. No task is done or prematurely executable,
  and no DOC task retains product implementation scope.
- A temporary independent Rust test invoked the public
  `roko_cli::task_parser::TasksFile::parse` API. It proved the parser sees the exact
  IDs/meta/statuses, nonempty task-level acceptance and product dependencies, all 27
  E19-E45 owners, and 34 unique docs-only writers:

  ```text
  running 1 test
  test doc_v2_rollups_have_parser_visible_acceptance_and_dependencies ... ok
  test result: ok. 1 passed; 0 failed
  ```

  The temporary test and its executable were removed; neither is review scope.
- All 193 tracked TOMLs parse. Combined graph truth is 93 plans
  (32 top-level + 55 backlog + 6 self-heal), 881 tasks, statuses 33 done / 752 ready /
  96 skipped, zero unresolved local references, zero unresolved plan references, and
  zero cyclic strongly connected components.
- Disposable candidate-archive validation with the integrated repository binary
  reports `0 diagnostics in 55 plans`, `0 diagnostics in 6 plans`, and
  `0 diagnostics in 1 plan` for backlog, self-heal, and DOC-v2-core respectively.
- `plans/INDEX.md` remains sealed at SHA-256
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.

## Rejecting finding

### F1 — committed combined-graph proof is factually wrong

`tmp/status-quo/execution-evidence/CTRL-09.md` claims:

```text
unique plan edges=162
```

That value is false for the immutable candidate. Counting the stated combined
universe by unique `(source meta.plan, referenced plan)` pairs across both
`meta.depends_on` and every task's `depends_on_plan` yields **160**, not 162. Counting
raw occurrences yields 320, so 162 is not an alternate occurrence-based definition.

The difference is fully localized and reproducible:

```text
base bb5048f4c:      133 unique plan-to-plan pairs
candidate additions: 27 (exactly E19 through E45 from DOC-v2-core)
candidate removals:   0
candidate total:    160
```

The reported 162 is consistent with carrying forward the earlier CTRL-08 evidence's
stale `135` count and adding 27, rather than recomputing against this candidate. The
actual unique-pair count is 133 at both CTRL-08 candidate `1e07967a3` and this r2
base, and 160 at `abfa50fb8`.

This does not invalidate the graph itself: references and SCC checks are green. It
does invalidate a committed proof claim, and the master evidence contract does not
permit acceptance with a known factual correction outstanding.

## Required next action

Do not merge `abfa50fb8ff50226e6ade3e00e1e13aa3de9c338` as accepted CTRL-09 work.
Create a new immutable candidate that changes the CTRL-09 evidence count from 162 to
160 and records the exact `133 + 27 - 0` derivation (or names and proves a different
counting definition). Preserve the manifest and ledger unchanged unless another
reviewed requirement arises, then obtain fresh independent review of that candidate.
