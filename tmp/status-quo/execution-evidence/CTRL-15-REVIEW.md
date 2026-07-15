# CTRL-15 independent review

- **Verdict:** `REJECTED`
- **Candidate:** `763f47308ebc880949f57408001cac4d5b22e85c`
- **Exact base:** `ebcc3add020af2a3ff2f3041f721839c16463be2`
- **Review worktree:** `reviews/CTRL-15-763f47308ebc`
- **Confidence:** high

## Independent reconstruction

I read the complete master, all 29 sealed-baseline manifests, the separately
recovered architecture queue, all current E/SH manifests, CTRL-08/09/13/14
evidence, backlog 02/03, the parser and validator implementations, relevant
current production paths, candidate/base history, the complete candidate diff,
and the worker evidence. I reconstructed the baseline and ownership mapping from
the manifests before comparing it with the candidate ledger.

The 29-plan baseline contains exactly 120 unique qualified task IDs. The ledger
is an exact 120-row bijection: 99 retained rows and 21 roll-ups in the declared
groups P11=4, P12=5, P14=3, P18=5, P29=2, and P30=2. All 99 retained task
dictionaries and all 29 meta dictionaries are TOML-semantically identical to
base, including order, status, files, context, verification, acceptance, and
dependencies. Every baseline task remains `ready`.

I independently challenged each roll-up group:

- P11 correctly separates the retained generation-validation task from E01's
  runner default and E12's stronger removal of the legacy feature facade.
- P12's five container-shape prescriptions are covered by SH02/E01's effective
  capacity, DAG, and task-lifecycle ownership plus SH04's structured parallel
  output contract.
- P14's legacy-orchestrator edits are correctly replaced by E05's live-runner
  inputs, canonical seven-rung proof, and toggle removal.
- P18's delimiter and ad-hoc bridge work is correctly replaced by SH04's
  structured identity, typed output/usage, connected TUI, and diagnosis path.
- P29 T1/T2 are literal duplicates of qualified owners
  `P10-slash-command-flags#T3/#T4`; their owner checks correctly resolve local
  IDs `T3/T4` in the named P10 manifest.
- P30 T1/T2 are correctly narrowed to qualified owner
  `P27-provider-error-ux#T1`; provider-aware effective-configuration behavior is
  stronger than unconditional OpenAI/Gemini warnings, and its checks correctly
  resolve local ID `T1` in P27.

There is no mapping disagreement. All 21 rows have `files = []`, role
`quick-reviewer`, task-scoped acceptance and verification, owner-plan scheduler
edges, and no product/API authority. The named implementation tasks exist and,
together with each roll-up's preserved stronger acceptance check, cover the
original outcomes.

The recovered `architecture-core-queue` is separate from the sealed population:
24/24 tasks remain `ready`, have nonempty implementation write scopes, and do not
appear in the 120-row ledger. The regenerated index is correctly 30 executable
plans/144 tasks, while the ownership ledger and both updated historical notices
preserve the explicit 29-plan/120-task baseline boundary. The two superseded
self-dev plans remain excluded at 66 tasks.

## Independent verification

I added a temporary `roko-cli` integration test that imported the public
`roko_cli::task_parser::TasksFile`. It parsed the 29 baseline manifests and the
architecture queue and asserted the 120-ID bijection, the six exact roll-up
groups, roll-up parser semantics, and the separate 24-task population:

```text
CARGO_TARGET_DIR=integration/target cargo test -p roko-cli \
  --test ctrl15_review_semantics -- --nocapture
test independently_parses_ctrl15_ownership_population ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test was removed. No review target was created.

A disposable exact-candidate archive produced:

```text
tracked TOMLs parsed: 193; parse errors: 0
backlog strict: 0 diagnostics in 55 plans; exit 0
self-heal strict: 0 diagnostics in 6 plans; exit 0
top-level strict: 94 diagnostics in 32 plans; exit 1
top-level diagnostic codes: PLAN_031 only
generated plans/INDEX.md SHA-256:
27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
index: 30 executable plans / 144 tasks; 2 excluded plans / 66 tasks
```

The candidate's write census is reproducible: 1,658 claims, 586 unique paths,
and 161 paths claimed by more than one plan. All 21 CTRL-15 roll-ups contribute
zero claims.

The exact diff is one commit over the stated base and exactly 11 paths: six
top-level manifests, the ownership ledger, generated index, backlog 02/03, and
worker evidence. It changes no production/test/Cargo/lockfile, canonical E/SH
manifest, master/status record, or implementation-order file. `git diff
--check` passes. The review worktree was clean before this review record; the
disposable base/candidate archives and temporary test were removed.

## Rejection finding

### High: graph proof repeats the task-runtime versus meta-edge ambiguity

`tmp/status-quo/execution-evidence/CTRL-15.md` reports only:

```text
unique plan dependency edges: 171
```

The number 171 is numerically correct for the **all-declared** edge set, but the
committed evidence neither labels it that way nor records the parser/runtime
edge set separately. `TasksFile` exposes task-level `depends_on_plan`, while two
additional declarations live at `meta.depends_on_plan`; the distinction was an
explicit CTRL-09 correction and is required again here.

Independent exact-base and candidate traversal gives:

```text
                              raw refs   unique source->target edges
base task depends_on_plan          320   160
candidate task depends_on_plan     345   169
candidate meta.depends_on_plan       2     2
candidate all-declared             347   171
task/meta overlap                            0
same-plan task references          849
unresolved same-plan references      0
unresolved plan references           0
runtime cyclic SCCs                  0
all-declared cyclic SCCs             0
```

The 21 roll-ups add 25 task-level references but only nine new unique
source-plan pairs, so the candidate runtime edge count is 169, not 171. The two
disjoint meta edges are
`E46-github-workflow-integration -> E01-execution-engine` and
`E48-rate-limit-budgeting -> E01-execution-engine`. A result of 173 is not
reproducible at this exact base/candidate.

This does not invalidate the manifests, mapping, scheduler resolution, or
acyclicity. It does invalidate the required exact committed proof and would
reintroduce the same ambiguous graph accounting that CTRL-09 had to correct.

## Required next action

Do not merge this review as acceptance. Submit a new immutable candidate that
updates `CTRL-15.md` to record and label the exact 345/169 task-runtime,
2/2 meta, and 347/171 all-declared counts, plus 849 local references, zero
unresolved references, and zero runtime/all-declared SCCs. Preserve the six
manifests, ownership ledger, index, backlog notices, and 99/21 mapping unchanged
unless another independently reviewed correction is required. Rerun the parser,
graph, TOML, disposable strict-root, index, scope, and cleanliness checks, then
obtain fresh independent review of the replacement commit.
