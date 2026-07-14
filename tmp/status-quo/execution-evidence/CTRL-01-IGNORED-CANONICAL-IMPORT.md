# CTRL-01 ignored canonical import implementation evidence

Assignment:

- Plan: Wave 0 `CTRL-01`, reopened ignored-canonical-control-plane import
- Base SHA: `98a238aed98549a6a0e43077124cc7146a815799`
- Branch: `agent/CTRL-01-ignored-canonical-import`
- Worktree: `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-01-IGNORED-CANONICAL`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `.gitignore`, `plans/**`, `.roko/GAPS.md`, and this evidence file
- Implementation commit: `699df4e0ea34bddabc4516695d28d1bf41328774`

## Requirement

The sealed root contained canonical control-plane inputs that the base checkout could not see because `.gitignore` excluded both `/plans/` and the complete root `.roko/` directory. Base-tree inspection returned no tracked paths for `plans/` or `.roko/GAPS.md`. Git history identified `7899494d3` as the commit that removed the then-tracked plans and added the plans ignore rule while leaving later P08-P34 files only on disk.

The required outcome was to recover, without semantic edits:

- all 33 intended non-`.DS_Store` files under the sealed root's `plans/` directory;
- root `.roko/GAPS.md`;
- one of five byte-identical architecture queue sources as `plans/architecture-core-queue/tasks.toml`;
- minimal ignore behavior that exposes canonical root plans and only root `.roko/GAPS.md`, while preserving runtime and nested-workspace `.roko` ignores.

Explicit non-goals were importing `.DS_Store`, AppleDouble metadata, logs, runtime state, or `.claude` worktrees; changing task/meta status; reconciling historical plan counts; editing the master/shared backlog; or executing any plan. CTRL-15 remains the owner of executable ownership/status reconciliation, and CTRL-16 remains the owner of stale implementation-order references.

The source authorities were the sealed root and recovery bundle:

```text
/Users/will/dev/nunchi/roko/roko
/Users/will/.local/state/roko/status-quo-20260714T073140Z/ignored-canonical-control-plane.tar.gz
```

The archive SHA-256 is `01c10b4565c1a897c92ced109c7f351fcb35513816860d094efa446da62c34e0`.

## Reproduction

Before the change:

```text
git ls-tree -r --name-only 98a238aed98549a6a0e43077124cc7146a815799 -- plans .roko/GAPS.md
<no output>

base .gitignore rules:
.roko/
**/.roko/
/plans/
```

Thus the canonical queue, plan index/order, and gaps record existed in the sealed checkout but could not participate in Git review or clean-checkout validation.

## Implementation

- Imported the 33 sealed-root `plans/` payload files byte-for-byte: 27 P08-P34 manifests, `architecture-defi-critical-path`, `e2e-smoke`, the two superseded manifests, `INDEX.md`, and `_meta/IMPLEMENTATION_ORDER.md`.
- Imported `.roko/GAPS.md` byte-for-byte.
- Recovered the architecture queue as `plans/architecture-core-queue/tasks.toml`. Its bytes match every one of the five sealed/archived sources and the last tracked historical architecture queue.
- Removed the root `/plans/` ignore rule.
- Kept `**/.roko/` for nested workspaces, re-included only the root `.roko` directory, ignored all of its root children, and then re-included only `/.roko/GAPS.md`.

No task or meta status changed. The imported `plans/INDEX.md` remains the sealed historical file: its declared 29 executable plans/120 tasks and two superseded plans/66 tasks exactly match the 31 manifests it lists. The separately recovered 24-task architecture queue is intentionally accounted for outside those imported headline figures until CTRL-15 reconciles canonical ownership and status. A current validator-generated index would include that queue as 30 plans/144 tasks, but replacing the sealed index here would violate the byte-for-byte import assignment and prematurely perform CTRL-15 reconciliation.

## Verification

### Archive scope and byte identity

The archive contains 39 intended payload entries plus 72 macOS AppleDouble metadata entries. The payload set is exactly 33 root plan files, one gaps file, and five architecture-source copies; it contains no `.DS_Store`, log, or runtime payload. No AppleDouble file was materialized in the worktree.

```text
shasum -a 256 ignored-canonical-control-plane.tar.gz
01c10b4565c1a897c92ced109c7f351fcb35513816860d094efa446da62c34e0

ARCHIVE_SCOPE_OK intended_payloads=39 AppleDouble_metadata=72
  canonical_plans=33 gaps=1 architecture_copies=5 forbidden_payloads=0

BYTE_MATCH_OK sealed=archive=imported files=34
  aggregate_sha256=be1115dde664b5b46429ba77f97e084cbbae3e55134245ffc01b528ac4545b23

ARCHITECTURE_SOURCE_OK copies=5
  sha256=3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5
```

The 34-file aggregate covers the 33 original `plans/` payload files plus `.roko/GAPS.md`, using sorted records of `SHA-256 + relative path`. The architecture queue is verified separately because its destination path is new.

Final imported counts:

```text
plans regular files: 34
tasks.toml manifests: 32
root .roko tracked files: 1 (.roko/GAPS.md)
.DS_Store / AppleDouble materialized: 0
```

### TOML and index structure

Python 3 `tomllib` parsed all 32 manifests and asserted that each `[meta].total` equals the number of `[[task]]` records, all plan IDs are unique, and the corpus contains 210 tasks:

```text
TOML_OK manifests=32 meta_totals_match=true unique_plans=true all_tasks=210
INDEX_OK declared_and_actual_executable=29/120/0_done/120_ready
  superseded=2/66
SEPARATE_QUEUE_OK architecture-core-queue=24/0_done/24_ready
  and intentionally absent from imported historical index
```

### Integrated validator

The integrated binary at `integration/target/debug/roko` was run against a disposable copy of `plans/` from a disposable repository root whose `crates`, `apps`, `demo`, `contracts`, `tests`, and `Cargo.toml` resolve to the actual worktree. This is necessary because the current validator regenerates `plans/INDEX.md` relative to its working directory as a side effect even for `plan validate`. The disposable root prevents that generated output from changing the required byte-identical source index.

```text
roko plan validate --strict plans --color never
0 diagnostics in 32 plans
exit 0
```

The disposable generated index reported 30/144 because it includes the recovered architecture queue. The source `plans/INDEX.md` was rechecked afterward at sealed SHA-256 `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.

### Ignore behavior

`git check-ignore -v --no-index` proves:

```text
VISIBLE .roko/GAPS.md
VISIBLE plans/INDEX.md
VISIBLE plans/P34-verification-sweep/tasks.toml
VISIBLE plans/architecture-core-queue/tasks.toml

IGNORED .roko/INDEX.md                         by /.roko/*
IGNORED .roko/VERSION                          by /.roko/*
IGNORED .roko/runtime/agent-pids.json          by /.roko/*
IGNORED .roko/state/executor.json              by /.roko/*
IGNORED .roko/prd/ideas.md                     by /.roko/*
IGNORED nested/workspace/.roko/GAPS.md          by **/.roko/
IGNORED plans/.DS_Store                        by .DS_Store
```

`git ls-files .roko` contains only `.roko/GAPS.md` after staging. Validation-created ignored `.roko` indexes/logs were removed; no runtime artifact remains.

### Diff hygiene

```text
git diff --cached --check
PASS

implementation commit paths: 36
  .gitignore: 1
  .roko/GAPS.md: 1
  plans/: 34
unexpected paths outside reserved scope: 0
```

No compilation gate is required for this byte-preserving control-plane import. Strict plan validation and complete TOML parsing exercise the affected data path.

## Review readiness

- Implementation commit: `699df4e0ea34bddabc4516695d28d1bf41328774`
- Known limitation: imported status/count claims remain historical until CTRL-15; this commit deliberately does not mark work done or reconcile ownership.
- Required reviewer focus: independently verify archive/source/destination hashes, all five architecture copies, intended file counts, ignore precedence for root versus nested `.roko`, validator side effects/isolation, imported index arithmetic, and absence of junk/runtime imports.

Integration:

- Review evidence: pending independent review
- Integration commit: pending
- Post-merge verification: pending
- Final status: implementation committed; not DONE before accepted review and integrated proof
