# CTRL-06 independent review

Verdict: **REJECTED**

## Candidate identity and review scope

- Exact candidate: `5014afa2cd2c7f00f7dd40c5e17f08d8f8646acc`.
- Candidate parent: `25da49fc9b9b33aac003649b0d17b9c791727fc5`.
- Implementation commit: `ea018feedcbccca3a3d922d293721134e6c7e829`.
- Prior rejected candidate: `5e837e25e78bfd702f432dffb2d3cd1c022db080`.
- Base: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`.
- Review branch/worktree: `review/CTRL-06-5014afa2cd2c` at the designated
  review worktree. The tree was clean at candidate identity verification.
- Reviewed the full master, canonical task schema, July 14 audit README and
  backlog audit, worker evidence, prior uncommitted rejection record, complete
  candidate diff, validator implementation, focused tests, and CLI/pre-run/PRD
  production call sites.

## Requirement reconstructed independently

Task `files` and `write_files` are creation or mutation outputs and need not
exist before execution. `context.read_files[*].path` entries are prerequisites:
they must exist on disk unless the exact lexical path is produced by a declared
transitive same-plan task dependency or by a loaded plan named in
`depends_on_plan`. Unrelated producers must not suppress diagnostics. Missing
prerequisites remain `PLAN_030`/`PLAN_031` warnings and therefore fail strict
validation. Production behavior, public rustdoc, canonical schema, tests, and
evidence must state the same contract without plan/path/count special cases.

## Prior rejection disposition

The corrected candidate dispositions the specifically cited F1 locations:

- `validate_plans_dir_with_workdir` rustdoc now describes outputs,
  prerequisites, and the declared-dependency exception.
- Canonical schema rows for `files`, `read_files`, and `PLAN_030/031` now match
  the implementation.
- The stale setup/rationales previously cited around the well-formed-plan,
  typed-contract, and architecture-packet tests were removed or rewritten.
- Worker evidence identifies the rejected candidate and correction commit and
  records a clean-target rerun.

The production algorithm remains coherent. It classifies paths with typed sets,
deduplicates deterministically, follows the transitive same-plan dependency
closure, imports outputs only from explicitly named loaded plan dependencies,
and reports undeclared missing prerequisites. CLI `plan validate`, plan pre-run
validation, and PRD artifact validation all reach this code. No suppression or
status-quo-specific special case was found.

## Acceptance-blocking finding

### F2 — one obsolete output-stub rationale and setup remains

Severity: medium; narrow documentation/test-maintainability correction.

Evidence:

- `crates/roko-cli/tests/plan_validate.rs:709-715` says a task reference to
  `plans/architecture-core-queue/tasks.toml` requires creating a valid plan and
  that the fixture's `files` entry points at a file "we also create". It then
  creates `stub/` and writes `stub/lib.rs` solely for that output entry.
- The same test's final comment describes the second plan as "the stub
  referenced by files".
- Under the corrected contract, `stub/lib.rs` is an output and deliberately
  need not exist. The focused `plan_validate_strict_allows_missing_task_outputs`
  test and the reviewed production path independently prove this.
- `tmp/status-quo/execution-evidence/CTRL-06.md` says obsolete output-stub setup
  was removed, so the retained setup also makes the candidate evidence
  factually incomplete.

Expected: every test rationale and fixture agrees that output paths may be
absent, especially after the prior rejection required contract reconciliation.

Actual: this remaining fixture still teaches and encodes the pre-fix rationale
that a `files` output should be pre-created. The production behavior is not
wrong, but accepting this candidate would leave the exact documentation/test
contradiction CTRL-06 is meant to eliminate.

Smallest correction:

1. Keep the second valid plan if multi-plan validation coverage is desired, but
   remove creation of `stub/` and `stub/lib.rs`.
2. Rewrite the comments to say the second plan exercises recursive multi-plan
   validation and that its declared output is intentionally absent. Rewrite the
   final two-plan assertion comment accordingly.
3. Update worker evidence to record this renewed rejection and correction,
   create a new immutable candidate, and rerun the focused contract.

No production algorithm or schema change is requested.

## Independent verification

- `git rev-parse HEAD` — exact candidate
  `5014afa2cd2c7f00f7dd40c5e17f08d8f8646acc`.
- `git rev-parse HEAD^` —
  `25da49fc9b9b33aac003649b0d17b9c791727fc5`.
- `git diff --check e736f324..5014afa2` — pass.
- `cargo fmt --all -- --check` — pass.
- Shared serialized target after a clean-target ENOSPC recovery:
  `cargo test -p roko-cli --test plan_validation` — pass, 24/24. The existing
  test-crate missing-docs warning remains.
- `cargo test -p roko-cli --test plan_validate` — pass, 18/18.
- `cargo check -p roko-cli` — pass.
- `cargo build -p roko-cli --bin roko` — pass; rebuilt binary reports Git
  `5014afa2c`.
- Rebuilt strict self-heal validation — exit 0,
  `0 diagnostics in 6 plans`.
- Rebuilt strict backlog validation — exit 1, exactly 16 `PLAN_031`
  prerequisite diagnostics in 55 plans; no task output is diagnosed.
- Two consecutive rebuilt strict backlog runs had exit 1 and byte-identical
  complete output.
- Initial independent cold target `/private/tmp/roko-ctrl06-rereview-5014`
  failed during dependency compilation with `No space left on device`. Only
  that review-created target was removed, recovering space; the complete
  assigned suite then passed using the coordinator-provided serialized shared
  target. No repository source was changed as a remedy.

Focused coverage and source inspection prove missing outputs pass, missing
prerequisites fail, existing prerequisites pass, declared same-plan and loaded
cross-plan dependency outputs pass, unrelated producers fail, transitive task
dependencies are followed, and path diagnostics are deterministic.

## Confidence and required next action

Confidence: high. No production behavioral defect was found and every assigned
command passed. F2 is directly visible in immutable candidate source and has a
four-line cleanup plus comment/evidence correction.

Required next action: issue a corrected immutable candidate that removes or
truthfully rewrites the remaining output-stub fixture, updates evidence, and
reruns the focused contract; then perform renewed independent review. This
review must not be merged as acceptance.
