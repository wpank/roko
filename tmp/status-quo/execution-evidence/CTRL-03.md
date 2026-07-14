# CTRL-03 — canonical backlog plan dependency IDs

## Assignment

- Task: `CTRL-03`
- Base: `e736f324bf6f1c6840d9d011c9189fe26e4cf052`
- Branch: `agent/CTRL-03-canonical-plan-ids`
- Dependency: `CTRL-01` is integrated; this change does not overlap the validator work owned by `CTRL-06`.
- Reserved write scope: the five backlog manifests named below and this evidence file only.

## Requirement and defect

`depends_on_plan` is a cross-plan reference and must use the exact `[meta].plan`
identifier of an internal backlog manifest. The five assigned manifests contained 36
references to shorthand or case-drifted internal aliases. A complete pre-change
census of every backlog manifest found 47 references that did not match an internal
plan ID: the 36 defective aliases plus 11 intentional references to external `P*`
plans.

Expected behavior is that all 36 internal references resolve exactly, while the 11
external `P*` references remain unchanged. This task changes no task ordering,
status, count, production behavior, validator logic, external-plan ownership, master
checklist state, or shared index.

## Context reviewed

- Entire `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`.
- Entire assigned manifests:
  - `tmp/status-quo/backlog/plans/DOC-v1-cognition/tasks.toml`
  - `tmp/status-quo/backlog/plans/E06-COMPOSE-UNIFY/tasks.toml`
  - `tmp/status-quo/backlog/plans/E07-learning-knowledge/tasks.toml`
  - `tmp/status-quo/backlog/plans/E10-FRONTEND-CONTRACT/tasks.toml`
  - `tmp/status-quo/backlog/plans/E14-providers-tools/tasks.toml`
- Current `[meta].plan` identifiers across all 55 backlog manifests and recent Git
  history for the assigned manifests.

## Reproduction

The reproduction enumerated all `[meta].plan` values first, then every task's
`depends_on_plan` value, and printed references absent from the internal-ID set.
Before the fix it printed 47 rows. Grouped counts were:

~~~text
13 E01
1 E01-EXECUTION-ENGINE
2 E02
2 E03
3 E05
1 E06
6 E07
4 E08
1 E09
1 E10
1 E17
1 E18
1 P08-search-command-fix
1 P09-tool-alias-fix
1 P16-safety-contracts
2 P19-cascade-router-acp
3 P22-acp-tool-permission
1 P23-prd-pipeline-fix
1 P25-mcp-acp-passthrough
1 P28-image-support
~~~

The non-`P*` subtotal is exactly 36. The external-plan subtotal is exactly 11.

## Implementation

Only `depends_on_plan` strings were replaced. The diff contains 17 removed and 17
added manifest lines, representing exactly 36 removed aliases and 36 canonical IDs:

- `DOC-v1-cognition`: 26 references across seven tasks.
- `E06-COMPOSE-UNIFY`: three `E01` references.
- `E07-learning-knowledge`: four `E01` references; its `P19-cascade-router-acp`
  reference is unchanged.
- `E10-FRONTEND-CONTRACT`: one `E01` and one `E03` reference.
- `E14-providers-tools`: one case-drifted `E01-EXECUTION-ENGINE` reference.

Implementation commit: the immutable candidate commit containing this evidence;
the coordinator records its SHA from `git rev-parse HEAD`.

## Verification

### Exact internal resolution and external preservation

The same global census after the change prints only these 11 unresolved external
references:

~~~text
P16-safety-contracts       E04-T06
P22-acp-tool-permission    E04-T14
P19-cascade-router-acp     E07-T09
P08-search-command-fix     E16-T1
P23-prd-pipeline-fix       E16-T2
P09-tool-alias-fix         E16-T2
P22-acp-tool-permission    E17-T01
P19-cascade-router-acp     E17-T02
P25-mcp-acp-passthrough    E17-T03
P22-acp-tool-permission    E17-T04
P28-image-support          E17-T05
~~~

Result: all internal references resolve; all 11 external references are preserved.
A scoped exact-string search also finds none of the 12 replaced aliases in the five
assigned manifests. A diff counter reports
`legacy_refs_removed=36 canonical_refs_added=36`.

### Manifest validation

Using `target/debug/roko` from the sealed original checkout (reported provenance:
`roko 0.1.0 ... git 1649c18b2`):

- Each of the five assigned directories parsed successfully. Four reported zero
  diagnostics; `E06-COMPOSE-UNIFY` reported only its pre-existing missing future ADR
  warning for `tmp/status-quo/backlog/decisions/E06-canonical-surface.md`.
- `plan validate --strict tmp/status-quo/self-heal/plans`: `0 diagnostics in 6 plans`,
  exit 0.
- `plan validate --strict tmp/status-quo/backlog/plans`: all 55 manifests parsed and
  reported 25 pre-existing file-reference diagnostics, exit 1. The count and paths
  are unchanged by CTRL-03 and are assigned to other Wave 0 control-plane work.

The global exact-ID census is independent of the current validator's cross-plan
classification and is the acceptance proof for this repair.

A separate current-source `cargo run` was attempted with an isolated
`CARGO_TARGET_DIR`. It produced no diagnostics or executable output during five
minutes of dependency compilation and was interrupted cleanly with exit 130; it is
not represented as a passing or failing gate. No compiler process was left running.

### Repository hygiene

- `cargo fmt --all -- --check`: exit 0.
- `git diff --check`: exit 0.
- Manifest diff counter assertion (`36` removed, `36` added): exit 0.
- Changed paths before this evidence file: exactly the five reserved manifests.

## Review readiness

The candidate is ready for an independent reviewer. The reviewer should reconstruct
the internal plan-ID set from all backlog manifests, rerun the 47-to-11 census, check
that only the five assigned manifests plus this evidence file changed, and confirm
that no external `P*` reference was canonicalized or removed.

## Integration

Not yet integrated and not DONE. The coordinator/integration owner must record the
candidate SHA, obtain independent acceptance of that immutable candidate, merge it
in dependency order, rerun the census and validation on the integration branch, and
only then update canonical Wave 0 status.
