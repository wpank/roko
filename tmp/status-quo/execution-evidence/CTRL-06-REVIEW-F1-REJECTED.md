# CTRL-06 independent review — historical F1 rejection

Verdict: **REJECTED**

## Candidate identity and scope

- Candidate reviewed: `5e837e25e78bfd702f432dffb2d3cd1c022db080`.
- Implementation commit: `ea018feedcbccca3a3d922d293721134e6c7e829`.
- Base: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`.
- The candidate adds only an evidence correction after the implementation commit;
  `git diff ea018feed..5e837e25` contains only
  `tmp/status-quo/execution-evidence/CTRL-06.md`.
- Production/test scope inspected line by line:
  `crates/roko-cli/src/plan_validate.rs`,
  `crates/roko-cli/tests/plan_validate.rs`, and
  `crates/roko-cli/tests/plan_validation.rs`.

## Requirement reconstructed independently

The canonical schema distinguishes mutation outputs (`files` and its
`write_files` alias) from dispatch prerequisites (`context.read_files[*].path`).
An output may be absent before its owning task runs. A prerequisite must exist on
disk unless the exact lexical path is produced by a declared same-plan task
dependency or a loaded plan named by `depends_on_plan`. An unrelated producer must
not suppress a warning. Strict mode must continue failing on these warnings, with
stable, deduplicated diagnostics and no plan/path special cases.

Primary context read in full: the master execution checklist, canonical task schema,
July 14 audit README, backlog/roadmap audit, worker evidence, current validator and
tests, CLI plan validation/pre-run call sites, PRD validation call site, and candidate
Git history.

## Changed-line and production-path assessment

The implementation behavior is coherent:

- `collect_task_path_references` classifies outputs and prerequisites separately.
- `BTreeSet` deduplicates and orders paths within a task.
- Same-plan output availability follows the transitive `depends_on` closure.
- Cross-plan output availability requires an exact loaded plan ID named in the
  consuming task's `depends_on_plan`.
- Missing existing-crate files remain `PLAN_031`; missing crate/package roots remain
  `PLAN_030`.
- `validate_plans_dir_impl` performs prerequisite checks only when a workdir is
  supplied, and final diagnostics are sorted deterministically.
- `cmd_plan_validate`, plan pre-run validation, and PRD artifact validation reach
  the updated path. No warning suppression, plan ID, status-quo path, or expected
  diagnostic-count special case exists in production code.

No behavioral defect was found in the reviewed implementation. The candidate is
nevertheless not acceptable because it leaves its public and canonical contracts
factually wrong.

## Blocking finding

### F1 — validator behavior contradicts its public rustdoc and canonical schema

Severity: medium; acceptance-blocking documentation/maintainability defect.

Evidence:

- `crates/roko-cli/src/plan_validate.rs:108-112` still says a supplied workdir
  causes declared `files` and `write_files` paths to be checked against the
  filesystem. The candidate now deliberately does the opposite: those paths are
  outputs and are never required to pre-exist.
- `tmp/status-quo/backlog/01-TASK-EXECUTION-SCHEMA.md:75` still says `files` is
  checked against the workspace by `validate_file_references`.
- The same canonical schema at line 292 still defines `PLAN_030/031` as missing
  declared `files` diagnostics. They are now missing `context.read_files`
  prerequisite diagnostics.
- Stale test rationales at `crates/roko-cli/tests/plan_validate.rs:70`, `:416`, and
  `:512` claim output stubs are created to satisfy `PLAN_030/031`; those stubs are no
  longer required for output validation.

Expected: implementation, public rustdoc, tests, and the canonical schema describe
one exact output/prerequisite contract.

Actual: users and future maintainers following the public function documentation or
canonical schema are told that intended creation outputs must already exist—the
precise false contract CTRL-06 is meant to remove.

Smallest required correction:

1. Update the rustdoc on `validate_plans_dir_with_workdir` to state that
   `files`/`write_files` are creation or mutation outputs and that workdir checks
   apply to `context.read_files` prerequisites, including the declared-dependency
   exception.
2. Update the canonical schema's `files`, `read_files`, and `PLAN_030/031`
   descriptions to the same exact contract.
3. Remove or rewrite the three stale test comments/setup rationales; retain any
   directories actually needed by unrelated greenfield checks and say why.
4. Update worker evidence scope/results, create a new immutable candidate, and rerun
   the same acceptance suite. No production algorithm change is requested by this
   finding.

## Independent verification

- `git diff --check e736f324..5e837e25` — pass.
- Search of the implementation diff for plan IDs, backlog/self-heal paths,
  `roko-gateway`, workflow/config output names, and expected counts — no production
  special cases found.
- Isolated rebuild:
  `CARGO_TARGET_DIR=/private/tmp/roko-ctrl06-review-target cargo build -p roko-cli --bin roko`
  — pass in 6m32s. Rebuilt binary reports Git `5e837e25e`.
- `cargo test -p roko-cli --test plan_validation` using the isolated target — pass,
  24/24. The pre-existing missing-crate-doc warning remains.
- `cargo test -p roko-cli --test plan_validate` using the isolated target — pass,
  18/18.
- `cargo check -p roko-cli` using the isolated target — pass.
- `cargo fmt --all -- --check` — pass.
- Rebuilt CLI, strict self-heal — exit 0, `0 diagnostics in 6 plans`.
- Rebuilt CLI, strict backlog — exit 1, exactly 16 `PLAN_031` prerequisite
  diagnostics in 55 plans. Every message names a missing prerequisite; no former
  missing task output is diagnosed.
- Two consecutive rebuilt-CLI strict backlog runs — both exit 1 and their complete
  outputs are byte-identical.
- Existing focused tests independently prove: missing outputs pass, missing
  prerequisites fail, existing prerequisites pass, a declared same-plan dependency
  output passes, an unrelated same-plan producer fails, a direct loaded plan output
  passes, and an unrelated loaded-plan output fails. Source inspection confirms
  transitive same-plan traversal and per-task path deduplication.

## Confidence and next action

Confidence: high. The implementation behavior passes all requested focused and
repository acceptance checks. F1 is directly reproducible from immutable source and
canonical documentation and has a narrow correction.

Required next action at the time of this verdict: worker issues a corrected immutable
candidate containing the contract text/test-rationale reconciliation above, then
this review is renewed.

## Historical disposition

This record remains the rejection of candidate
`5e837e25e78bfd702f432dffb2d3cd1c022db080`; its verdict is not rewritten by later
work. The correction sequence ending at
`5014afa2cd2c7f00f7dd40c5e17f08d8f8646acc` included commit
`25da49fc9b9b33aac003649b0d17b9c791727fc5`, which aligned the validator rustdoc,
canonical schema, and stale test rationales identified in F1. That corrected
candidate was separately rejected for an additional obsolete fixture, not for F1.
Commit `e9df2b2b064c5967354157306dbfa33c6282d60e` removed that fixture, and final
candidate `7f62204e50bf0845f383ca18c97f25cf7170d391` was accepted in review commit
`595eac759a2fea5b7dc22c4de182a94574971d6e`.

The original rejection record was left uncommitted under the then-current review
assignment. It is now committed under this explicit `F1-REJECTED` filename at the
coordinator's request so the historical correction chain remains auditable.
