# The plan-level slice (PL)

Whole features that need multi-task plans, each with a held-out whole-feature suite. The slice tests the golden path
that single tasks cannot: a frontier model plans a change, cheap models carry out its tasks in parallel, and the
result integrates. It is exploratory (RQ3): paper §5.3 and Appendix A.7, work items gap-89f393 (these fixtures) and
gap-1cd676 (the runs). It enters no hypothesis or Holm family.

Stdlib-only Python 3.11 or newer, like the rest of the benchmark. `slicekit.py` reuses `families/common/` for task
repos, canaries, surface renames and the gaming detectors, and `schema/validate.py` for its manifest schema.

## The features

| Id | Slug | What the arm builds | Tasks | Width | Requirements | Hidden checks |
|---|---|---|---|---|---|---|
| PL01 | `stockroom-reorder` | on-hand stock, catalog parsing, trailing demand, a reorder rule, a CLI command | 5 | 4 | 8 | 14 |
| PL02 | `docsearch-ranked-query` | tokenizing, a positional index, a query language, BM25, phrase search, a CLI command | 6 | 3 | 6 | 17 |
| PL03 | `logtally-access-report` | log parsing, longest-prefix geo lookup, UTC buckets, a report, a CLI command | 5 | 3 | 6 | 15 |
| PL04 | `pkgsolve-resolve-and-lock` | SemVer, constraints, a backtracking resolver, lock files, dependency chains, two CLI commands | 6 | 3 | 6 | 19 |
| PL05 | `rosterly-meeting-slots` | interval arithmetic, weekly hours, busy calendars, slot search, team files, a CLI command | 6 | 3 | 6 | 15 |
| PL06 | `csvclean-schema-clean` | typed coercion, a schema, a CSV reader, dedupe, the cleaning pipeline, a writer, a CLI command | 7 | 4 | 7 | 18 |

Width is the most skeleton tasks that can run at once (the largest set of which no task depends on another). Every
feature spans 5–7 modules, and each task owns its own files, so parallel tasks never edit the same file.
`slicekit.py list` prints this table's numbers, and `slicekit.py check` enforces the shape: 4–8 tasks, width at
least 3, two or more modules, and disjoint task files that together are exactly the reference's files.

## A feature's files

```
features/<dir>/
  feature.toml     vb.feature_source/1: id, slug, package, visible tests and verify command, suite provenance,
                   and the rename pools for the seed's surface
  description.md   the feature description: the only input either arm gets. Its requirements are "- **R1.** ..."
  plan.toml        vb.plan_skeleton/1: the reference solution's task decomposition, without code. Each task lists
                   its files, dependencies and the requirements it covers. No arm gets it.
  base/            the repo the agent starts from: an existing package, its passing tests, and a visible smoke
                   test for the feature, which fails until the feature exists. The untouched base is the stub.
  reference/       the reference solution: the files it adds or replaces
  hidden/          the whole-feature suite (unittest). Each test's docstring starts with the requirements it checks
                   ("R2, R3: ..."), so every requirement is stated in the description (recoverability) and checked.
```

Every file carries the release canary on a marker line (`slicekit.py mark` adds any that are missing). Rendering
strips the markers, so no task repo holds a canary.

## Running an instance

```bash
PY=benchmarks/viabilitybench/.venv/bin/python
PL=benchmarks/viabilitybench/families/plan_slice/slicekit.py
$PY $PL materialize --feature PL03 --seed 1 --workdir ~/vb-work/PL03-0001 --private ~/vb-private/PL03-0001
# the arm works in the workdir from ~/vb-private/PL03-0001/description.md alone
$PY $PL census --manifest ~/vb-private/PL03-0001/feature.json --workdir ~/vb-work/PL03-0001
```

- `materialize` renders `base/` with the seed's surface renames (the package name, drawn from the public surface
  stream), makes the workdir a task repo with a pristine bundle, and writes `description.md` and the manifest
  `feature.json` to the private directory. The manifest is `vb.feature/1` (`schema/feature.schema.json`), which a
  PL instance uses in place of `vb.task/1`. The private directory must lie outside the workdir, because the
  manifest records the canary.
- `census` exports the workdir as it is on disk. It records edited or deleted visible tests, skip markers added to
  them and canaries, restores the visible tests from the pristine base, and runs them. Then it adds the hidden suite
  to the export (never to the workdir) and runs it. Suites run in a child `python -I`, so a planted `unittest.py`
  or `sitecustomize.py` cannot take over the runner.
- The verdict: `passed` (the hidden suite), `checks` (one per hidden test, with its requirement ids), `visible`,
  `gaming`, `canary_hits`, `leak_suspected`, `vf` and `verified`.
  - `vf` is the census side of S09 §4.9's verified feature: the hidden suite passes and the visible tests were
    neither edited nor skipped. The driver adds the arm's own "done", and a canary hit makes the run
    `leak_suspected`, which is excluded and counted.
  - `verified` is stricter, and it is what verifier CI demands of a reference: the visible tests pass too, with no
    gaming flag and no canary. The visible tests include the base's own tests, so they catch regressions the hidden
    suite does not cover. S09 §4.9 has no visible condition, because it assumes the arms share no visible checks.
    They do share some here: both arms get the same base repo and its tests.

## Verifier CI

`slicekit.py selftest` checks each feature: the reference is verified, and the stub fails both suites. Beyond
that, the reference with any one skeleton task left undone fails the hidden suite, so every task is needed for the
feature to pass. The item's verify runs `tests/test_plan_slice.py`, the reference-and-stub check, in about 10 s.
`test_slice.py` runs the one-task-short check, a second seed, and the census's isolation, gaming and determinism
checks.

## Run records

Each `feature.toml` has a `[run_record]` table with the `task` values of the feature's `vb.run_record/1` rows.
`slicekit.run_record_task` adds `family = "PL"`, the instance id and `ladder = null`, and the manifest carries the
result as `run_record_task`. The schema tests (`schema/test_schemas.py`) build full rows for both of S09 §4.9's
arms, `roko_plan` and `fd_claude`, and validate them.

- `family = "PL"`. S08 §4.7 calls the task set `plan_slice`, and S09's lock entry is `exploratory.PL`.
- `spec_variant = "precise"`. The description is the precise spec: it states every requirement the hidden suite
  checks, and there are no vague or refined variants.
- `ladder = null`. A feature has no difficulty level. `vb.run_record/1` allows `null` for such rows, and
  `schema/feature.schema.json` requires it in the manifest, so no ℓ analysis can mistake a PL row for a level.
  `slicekit.py check` rejects a `[run_record]` that sets a ladder.

## Known gaps

- **Suite provenance.** The design asks for suites written by a different model family from the arms'. These were
  written by `claude-opus-5-5` (Anthropic), the model family of the `fd_claude` arm. Every `feature.toml` records
  `author` and `cross_family_review = "pending"` until another family reviews or rewrites the suites from the
  descriptions.
- **Static suites.** Unlike F1 and F4, whose hidden cases derive from HMAC(secret, instance), these suites are fixed
  files. Only isolation and canaries keep them from the agent, and a model that has read this tree could pass them.
- **No planted gaming solution** per feature. The generic detectors (test edits, skip markers, canaries) apply.
- **One surface knob.** A seed renames the package, and nothing else varies.
