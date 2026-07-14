# Self-heal changelog

This directory records the changes associated with the executable plans in
`tmp/status-quo/self-heal`. It deliberately distinguishes implemented work from
plan definitions: a task manifest is an acceptance contract, not evidence that
the implementation exists.

## Scope

- Plan catalogue introduced: `437be433a` on 2026-07-11.
- Implementation history audited: `73d28a644..1649c18b2` on 2026-07-11 through
  2026-07-12.
- Audit date: 2026-07-14.
- Working-tree refinements made after `1649c18b2` are described in
  `AUDIT-2026-07-14.md`; they are not counted as completed manifest tasks unless
  their acceptance evidence is complete.

## Batch status at audit start

| Batch | Manifest status | Implemented scope |
|---|---:|---|
| SH01 runner lifecycle | 26 / 28 done | Canonical attempts, terminalization, DAG quiescence, retries, cancellation ownership, producer ownership, and deadline foundations |
| SH02 isolation/recovery | 0 / 6 done | Planned only; an earlier plan-worktree precursor exists, but it does not satisfy task-owned isolation |
| SH03 persistence/integrity | 0 / 6 done | Planned only; SH01 added lifecycle persistence primitives, not the complete SH03 contract |
| SH04 telemetry/TUI | 0 / 8 done | Planned only; an earlier live-telemetry precursor exists, but the structured telemetry batch is not complete |
| SH05 config/dispatch | 0 / 4 done | Planned only |
| SH06 regression harness | 0 / 5 done | Planned only |

## Files

- `SH01-runner-lifecycle.md` — detailed implemented SH01 changes and remaining
  acceptance gaps.
- `SH02-SH06-planned-batches.md` — the contracts that are still pending and the
  partial precursors that must not be over-claimed.
- `COMMIT-INVENTORY.md` — chronological inventory of the 88 commits in scope.
- `AUDIT-2026-07-14.md` — review findings, corrective changes, and verification
  evidence from this audit.

## Changelog rules

1. A task is listed as complete only when its manifest says `status = "done"`.
2. Partial or cross-batch changes are called precursors until all acceptance
   criteria for the owning task are verified.
3. Test commands and failures belong in the dated audit file so historical
   claims remain reproducible.
4. Future self-heal runs should add a dated entry and update the status table
   rather than rewriting earlier evidence.
