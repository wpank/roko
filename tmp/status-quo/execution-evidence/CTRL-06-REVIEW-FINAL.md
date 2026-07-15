# CTRL-06 final independent review

Verdict: **ACCEPTED**

## Candidate identity and scope

- Exact candidate: `7f62204e50bf0845f383ca18c97f25cf7170d391`.
- Candidate parent: `e9df2b2b064c5967354157306dbfa33c6282d60e`.
- Implementation commit: `ea018feedcbccca3a3d922d293721134e6c7e829`.
- F1 correction commit: `25da49fc9b9b33aac003649b0d17b9c791727fc5`.
- F2 correction commit: `e9df2b2b064c5967354157306dbfa33c6282d60e`.
- Base: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`.
- Review branch/worktree: `review/CTRL-06-7f62204e50bf` in the designated
  final-review worktree. The worktree was clean at identity verification.
- Reviewed the full master again, candidate history and complete task diff,
  worker evidence, prior rejection commit
  `d5671d1e9994bb002563879e4f049004f470b31e`, validator production path,
  focused tests, and canonical schema.

## Requirement reconstructed independently

Task `files` and `write_files` are creation or mutation outputs and need not
exist before execution. `context.read_files[*].path` entries are prerequisites
that must exist unless the exact lexical path is produced by a declared
transitive same-plan task dependency or by a loaded plan named in
`depends_on_plan`. Unrelated producers must not suppress diagnostics. Missing
prerequisites remain `PLAN_030`/`PLAN_031` warnings and fail strict validation.
Implementation, public rustdoc, tests, canonical schema, and evidence must state
one contract without path, plan, or expected-count special cases.

## Rejection disposition

Both rejection cycles are fully dispositioned.

### F1

- `validate_plans_dir_with_workdir` public rustdoc describes outputs,
  prerequisites, and the declared-dependency exception.
- Canonical schema rows for `files`, `read_files`, and `PLAN_030/031` state the
  same contract.
- The three originally cited obsolete test setups/rationales are removed or
  rewritten.
- Worker evidence records the rejected candidate and F1 correction.

### F2

- `crates/roko-cli/tests/plan_validate.rs` no longer creates `stub/` or writes
  `stub/lib.rs` in the complete architecture-deferral fixture.
- The supporting plan remains useful recursive multi-plan coverage, and both
  comments now explicitly say its declared output is intentionally absent.
- Worker evidence records rejected candidate `5014afa2cd2c`, rejection review
  `d5671d1e9994`, correction commit `e9df2b2b064c`, and the focused rerun.
- A search of both focused test files, worker evidence, and canonical schema
  found no remaining old `PLAN_030/031` output-stub rationale.

The F2 range changes only `crates/roko-cli/tests/plan_validate.rs` and worker
evidence. `crates/roko-cli/src/plan_validate.rs`,
`crates/roko-cli/tests/plan_validation.rs`, and the canonical schema are
byte-identical to candidate `5014afa2cd2c`, whose production behavior was
already independently inspected and verified. No assertion was weakened.

## Production-path assessment

The implementation classifies outputs and prerequisites separately, uses
ordered sets for stable per-task paths, follows the transitive same-plan
dependency closure, and imports cross-plan outputs only for explicitly named
loaded plan dependencies. Existing paths pass; undeclared missing prerequisites
remain deterministic strict warnings. CLI validation, plan pre-run validation,
and PRD artifact validation reach this implementation. No warning suppression,
task/plan identity special case, backlog path special case, fake success, hidden
fallback, or new serialized/public interface was found.

## Independent verification

- `git rev-parse HEAD` — exact candidate
  `7f62204e50bf0845f383ca18c97f25cf7170d391`.
- `git rev-parse HEAD^` —
  `e9df2b2b064c5967354157306dbfa33c6282d60e`.
- `git diff --check e736f324..7f62204e` — pass.
- `cargo fmt --all -- --check` — pass.
- With dedicated serialized `CARGO_TARGET_DIR=/private/tmp/roko-ctrl06-review-target`
  and `CARGO_INCREMENTAL=0`, `cargo test -p roko-cli --test plan_validation` —
  pass, 24/24. The pre-existing test-crate missing-docs warning remains.
- Same target, `cargo test -p roko-cli --test plan_validate` — pass, 18/18,
  including the corrected absent-output multi-plan fixture.
- The target binary reports Git `7f62204e5`.
- Rebuilt strict self-heal validation — exit 0,
  `0 diagnostics in 6 plans`.
- Two consecutive strict backlog validations — both exit 1 with
  `16 diagnostics in 55 plans`; complete outputs are byte-identical.
- Inspection of all 16 backlog warning lines found 16 prerequisite-only
  `PLAN_031` diagnostics and zero other warnings. No known creation output is
  diagnosed.
- Final candidate worktree remained clean before this review record.

The focused suite covers missing outputs, missing and existing prerequisites,
same-plan dependency outputs, transitive closure, explicitly loaded cross-plan
outputs, unrelated producer rejection, path classification/deduplication, and
strict CLI behavior.

## Verdict and confidence

**ACCEPTED** with high confidence. The complete CTRL-06 contract is satisfied,
F1 and F2 are fully dispositioned, and no required next action remains for the
candidate. The integration owner should merge this exact candidate with this
review record, rerun the focused tests and strict validators on the integration
commit, and only then reconcile canonical CTRL-06 status.
