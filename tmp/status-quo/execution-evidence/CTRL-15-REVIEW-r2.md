# CTRL-15 r2 independent review

- **Verdict:** `ACCEPTED`
- **Cumulative candidate tip:** `7303d2f8701c24842caec9a3e1850b43de92c906`
- **Substantive candidate:** `763f47308ebc880949f57408001cac4d5b22e85c`
- **Exact base:** `ebcc3add020af2a3ff2f3041f721839c16463be2`
- **Correction:** `7303d2f8701c24842caec9a3e1850b43de92c906`
- **Prior rejection:** `ba179e68243503944ef73c38abdacc46aa58a97e`
  (retained on integration as `3364eacbd`)
- **Review worktree:** `reviews/CTRL-15-r2-7303d2f`
- **Confidence:** high

## Rejection disposition and scope

The prior review accepted the 120-row ownership mapping but rejected its committed
graph evidence because the single reported edge count did not distinguish runtime
task edges from the two disjoint metadata edges. I reread that rejection and then
reviewed the full cumulative 11-path candidate rather than only trusting the r2
summary.

The correction changes exactly one path,
`tmp/status-quo/execution-evidence/CTRL-15.md`. Blob comparison proves that the six
manifests, ownership ledger, generated index, and backlog 02/03 notices are
byte-identical to rejected candidate `763f47308`. The corrected evidence is the only
changed blob. The cumulative chain is exactly two commits over the stated base:

```text
ebcc3add0 -> 763f47308 -> 7303d2f87
```

The cumulative diff is exactly the declared 11 paths. It changes no production or
test source, Cargo manifest/lockfile, canonical E/SH manifest, implementation-order
file, master/status record, or task status. `git diff --check` passes.

## Independent ownership reconstruction

I parsed every sealed-baseline manifest at the exact base and candidate. P08-P34
contain 115 tasks and `architecture-defi-critical-path` plus `e2e-smoke` contain
five, for exactly 29 plans and 120 unique qualified task IDs. Candidate task order,
all 29 meta dictionaries, and all statuses are unchanged. Every task remains
`ready`.

The candidate is an exact 120-row bijection: 99 retained rows and 21 zero-write
acceptance roll-ups in these independently confirmed groups:

| Plan | Retained | Roll-ups | Canonical owners challenged |
|---|---:|---:|---|
| P11 | 1 | 4 | E01-T01 and E12-T03 |
| P12 | 0 | 5 | SH02-T01, E01-T04/T05, SH04-T01/T02 |
| P14 | 0 | 3 | E05-T05/T07 |
| P18 | 0 | 5 | SH04-T01-T05 |
| P29 | 1 | 2 | P10-slash-command-flags T3/T4 |
| P30 | 2 | 2 | P27-provider-error-ux T1 |

All other plans retain every row. The complete dictionaries of all 99 retained tasks
are TOML-semantically identical to base, including files, role, context, verify,
acceptance, and dependencies. Each roll-up has no files, role `quick-reviewer`,
nonempty scheduler-recognized plan dependencies, nonempty task-scoped acceptance,
an ownership check, and no implementation or public-API authority. I ran all 21
ownership-check commands successfully and inspected the named owner records and their
write scopes. Waiting for the whole owner plan is conservative relative to waiting
for only the named task.

I parsed all 120 rows in `plans/_meta/EXECUTION-OWNERSHIP.md` and proved an exact
qualified-ID bijection with the manifests. Every retained row's listed write scope
equals its task `files` list and all 21 roll-up rows say `zero-write`. The separately
recovered `architecture-core-queue` has 24 ready, nonempty-write tasks, none in the
120-row set. The current index boundary is therefore 30 executable plans/144 tasks;
the two superseded self-dev plans remain excluded at 66 tasks.

Across the complete declared 93-manifest corpus, including execution-excluded
historical rows, the write census is 1,658 claims over 586 unique paths, with 161
paths claimed by more than one plan. All 21 CTRL-15 roll-ups contribute zero claims.

## Exact dependency proof

I traversed every task-level `depends_on`, task-level `depends_on_plan`, and
`meta.depends_on_plan` declaration independently at the exact base and cumulative
candidate. I resolved plan IDs against all 93 unique manifest plan IDs and computed
runtime and all-declared strongly connected components separately:

```text
                                      raw refs   unique source->target edges
base task runtime edges                    320                           160
base meta edges                              2                             2
base all-declared edges                    322                           162
r2 task runtime edges                      345                           169
r2 meta edges                                2                             2
r2 all-declared edges                      347                           171
task/meta overlap (base and r2)                                            0
same-plan task references (both)           849
unresolved same-plan references              0
unresolved plan references                   0
runtime cyclic SCCs                          0
all-declared cyclic SCCs                     0
```

The two metadata edges are exactly
`E46-github-workflow-integration -> E01-execution-engine` and
`E48-rate-limit-budgeting -> E01-execution-engine`. The roll-ups add 25 raw
task-runtime references but only nine unique source-plan pairs. Thus the corrected
evidence now states the exact runtime count (169), metadata count (2), and
all-declared count (171) without repeating the rejected ambiguity.

## Independent verification

I added a temporary `roko-cli` integration test using the public
`roko_cli::task_parser::TasksFile`. It found exactly one P08-P34 manifest for every
number, parsed all 29 baseline manifests, asserted the 120 unique IDs and exact six
roll-up groups, checked the runtime-visible zero-write/role/dependency/acceptance
contract, and proved the separate 24-task architecture population:

```text
CARGO_TARGET_DIR=integration/target cargo test -p roko-cli \
  --test ctrl15_r2_review_semantics -- --nocapture
test independently_parses_ctrl15_r2_ownership_population ... ok
test result: ok. 1 passed; 0 failed
```

The temporary source and its named build artifacts were removed. No review target
or repository artifact remains.

Independent Python traversal produced:

```text
baseline_plans=29 baseline_tasks=120 retained=99 rollups=21
architecture_tasks=24
tracked TOMLs parsed=193; parse errors=0
ledger rows=120; exact task/write-scope bijection
ownership verification commands passed=21
```

A disposable exact-tip archive, using the integrated binary whose production source
is unchanged by CTRL-15, produced:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
exit 0; 0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans
exit 0; 0 diagnostics in 6 plans

roko plan validate --strict plans
exit 1; 94 diagnostics in 32 plans; codes: PLAN_031 only

generated plans/INDEX.md SHA-256:
27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
Executable Total: 30 plans, 144 tasks, 0 done, 144 remaining
Excluded: 2 plans, 66 tasks
```

The bounded top-level diagnostics are the existing intended-future-output census;
there are no parse, ownership, dependency, cycle, or index diagnostics. The archive
and logs were removed. The review worktree was clean before this review record.

## Verdict

`ACCEPTED`. The r2 tip fully corrects the prior rejection, the cumulative ownership
mapping and generated records remain valid, and no required candidate correction
remains. The coordinator may merge this exact review chain, rerun the same proof on
the integrated commit, and only then reconcile CTRL-15 canonical status.
