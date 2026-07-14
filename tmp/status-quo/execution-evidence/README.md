# Execution evidence conventions

This directory is the append-only audit trail for the Roko remediation programme.
The [master execution checklist](../MASTER-EXECUTION-CHECKLIST.md) remains the
authority for task state, review, integration, and release. This document defines
how that contract is recorded here; it does not weaken or replace the master.

These conventions apply prospectively from CTRL-10. Earlier records remain
historical evidence and must not be renamed or rewritten merely to match this
format. Their identity and chronology are recoverable from their contents and Git
history.

## Evidence set and ownership

One task normally has three evidence layers:

1. implementation evidence authored by the worker;
2. one or more independent review records, each bound to one immutable candidate;
3. integration and post-merge proof authored by the coordinator or integration
   owner.

The worker must not author or commit an acceptance review of their own candidate.
The reviewer must be a different agent/person and use a separate review
branch/worktree. A reviewer does not modify the candidate under review. If review
discovers a required change, the worker produces a new immutable candidate and a
new review cycle.

Only the coordinator or integration owner records canonical task status, integration
commits, post-merge proof, or programme-level completion. An `ACCEPTED` review is not
`DONE`.

## Names and chronology

- `TASK-ID.md` is the implementation evidence named by the master.
- `TASK-ID-REVIEW.md` is the first independent review record.
- If another review cycle is needed, preserve the first file and add
  `TASK-ID-REVIEW-02.md`, then `-03.md`, in chronological order. A descriptive
  suffix such as `-F1-REJECTED` or `-FINAL` may follow the ordinal, but must not
  replace it.
- A distinct integration addendum may use `TASK-ID-INTEGRATION.md` when adding the
  result to the implementation file would obscure the reviewed candidate.

Never overwrite or delete a rejection, blocked verdict, nonterminal review, or
superseded candidate record. Later records must name the earlier candidate, review
record/commit, findings, correction commits, and final disposition. Git history
alone is not a substitute for a current-tree rejection record after this convention
becomes effective.

The current corpus demonstrates the supported historical shapes:

- a simple accepted pair: [CTRL-03.md](CTRL-03.md) and
  [CTRL-03-REVIEW.md](CTRL-03-REVIEW.md);
- retained rejection cycles:
  [CTRL-06-REVIEW-F1-REJECTED.md](CTRL-06-REVIEW-F1-REJECTED.md),
  [CTRL-06-REVIEW.md](CTRL-06-REVIEW.md), and
  [CTRL-06-REVIEW-FINAL.md](CTRL-06-REVIEW-FINAL.md);
- rejected metadata followed by corrected acceptance:
  [CTRL-07-REVIEW.md](CTRL-07-REVIEW.md) and
  [CTRL-07-REVIEW-FINAL.md](CTRL-07-REVIEW-FINAL.md);
- accurate but nonterminal evidence followed by terminal proof:
  [CTRL-14-REVIEW-NOT-READY.md](CTRL-14-REVIEW-NOT-READY.md) and
  [CTRL-14-REVIEW-FINAL.md](CTRL-14-REVIEW-FINAL.md);
- renewed review after a semantic merge conflict:
  [SH01-T06A-C1-C2-CORRECTION.md](SH01-T06A-C1-C2-CORRECTION.md) and
  [SH01-T06A-C1-C2-CORRECTION-REVIEW-RENEWED.md](SH01-T06A-C1-C2-CORRECTION-REVIEW-RENEWED.md).

## Immutable candidate identity

Every review record must identify, using full commit SHAs:

- the exact candidate commit reviewed;
- its parent or assigned base;
- the implementation commit(s), when the candidate also contains evidence-only
  commits;
- the review branch/worktree and reviewer identity or assignment;
- the exact cumulative path set and diff range reviewed;
- integrated prerequisite commits used by the candidate.

The worker evidence is usually part of the candidate and cannot reliably contain
the SHA of the commit that contains itself. It must identify the assigned base,
branch/worktree, component implementation commits where applicable, reserved scope,
and state that the exact candidate SHA is supplied to the reviewer. The independent
review record is the durable authority for the exact candidate identity.

Do not amend, rebase, force-update, or add commits to a reviewed candidate and keep
the old verdict. Any byte change creates a new candidate. A conflict resolution,
reconstruction on a newer base, or integration-owner semantic edit also creates a
new candidate and requires renewed independent review. A prior review may be cited
as historical context but never transferred to different bytes.

## Implementation evidence contract

Implementation evidence must record:

- assignment: plan/task, base SHA, branch/worktree, integration branch, reserved
  write scope, dependencies and their integration commits;
- requirement: original defect, expected behavior, full acceptance, non-goals, and
  terminal scope;
- reproduction: exact command, working directory, expected result, actual result,
  and why the failure demonstrates the defect;
- implementation: changed files/symbols, design invariants, compatibility or
  migration, and relevant failure/recovery/security/concurrency/resource behavior;
- verification: every exact command and result described below;
- review readiness: component commits, complete cumulative diff scope, known
  limitations, and required reviewer focus;
- integration section: initially pending, then completed only by the coordinator or
  integration owner under the rules below.

Status-only or already-present behavior requires the same evidence: exact historical
implementation commits, current production/test proof, independent review, and
post-merge reconciliation. “Already implemented” is not a terminal status by itself.

## Independent review contract

The reviewer reconstructs the requirement from the task, primary issue/audit
evidence, source, tests, and dependency commits rather than accepting the worker
summary. The review record must include:

- exact candidate/base/component commit identities and complete changed-path scope;
- independence statement and review worktree identity;
- changed-line and unchanged production-path trace;
- independent reproduction and adversarial checks proportional to the risk;
- every exact command/result and artifact disposition;
- one verdict, confidence, and an exact next action.

The canonical verdicts are:

- `ACCEPTED`: the exact candidate satisfies the complete assigned contract and no
  candidate correction remains. Integration and post-merge proof are still pending.
- `REJECTED`: one or more acceptance-blocking findings exist. Each finding names
  severity, file/symbol, reproduction, expected versus actual behavior, and the
  smallest correction. The reviewed candidate must not be merged as accepted work.
- `BLOCKED`: only when the master's blocker contract is met. Record the same blocking
  cause, three materially different remedies, their exact outcomes, why no safe
  in-scope work remains, and the authority or external state needed to resume.

Do not use “accepted with required changes.” Required candidate work means
`REJECTED`. An evidence-accuracy review that does not accept terminal task scope must
say so prominently and must not be treated as task acceptance.

Reviewer-authored adversarial tests may be committed only when separately assigned.
If they alter the candidate tree, they form a new candidate and require independent
acceptance; they do not let a reviewer approve their own changed bytes.

## Commands, results, and artifacts

For every material command, record:

- the exact command without truncating away flags that affect behavior;
- working directory and relevant environment such as `CARGO_TARGET_DIR`, feature
  flags, concurrency, offline mode, or selected binary;
- binary provenance, including Git build identifier or SHA when available;
- exit code and objective result: test counts, diagnostic counts/classes, hashes,
  or parsed assertions;
- whether the command passed, failed, timed out, was interrupted, or was not run.

Never represent an attempted or interrupted command as passing. Preserve environmental
failures such as ENOSPC, target contamination, lost process handles, or missing
generated assets, together with the materially different remedy and final rerun.
Warnings are recorded, not silently omitted.

Every created artifact must have an owner and disposition. Record paths and hashes
for archives, bundles, generated fixtures, logs used as evidence, and binaries where
identity matters. State which programme-created temporary files, processes, and
targets were removed. Do not claim a clean worktree until `git status --short` and
the relevant artifact scan prove it.

## Generated-index hygiene

Commands such as plan validation can regenerate `plans/INDEX.md` and ignored
`.roko` indexes/logs relative to their working directory. Unless index generation is
the assigned change:

1. record the tracked source index hash;
2. run the command from a disposable repository-shaped archive/root;
3. allow generated output only inside that disposable root;
4. rerun the source hash and Git-status checks afterward;
5. remove only the temporary artifacts created by that command.

Do not validate in the source worktree and then hide the side effect with checkout,
reset, or an unexplained rewrite. If a generated index is intentionally changed, the
integration owner must own it, review the generator inputs and output together, and
record the exact regeneration command.

## Integration and post-merge proof

The integration owner records:

- accepted candidate SHA and review-record commit SHA;
- prerequisite ancestry and clean pre-merge status;
- integration merge commit;
- whether the merge was conflict-free;
- exact post-merge commands/results on the integration commit;
- status/index/manifest reconciliation commit where applicable;
- remaining authorized external action, if any;
- final status limited to the exact accepted scope.

If integration changes semantics or resolves a content conflict, abort the
mechanical merge and return the work to a worker-created candidate for renewed
review. After merging, prove both candidate and review ancestry. An implementation
file's pending `Integration` section may be completed only by an append-only,
clearly labelled coordinator reconciliation commit; changing prior worker claims
requires separate review. Prefer a separate integration addendum when that keeps the
immutable candidate easier to audit.

`DONE` requires accepted review, integration, post-merge verification, canonical
status reconciliation, and clean scope. State what the result does **not** complete.
Task-level acceptance must never imply that an enclosing epic, wave, release, or the
programme is complete. `SUPERSEDED` must name the canonical owner and prove
equivalent-or-stronger integrated acceptance.

## Historical compatibility

Records committed before CTRL-10 use several naming and layout variants. They remain
valid historical evidence because the present files and Git graph retain their
candidate identities, verdicts, findings, correction chains, integration commits,
and bounded completion claims. Do not normalize them retroactively.

When relying on an older record, verify its cited commits and current-tree
compatibility. If an old accepted candidate conflicts with the current integration
base, reconstruct the behavior on the current base and obtain a new review, as the
SH01 deadline correction demonstrates. If a legacy rejection existed only in an
older Git version of a reused path, preserve that commit in the evidence chain and
use separate chronological review files for every future cycle.
