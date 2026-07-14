# CTRL-06 implementation evidence

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0
- Base SHA: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`
- Branch/worktree: `agent/CTRL-06-output-prereq-validation` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-06`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-cli/src/plan_validate.rs`, `crates/roko-cli/tests/plan_validate.rs`, `crates/roko-cli/tests/plan_validation.rs`, and this evidence file

Requirement:
- Original defect or missing behavior: `validate_file_references` treats `task.files` and `write_files` mutation outputs as required pre-existing inputs, producing 25 misleading strict diagnostics for intended ADRs, workflows, config, graph, and the planned `roko-gateway` crate. It does not validate the actual prerequisites in `task.context.read_files`.
- Expected behavior: missing task outputs are valid; missing `context.read_files` inputs remain strict diagnostics unless a declared same-plan task dependency or loaded `depends_on_plan` plan produces that exact path; existing prerequisites pass.
- Acceptance requirements: implement the schema distinction without path/plan special cases; add positive and adversarial tests; preserve strict failure for undeclared missing prerequisites; run both validator integration suites, formatting, `cargo check -p roko-cli`, rebuild the CLI, and report backlog/self-heal strict results without changing manifests.
- Explicit non-goals: editing task manifests, hiding diagnostics by plan/path identity, weakening strict mode, changing runtime task parsing, or treating outputs from unrelated tasks as available.
- Dependencies and their integration commits: CTRL-01 integrated at `1a385eb52c405e9471f0ad7e23cae9650c570290`; coordinator status at base `e736f324bf6f1c6840d9d011c9189fe26e4cf052`.

Reproduction:
- Pre-fix command: `/Users/will/dev/nunchi/roko/roko/target/debug/roko plan validate --strict tmp/status-quo/backlog/plans`
- Expected: validator distinguishes authored outputs from prerequisites.
- Actual: exit 1 with `25 diagnostics in 55 plans`; all diagnostics are emitted from output paths declared in `files`, including 14 references under the not-yet-created `crates/roko-gateway`.
- Control command: `/Users/will/dev/nunchi/roko/roko/target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans`
- Actual: exit 0 with `0 diagnostics in 6 plans`.

Implementation:
- Design and invariants: `files` and `write_files` are collected as outputs, while only `context.read_files[*].path` is collected as a prerequisite. A missing prerequisite is accepted only when the exact path is produced by the transitive closure of the task's declared same-plan `depends_on` tasks or by a loaded plan named in `depends_on_plan`. Outputs from unrelated tasks or plans never satisfy an input. Existing `PLAN_030`/`PLAN_031` warning severities and strict-mode warning failure are preserved.
- Files/symbols changed: `crates/roko-cli/src/plan_validate.rs` adds typed path classification, plan-output indexing, dependency output closure, and prerequisite-only diagnostics; `crates/roko-cli/tests/plan_validate.rs` adds strict CLI coverage; `crates/roko-cli/tests/plan_validation.rs` adds validator coverage for missing/existing/output/dependency cases.
- Compatibility/migration: no task schema, CLI flag, manifest, lockfile, or public command changed. Existing plans continue using `files`, `write_files`, `context.read_files`, `depends_on`, and `depends_on_plan`; validation now applies their documented meanings.
- Failure/recovery/security behavior: malformed plans remain governed by existing schema diagnostics. Missing undeclared prerequisites remain warnings and therefore fail `--strict`. The implementation does not read prerequisite contents or execute plan commands and introduces no external mutation.

Verification:
- `cargo fmt --all -- --check` — exit 0.
- `cargo test -p roko-cli --test plan_validation` — exit 0; 24 passed, 0 failed. The existing test-harness `missing documentation for the crate` warning remains.
- `cargo test -p roko-cli --test plan_validate` — exit 0; 18 passed, 0 failed.
- `cargo check -p roko-cli` — exit 0.
- `cargo build -p roko-cli --bin roko` — exit 0.
- `target/debug/roko plan validate --strict tmp/status-quo/backlog/plans` — exit 1 with 16 prerequisite diagnostics in 55 plans. This replaces the baseline's 25 output-path false positives with actual missing `context.read_files` inputs for manifest remediation under CTRL-07; no `files`/`write_files` output is diagnosed.
- `target/debug/roko plan validate --strict tmp/status-quo/self-heal/plans` — exit 0 with 0 diagnostics in 6 plans.
- `git diff --check` — exit 0.

Review readiness:
- Implementation commit: `ea018feedcbccca3a3d922d293721134e6c7e829`.
- Diff scope reviewed: only the three reserved validator source/test files and this evidence record are modified; no manifest or lockfile changed.
- Known limitations: cross-plan dependency outputs are available only when the producer plan is loaded in the same validation run. Path matching is deliberately exact and lexical, matching the authored schema; aliases or undeclared producers are not inferred.
- Required reviewer focus: output/prerequisite classification, dependency closure, undeclared-producer rejection, and strict backlog/self-heal results.

Integration:
- Review evidence: pending.
- Integration commit: pending.
- Post-merge commands/results: pending.
- Final status: `IMPLEMENTED_UNREVIEWED` after commit.
