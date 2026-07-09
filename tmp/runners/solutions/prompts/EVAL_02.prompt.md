# EVAL_02: Define `EvidenceCollector` trait

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-02`](../ISSUE-TRACKER.md#eval-02)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.2
- Priority: **P0**
- Effort: 6 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Currently, each gate in `roko-gate` spawns its own subprocess: `CompileGate::verify()` creates a `tokio::process::Command("cargo", ["check", ...])` internally. This means evidence (stdout, stderr, exit code) is produced and consumed inside the same function -- no sharing between criteria.

The `EvidenceCollector` trait separates evidence production from evaluation. Collectors produce typed `EvidenceItem`s that multiple criteria can consume.

## Exact Changes

1. Define `CollectorRequirements` struct:
   ```rust
   pub struct CollectorRequirements {
       pub needs_filesystem: bool,
       pub needs_network: bool,
       pub timeout_ms: u64,
   }
   ```
2. Define `EvidenceCollector` trait:
   ```rust
   #[async_trait]
   pub trait EvidenceCollector: Send + Sync {
       fn name(&self) -> &str;
       fn produces(&self) -> &[EvidenceKind];
       fn requires(&self) -> CollectorRequirements;
       async fn collect(
           &self,
           artifact: &ArtifactRef,
           ctx: &Context,
       ) -> Result<Vec<EvidenceItem>, EvalError>;
   }
   ```
3. Implement `ProcessCollector` (spawns a shell command, captures stdout/stderr/exit code as ProcessOutput + ProcessStatus evidence items). Factory methods: `for_compile(build_system: BuildSystem)`, `for_lint(build_system: BuildSystem)`, `for_test(build_system: BuildSystem)`, `for_format(build_system: BuildSystem)`. Import `BuildSystem` from `roko_gate::payload::BuildSystem` (add `roko-gate` as dependency).
4. Implement `DiffCollector` (runs `git diff`, produces Diff evidence).
5. Implement `CompositeCollector` wrapping `Vec<Box<dyn EvidenceCollector>>` that runs all inner collectors and merges results into a single `Vec<EvidenceItem>`.
6. Add `pub mod collector;` to `lib.rs` and re-export public types.

## Design Guidance

`ProcessCollector` should reuse the subprocess spawning pattern from `ShellGate::verify()` at `crates/roko-gate/src/shell.rs` but instead of producing a `Verdict`, it produces `Vec<EvidenceItem>`. The timeout comes from `CollectorRequirements`. The `for_compile()` factory should produce the same command as `CompileGate::new()` -- check `crates/roko-gate/src/compile.rs` for the exact arguments per `BuildSystem`. `CompositeCollector` runs inner collectors sequentially (not parallel) to avoid resource contention.

## Write Scope

- `crates/roko-eval/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit tests for `ProcessCollector` with `echo hello` (passing) and `false` (failing) commands
- [ ] `DiffCollector` test in a temp git repo with a staged change
- [ ] `CompositeCollector` merges results from 2 inner collectors

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit tests for `ProcessCollector` with `echo hello` (passing) and `false` (failing) commands
- `DiffCollector` test in a temp git repo with a staged change
- `CompositeCollector` merges results from 2 inner collectors
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
