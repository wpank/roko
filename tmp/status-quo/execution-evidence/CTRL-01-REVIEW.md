# CTRL-01 independent review

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0
- Base SHA: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Candidate commit: `32885633d19efe322832a2aa63ee913d7a4a6174`
- Review branch/worktree: `review/CTRL-01-32885633d19e` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-01-32885633d19e`
- Reserved write scope: this review record only

Requirement reconstructed independently:
- Preserve the sealed root checkout and its Git history in an external recovery bundle.
- Attribute and preserve all existing visible and ignored control-plane artifacts without absorbing unrelated logs, symlinks, secrets, or transient state.
- Import the coherent control-plane corpus byte-for-byte except for the documented coordinator checkpoint, and repair the ignore rule that hid canonical nested plan manifests.
- Leave the candidate clean and make backlog validation reach the real Wave 0 plan corpus.

Prior rejection and disposition:
- Candidate `e13ec0a86680028f9d333962eb5d81193b5c4772` was rejected because unanchored `.gitignore:36` hid 56 canonical `tmp/status-quo/backlog/plans/**` files, so the candidate contained no backlog plan directory and strict validation exited 2. Its evidence also named a nonexistent first-import commit.
- The replacement imports all 56 ignored files, anchors the ignore as `/plans/`, records the actual first-import commit `e888da882db27d2dcd3fa03de968174cedb51ec4`, and documents the corrected import commit `5a0abcfaa252bf58d6b2480e935a99054eebe2a1`.

Changed-line and path review:
- Inspected the exact `3041d095d..32885633d` diff: 81 paths, 48,095 insertions, and 2 deletions.
- Compared the candidate path set with the two intended inventories plus `.gitignore` and the implementation evidence; there were no missing or extra paths.
- Compared all 22 unchanged visible imports and all 56 ignored manifest imports byte-for-byte with the sealed root. The only intended source differences are the populated master checkpoint and the scoped `.gitignore` repair.
- Inspected `.gitignore`, the master checkpoint, the implementation evidence, commit graph, and replacement diff directly. The imported corpus is content preservation rather than a product-code change.

Independent verification:
- `shasum -a 256 -c /Users/will/.local/state/roko/status-quo-20260714T073140Z/SHA256SUMS`
  - Exit 0: both patches, all three content archives, and the 3.3 GiB repository bundle matched.
- `git -C /Users/will/dev/nunchi/roko/roko bundle verify /Users/will/.local/state/roko/status-quo-20260714T073140Z/repository.bundle`
  - Exit 0; complete history recorded.
- Compare current root HEAD, branch, porcelain-v2 status, and visible untracked zlist with their sealed records.
  - All four comparisons matched exactly; the root remains at `3041d095d4daebed2c9e05c63eacb18e668e37e3` on `main`.
- Compare inventory unions with current root discovery and archive member lists.
  - Exact partitions: 23 visible intended paths, 56 ignored canonical paths, and 15 unrelated paths; no overlap, omission, or extra archive member.
- Extract all three archives and compare regular-file bytes and symlink targets with the sealed root.
  - Zero content or symlink-target mismatches across all 94 artifacts.
- Compare every unchanged imported file with the sealed root.
  - Zero mismatches across 78 files; the master differs only by the documented checkpoint.
- `git check-ignore -v --no-index plans/nonexistent tmp/status-quo/backlog/plans/nonexistent`
  - Only root `plans/nonexistent` matched `.gitignore:38:/plans/`; the canonical nested plan path remained visible.
- `/Users/will/dev/nunchi/roko/roko/target/debug/roko plan validate --strict tmp/status-quo/backlog/plans`
  - Exit 1 with `25 diagnostics in 55 plans`, the expected pre-remediation Wave 0 state; it no longer fails with a missing directory.
- `bash -n tools/run-status-quo-remediation.sh`
  - Exit 0.
- `git diff --check 3041d095d4daebed2c9e05c63eacb18e668e37e3 32885633d19efe322832a2aa63ee913d7a4a6174`
  - Exit 0.
- Candidate-path scan for committed log/JSONL artifacts and private-key, AWS, GitHub, Slack, and API-key signatures.
  - No committed transient-log path or secret signature found.
- `git status --short --branch`
  - Clean before creating this review record.

Adversarial checks:
- Re-ran the exact missing-directory reproduction from the rejected candidate; the replacement contains 56 plan-corpus files, including 55 `tasks.toml` manifests.
- Verified the anchored rule still ignores an untracked root workspace `plans/` path while no longer hiding nested canonical plans.
- Verified archive contents, not only archive checksums and filenames.
- Verified the unrelated archive contains the 14 preserved symlinks and one JSONL log while none is present in the candidate diff.
- Verified both implementation commit IDs resolve and the candidate is an immutable descendant of the clean base.

Verdict:
- **ACCEPTED**
- Confidence: high.
- Required next action: merge this review commit into `status-quo/integration-status-quo-20260714T073140Z`, verify candidate ancestry, rerun the focused preservation/validation checks on the integrated head, and only then mark CTRL-01 complete.
