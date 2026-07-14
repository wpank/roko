# CTRL-07 final independent review

## Verdict

`ACCEPTED`

Corrected candidate `9458a6920d72e457553e31cd51b9ac89d70d2483`
fully resolves finding R1 from rejected review
`18d16f2250bf1de3a09422c025019454b72511d6`. No required candidate correction
remains.

## Review identity

- Candidate: `9458a6920d72e457553e31cd51b9ac89d70d2483`
- Original worker base: `a4278ced081c9f42ef186b8c4a93528ef78c05c3`
- Rejected predecessor: `18973e221a5ce6f8f72366ca2d8815db21f85b7c`
- Prior rejection evidence:
  `18d16f2250bf1de3a09422c025019454b72511d6`
- Review branch: `review/CTRL-07-9458a6920d72`
- Validator: integrated `roko 0.1.0`, Git build identifier `d4749f9c7`
- Sealed source `plans/INDEX.md` SHA-256:
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`

## Rejection disposition

R1 said E17-T02's single `prompt_experiment.rs` range ended at line 455 while its
rationale also claimed `record_metric`, which is at lines 599–604.

The corrected task now has two exact surgical context entries:

```toml
{ path = "crates/roko-learn/src/prompt_experiment.rs", lines = "395-455", why = "Current ExperimentStore load, active lookup, and assign_variant APIs to mirror for ACP variant selection." }
{ path = "crates/roko-learn/src/prompt_experiment.rs", lines = "590-605", why = "ExperimentStore replay boundary and record_metric implementation for the outcome-recording half of the ACP selection/outcome pair." }
```

Independent source inspection confirms:

- lines 395–441 contain `ExperimentStore`, `load_or_new`, `get`, and
  `active_for_section`;
- lines 447–454 contain
  `assign_variant(experiment_name) -> Option<(String, String)>`;
- lines 591–597 contain the replay boundary;
- lines 599–604 contain
  `record_metric(experiment_id, variant_id, metric)` and its delegation.

The worker evidence now names both ranges, records the prior rejection, explains the
split selection/outcome contract, and compares the manifests against the explicit
base SHA rather than ambiguous `HEAD`. R1 is completely resolved rather than merely
waived.

## Corrected-diff review

The correction commit changes exactly two paths relative to the rejected candidate:

1. `tmp/status-quo/backlog/plans/E17-acp-completion/tasks.toml`
2. `tmp/status-quo/execution-evidence/CTRL-07.md`

It only splits the E17 source context and reconciles its evidence. It does not alter
a task ID, status, dependency, files/write scope, verify command, acceptance outcome,
manifest metadata, production source, validator source, index, or lockfile.

The complete candidate relative to base remains exactly the previously reviewed 13
paths: 12 backlog manifests plus `CTRL-07.md`. The 11 non-E17 manifests and both
producer-edge changes are byte-identical to the predecessor accepted on every point
other than R1. The immutable prior review established that:

- all ten removed stale paths have semantically correct live module/symbol owners;
- E06-T07 retains E06-T06 and adds the real E06-T01 producer of
  `E06-canonical-surface.md` without a cycle;
- E18-T14 retains E18-T02 and adds the real E18-T03 producer of `deny.yml` without
  a cycle;
- no prerequisite was deleted, hidden, weakened, or replaced with a placeholder.

The current correction does not touch those conclusions.

## Independent structural verification

Fresh Python `tomllib` comparison of every touched manifest against
`git show a4278ced081c9f42ef186b8c4a93528ef78c05c3:<path>` confirmed:

- all 12 `[meta]` tables are unchanged;
- all task counts still equal `meta.total`;
- every ordered task ID is unchanged;
- every task status is unchanged;
- exactly the intended task changes in each manifest;
- E17-T02 has exactly the corrected 395–455 and 590–605 context records.

Observed manifest comparison:

```text
E01-execution-engine E01-T11 16 meta_ids_status=unchanged
E02-STORAGE-CONVERGENCE E02-T03 12 meta_ids_status=unchanged
E06-COMPOSE-UNIFY E06-T07 9 meta_ids_status=unchanged
E09-OBSERVABILITY E09-T10 11 meta_ids_status=unchanged
E14-providers-tools E14-T08 12 meta_ids_status=unchanged
E15-mcp-config E15-T1 7 meta_ids_status=unchanged
E17-acp-completion E17-T02 8 meta_ids_status=unchanged
E18-DOCS-CONFIG-OPS E18-T14 15 meta_ids_status=unchanged
E20-cell-unification E20-T08 10 meta_ids_status=unchanged
E31-trigger-system E31-T03 8 meta_ids_status=unchanged
E33-telemetry-lens E33-T03 9 meta_ids_status=unchanged
E35-auth-protocol E35-T01 8 meta_ids_status=unchanged
E17_F1=resolved exact_ranges=395-455,590-605
```

All 193 tracked TOML files in a disposable full archive of the corrected candidate
parse successfully.

## Independent validator rerun

The corrected commit was exported to a disposable repository-shaped directory so
the validator's generated-index behavior could not modify the review worktree:

```sh
git archive 9458a6920d72e457553e31cd51b9ac89d70d2483 | tar -x -C "$fixture"
(cd "$fixture" && "$roko" plan validate --strict tmp/status-quo/backlog/plans --color never)
(cd "$fixture" && "$roko" plan validate --strict tmp/status-quo/self-heal/plans --color never)
```

Observed results:

```text
corrected backlog:  exit 0; 0 diagnostics in 55 plans
corrected self-heal: exit 0; 0 diagnostics in 6 plans
all TOML:            193 files; 0 parse errors
```

The prior independent review separately reproduced the uncorrected base at exit 1
with the exact 12 `PLAN_031` diagnostics. Because the correction only adds a second
read range within the already-correct E17 replacement path, it neither hides a base
diagnostic nor changes validator behavior.

## Hygiene and acceptance

- `git diff a4278ced..9458a692 --check`: exit 0.
- `git diff 18973e221..9458a692 --check`: exit 0.
- Source `plans/INDEX.md` SHA-256 before and after isolated validation:
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
- Review worktree was clean before this final review record.
- No `.roko` state, generated index delta, temporary artifact, lockfile, or other
  test output remains in the worktree.

The coordinator may integrate the exact accepted candidate and this review record,
then perform the master's required post-merge verification. No renewed worker action
is required.

