# CTRL-02 sealed-checkout breach reconciliation

During the independent final precursor census, the programme detected an untracked
`tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH-REVIEW.md` in the
original sealed checkout. It was absent from the Wave 0 `original-status.txt`, had
mtime `2026-07-14 16:25:49`, and SHA-256
`7cb77fcac159e166ff592384fe082e6d47074b8f3cd29a4cb626d511a0cf0106`.

The file was a programme-created reviewer draft. It differed from the canonical
record committed at integration commit `eed6bc786` (SHA-256
`cf8bff45db9748e245ff0d2830400eb0725e7c798510562851abec70181aba6d`) in exactly
one line: the draft said Tokio `1.52.3`, while the repository lockfile and canonical
review correctly say `1.51.1`. No other byte differed.

The exact draft is durably reconstructible from the canonical blob plus that
one-line substitution. Its hash, diff, provenance, and deletion disposition are
also recorded outside the repository at
`$HOME/.local/state/roko/status-quo-20260714T073140Z/seal-breach-CTRL-02-WORKTREE-REATTACH-REVIEW.txt`.
After preservation, the superseded untracked duplicate was deleted from the
original checkout with a named `apply_patch` operation. The coordinator then
reproved original HEAD/branch and porcelain-v2 status against the saved Wave 0
inventory. This reconciliation changes no pre-existing user artifact and does not
alter the canonical rejection or its pending correction.
