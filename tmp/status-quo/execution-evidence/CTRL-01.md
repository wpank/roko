# CTRL-01 implementation evidence

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0
- Base SHA: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Branch/worktree: `agent/CTRL-01-preserve-control-plane` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-01`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: the 23 paths in the sealed `intended-control-plane-files.txt`, the 56 paths in `ignored-control-plane-files.txt`, `.gitignore`, this evidence file, and the coordinator checkpoint in the master

Requirement:
- Original defect or missing behavior: the July 14 audit/control-plane corpus existed only as untracked work in the original checkout, so it was not recoverable from Git or available to an isolated integration branch. In addition, an unanchored `plans/` ignore rule hid all 56 canonical backlog manifest files from standard status/inventory commands.
- Acceptance requirements: preserve the exact root state externally; attribute every untracked artifact; import only the coherent control-plane corpus; leave logs, symlinks, and unrelated artifacts untouched; independently review the exact candidate; merge and verify content identity.
- Explicit non-goals: changing product behavior, absorbing transient logs, rewriting the precursor commit, or mutating the sealed original checkout.
- Dependencies and their integration commits: clean baseline `3041d095d4daebed2c9e05c63eacb18e668e37e3`.

Reproduction:
- Pre-fix command: `git -C /Users/will/dev/nunchi/roko/roko status --short` followed by `git check-ignore -v tmp/status-quo/backlog/plans/E01-execution-engine/tasks.toml`
- Expected: intended control-plane documents are versioned and available from the integration history.
- Actual: 23 intended paths were visibly untracked, 56 canonical manifest files were silently ignored by `.gitignore:36`, and 15 unrelated log/symlink artifacts were visibly untracked.

Implementation:
- Design and invariants: the original checkout was sealed read-only after a binary diff, staged diff, untracked inventory, branch/worktree inventory, repository bundle, and two content archives were written under `/Users/will/.local/state/roko/status-quo-20260714T073140Z` and checksummed.
- Files/symbols changed: exact import of 79 attributed audit, roadmap, source-coverage, manifest, master, and remediation-script paths; anchor the root workspace plan ignore rule; this evidence record; coordinator launch/checkpoint fields.
- Compatibility/migration: no product or schema behavior changes.
- Failure/recovery/security behavior: the recovery bundle is external to the repository; unrelated user artifacts remain in place and are separately archived; no remote or external mutation occurred.

Verification:
- Command: `shasum -a 256 -c /Users/will/.local/state/roko/status-quo-20260714T073140Z/SHA256SUMS`
- Exit/result: exit 0 after adding the ignored-manifest archive; both patches, all three untracked-content archives, and the 3.3 GiB repository bundle match their recorded SHA-256 values.
- Command: compare each imported path byte-for-byte with the sealed root source and confirm the worker contains no unrelated untracked artifacts.
- Exit/result: exit 0 for all 78 unchanged imported files; the only intended source deltas are the populated master checkpoint and the anchored `.gitignore` rule. The backlog strict validator now reaches all 55 plans and truthfully reports the 25 Wave 0 diagnostics instead of failing because the directory is absent.

Review readiness:
- Implementation commits: incomplete first import `e888da882db27d2dcd3fa03de968174cedb51ec4`; corrected manifest/import commit `5a0abcfaa252bf58d6b2480e935a99054eebe2a1`.
- Diff scope reviewed: replacement candidate spans 81 files, 48,095 insertions, and 2 deletions: 79 attributed control-plane paths plus `.gitignore` and this evidence record.
- Known limitations: the existing root `.git` is write-protected by the execution environment; a writable bare coordinator clone owns the integration branch/worktrees. Final import/merge into original `main` requires that local filesystem permission to be lifted.
- Required reviewer focus: exact path attribution, byte identity, archive completeness/checksums, absence of logs/secrets, and correctness of the root seal.
- Prior review disposition: candidate `e13ec0a86680028f9d333962eb5d81193b5c4772` was rejected for omitting the 56 ignored manifests and recording the wrong first-import SHA; both findings are corrected in the replacement candidate.

Integration:
- Review evidence: pending.
- Integration commit: pending.
- Post-merge commands/results: pending.
- Final status: `IMPLEMENTED_UNREVIEWED` after commit.
