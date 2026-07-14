# CTRL-16 independent review

- **Verdict:** `REJECTED`
- **Candidate:** `2022df75387a52cd68b47c3e4ce41c4390601072`
- **Exact parent/base:** `3a4e57f02efa47f3106f54969799c34486b3ed7b`
- **Review branch/worktree:** `review/CTRL-16-2022df753` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-16-2022df753`
- **Confidence:** high

## Independence, scope, and sources

I did not implement the candidate. I read the complete master, the exact one-commit
candidate diff, all four changed files, the current generated index and ownership
ledger, the CTRL-01 ignored-canonical recovery implementation/review, CTRL-05
implementation/review, both CTRL-15 reviews and final evidence, the current and
historical architecture/DeFi/e2e manifests, all three deleted manifests, current
source artifacts bearing the deleted plans' outcomes, and every repository reference
to the four plan names.

The candidate is a direct child of its stated base and changes exactly four paths:

- `plans/_meta/IMPLEMENTATION_ORDER.md`;
- `tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md`;
- `tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md`;
- `tmp/status-quo/execution-evidence/CTRL-16.md`.

It changes no manifest, generated index, ownership ledger, master/status record,
production behavior, Cargo metadata, or lockfile. `git diff --check` passes.

## Independently accepted proof

The core census and history claims reproduce:

- Python `tomllib` parses all 193 tracked TOMLs with zero errors.
- `plans/*/tasks.toml` contains 32 unique manifests: 30 `ready` executable plans
  with 144 tasks and two `superseded` plans with 66 excluded tasks. All manifest
  totals equal their actual task counts.
- All 30 current ready roots appear in the implementation order. The three named
  runnable roots are tracked, non-empty, parseable, and exactly
  `architecture-core-queue=24`, `architecture-defi-critical-path=3`, and
  `e2e-smoke=2` tasks.
- `dry-run-flag`, `live-demo-phase1`, and `live-demo-phase2` have no current
  `plans/<name>/tasks.toml` and no generated-index row.
- Commit `7899494d336d83a7bf3dc95b6592f1b90de02c8f` deleted all three manifests,
  as well as the then-tracked architecture manifest. Its parent blobs contain
  exactly 10, 2, 2, and 24 tasks respectively.
- The current architecture manifest is byte-identical to the parent historical
  blob at SHA-256
  `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5`.
  It has exactly one `Q14-chain-registries-defi-foundation` task. The three DeFi
  parity rows all resolve exactly to that anchor.
- Every local Markdown link in the three changed control documents resolves;
  the eight current-control link targets cited by the candidate all exist.

Using the integrated validator binary reporting Git `7303d2f87` from a fresh
`git archive` under `/private/tmp` independently produced:

```text
backlog:   exit 0; 0 diagnostics in 55 plans
self-heal: exit 0; 0 diagnostics in 6 plans
top-level: exit 1; 94 diagnostics in 32 plans; all 94 are PLAN_031
generated plans/INDEX.md SHA-256:
27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
generated totals: 30 executable plans/144 tasks; 2 excluded plans/66 tasks
```

The source index had that same hash before and after. The disposable archive and
generated `.roko` state were removed, and the review worktree remained clean before
this review record.

These passing results do not cure the contradictions below.

## Rejection findings

### High — a current executable script still runs both removed demo roots

`plans/_meta/IMPLEMENTATION_ORDER.md:81-84` now says the removed roots are absent,
must not be passed to `roko plan run`, and must not be recreated. However,
`scripts/demo-knowledge-feedback.sh:42-43` still executes
`plan run plans/live-demo-phase1/`, and lines 64-65 still execute
`plan run plans/live-demo-phase2/` in its advertised `--live` mode. The script is
tracked, executable, and its header presents `--live` as a current real-agent mode.
Both target manifests are absent.

Reproduction:

```text
git grep -n 'plan run plans/live-demo-phase' -- scripts/demo-knowledge-feedback.sh
scripts/demo-knowledge-feedback.sh:43: plan run plans/live-demo-phase1/
scripts/demo-knowledge-feedback.sh:65: plan run plans/live-demo-phase2/

test -e plans/live-demo-phase1/tasks.toml  # exit 1
test -e plans/live-demo-phase2/tasks.toml  # exit 1
```

Expected: no current executable path advertises and dispatches plan roots that the
canonical implementation order forbids running. Actual: the live demo deterministically
targets two absent roots before it can prove knowledge feedback. This is not merely a
historical prose reference and directly contradicts the candidate's operational rule.

Smallest correction: do not recreate empty or unreviewed plan directories. Repair the
script's live mode against a real retained/current fixture, or make the removed live mode
fail closed with an explicit historical/supersession message while preserving the
simulated mode. Include that path in the corrected candidate's reviewed scope and verify
the script no longer passes either absent root to `roko plan run`.

### Medium — the disposition hides inherited partial artifacts

The historical table says the dry-run plan has no task-for-task supersession and each
live-demo plan has “no current replacement.” Those complete-outcome conclusions are
reasonable, but the candidate did not inspect or record the materially relevant partial
artifacts already in the current tree from ancestor commit
`236686c7a976c8bd1b1ebe07c62bbe185fe06576`:

- `crates/roko-cli/src/dry_run.rs` contains and exports the exact
  `DryRunGate`/`DryRunPreview` data types from dry-run T1, but current `roko run`
  has no `--dry-run` field, `WorkflowRunConfig` has no `dry_run` field, and no
  builder, early-exit branch, or named dry-run tests exist. Its module-level prose
  nevertheless claims that `roko run --dry-run` emits the preview.
- `crates/roko-std/src/greeting.rs` contains the exact phase-1 greeting function,
  but `roko-std/src/lib.rs` does not export the module and the phase-1 test is
  absent. No `format_farewell` function or phase-2 test exists.

Reproduction uses `git show 236686c7...`, `git blame` on both files, the current
source reads above, and symbol searches for the absent wiring/tests. The partial
commit is an ancestor of the candidate.

Expected: “non-equivalence rather than cancellation” records both what survived and
what remains unowned, so a later agent does not duplicate existing structs/function
or mistake them for accepted completion. Actual: the disposition jumps directly from
deleted manifests to “no replacement” and the worker evidence says it reconstructed
the current semantic boundary without mentioning these exact residues.

Smallest correction: update the implementation-order disposition and worker evidence
to identify the inherited partial dry-run/greeting bytes, their exact missing wiring
and tests, and why they are neither a complete replacement nor accepted supersession.
Carry the same concise distinction into the two current-control notices where needed.
Do not mark the deleted tasks done and do not invent a new owner without reviewed
task-level mapping.

### Medium — immutable candidate identity is left as a literal placeholder

`tmp/status-quo/execution-evidence/CTRL-16.md:12` and line 153 contain
`CANDIDATE_SHA_REPORTED_AFTER_COMMIT`. A record cannot contain its own commit SHA, but
the execution-evidence convention requires the worker to state that the exact candidate
identity is supplied by the independent reviewer; it does not permit an unresolved
template token in committed final evidence.

Smallest correction: replace both placeholders with the canonical self-reference-safe
wording (for example, “exact immutable candidate SHA supplied by the independent review
record”). The replacement cumulative candidate will receive its exact identity in the
next review.

## Required next action

Do not merge this review as acceptance. Submit a new immutable cumulative candidate
that:

1. removes or safely repairs the current live script's two absent-root executions
   without manufacturing plan directories;
2. records the exact partial dry-run/greeting residue and non-equivalence;
3. removes both evidence placeholders;
4. preserves the verified 30/144 plus 2/66 counts, 24/3/2 runnable-root facts,
   Q14/three-consumer resolution, history hashes/counts, and generated index bytes;
5. reruns 193-TOML parsing, disposable strict validation/index generation, link/ref
   scans, diff/scope hygiene, and obtains fresh independent review.

Verdict: **REJECTED**.
