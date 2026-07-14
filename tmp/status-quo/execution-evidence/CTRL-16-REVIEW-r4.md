# CTRL-16 r4 independent review

## Verdict and identity

- **Verdict:** `ACCEPTED`
- **Task:** `CTRL-16-r4`
- **Programme base:**
  `7f5221e9da762f51d2ab4056f0989b49de76bdea`
- **Exact cumulative candidate:**
  `82598e68ef2d3837e26594842b4a492325c7d927`
- **Exact candidate parent:**
  `9044be08f5e732dc376bd0c3c7bd35f55742e5ab`
- **Review branch/worktree:** `review/CTRL-16-r4-82598e68` /
  `reviews/CTRL-16-r4-82598e68`
- **Confidence:** high

I did not implement this candidate. I reread the complete master, all three
prior rejection records, the cumulative worker evidence, every cumulative
changed line, the current script and relevant Rust call sites, the current and
historical manifests, the implementation-order/history Git graph, and every
tracked reference to the four affected plan roots. I independently reconstructed
the gates instead of relying on the worker's recorded output.

The candidate is a direct child of the rejected r3 candidate. Its cumulative
diff from the assigned programme base contains exactly these five paths:

```text
plans/_meta/IMPLEMENTATION_ORDER.md
scripts/demo-knowledge-feedback.sh
tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md
tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md
tmp/status-quo/execution-evidence/CTRL-16.md
```

There is no cumulative task manifest, generated index, execution-ownership
ledger, master, canonical task status, production Rust, Cargo metadata, or
lockfile change. The r4 commit itself changes only the script and cumulative
worker evidence. `git diff --check` and `cargo fmt --all -- --check` pass.

## Prior rejection disposition and changed-line review

All r1-r3 findings are closed in the submitted bytes:

1. No active command passes `dry-run-flag`, `live-demo-phase1`, or
   `live-demo-phase2` to `roko plan run`. `--live` and unknown arguments exit 2
   before repository discovery, Python execution, source checks, state creation,
   or plan execution. Default mode is explicitly simulated.
2. The current-control notices and implementation order identify the exact
   partial residue from `236686c7`: exported `DryRunGate`/`DryRunPreview` types
   without top-level `roko run` or `WorkflowRunConfig` wiring and without the
   proposal's builder/tests; and `format_greeting` without module export/test,
   with no farewell function/test. They do not claim completion, cancellation,
   or a task-level supersession.
3. No candidate-SHA placeholder remains. The worker record uses the correct
   self-reference-safe wording and this review supplies the immutable SHA.
4. The lexical and canonical checks reject an inward alias to a removed root
   and a removed-root symlink resolving outward. Prefix-by-prefix repository
   identity projection preserves the lexical forbidden suffix through a direct
   repository alias, a chained alias, and `/tmp` to `/private/tmp` spelling.
5. The r4 `path_key()` applies `normpath().casefold()` consistently to the
   configured lexical path, canonical path, removed roots, and repository-prefix
   identity. `commonpath()` still enforces component boundaries, so a mere
   string prefix such as `live-demo-phase10` is not rejected.
6. The Python guard is now one single-quoted `python3 -c` argument. Its body has
   no embedded single quote, and the configured path/repository root are passed
   as separate argv values after the closing quote. Shell interpolation cannot
   alter the Python program. Both Bash parsers and both runtime paths accept it.

I specifically challenged casefold under- and over-rejection. Under-rejection
did not reproduce on the case-insensitive APFS review volume: mixed-case absent
roots, descendants, a pre-existing physical lowercase fixture reached through
uppercase spelling, mixed-case inward chains, and a mixed-case repository alias
over an outward root all reject before mutation. Unconditional casefold is
conservative on a case-sensitive volume: a distinct directory whose complete
absolute spelling differs from the forbidden repository root only by case could
also reject. That boundary is disclosed by the implementation, is limited to a
casefold-equivalent forbidden absolute prefix, and is consistent with this
task's fail-closed contract. It does not affect an independently located
external state directory, which succeeds under both shells. No weakening or
filesystem-dependent fallback is required.

The all-reference scan found only the intended current operational references:
the implementation-order disposition and the script's historical error/guard
strings. Other hits are preserved baseline audits, archived logs/PR text, PRD
inventory, or future task design material; none is a runnable manifest, index
row, active implementation-order instruction, or command invocation of a
removed root. The future plan-to-graph design explicitly tolerates missing
fixtures and is not the current executable queue. This change correctly leaves
historical bodies intact behind their current-control notices.

## Independent Bash and path-safety reproduction

I used a reviewer-authored temporary repo-shaped fixture containing the exact
candidate script, its two required current source anchors, and a fresh `plans/`
directory. Every adversarial case received a fresh fixture. The harness hashed
the complete watched plan/target/alias trees before and after each invocation,
including file bytes, directories, and symlink targets, and deleted all temporary
fixtures afterward.

The two independently exercised shells were:

```text
/bin/bash:             GNU bash 3.2.57(1)-release (arm64-apple-darwin25)
/opt/homebrew/bin/bash GNU bash 5.3.3(1)-release (aarch64-apple-darwin25.0.0)
filesystem probe:      case_insensitive
```

Both shells passed `-n`. Under both shells, `--help` exited 0 without state,
while `--live` and an unknown option exited 2 without state or repository
mutation. I then ran all 15 guard cases under both shells:

```text
absolute exact removed root
relative nonexistent descendant
lexical ordinary/../removed-root descendant
external inward symlink to a nonexistent removed-root descendant
chained inward symlinks
removed-root outward symlink to an existing external target
removed-root outward symlink to a dangling external target
repository alias plus outward removed root
/tmp to /private/tmp repository alias plus outward removed root
mixed-case exact removed root
mixed-case nonexistent descendant
mixed-case route to a pre-existing physical lowercase root fixture
mixed-case chained inward aliases
canonical inward alias to an existing physical removed-root fixture
mixed-case repository alias plus outward removed root
```

Result: all 30 invocations exited 2 with the expected refusal diagnostic and
with byte-identical watched-tree digests. No rejected case created a directory,
JSONL file, external-target byte, or altered symlink.

For the success path, both shells ran the normal external state case twice.
All four invocations exited 0, produced exactly records `ep-001` and `ep-002`,
and reproduced this byte-identical JSONL digest:

```text
ed729b8bf452ba56c3b7bdb61090ddf75ef6038d27bba337e7cc4b21df35a01e
```

PATH sentinels for `cargo`, `curl`, `wget`, `roko`, `git`, `npm`, and `claude`
recorded zero calls. The plan-tree digest remained unchanged. Thus the accepted
simulation is deterministic and performs no network, build, model, or plan
execution.

## Independent corpus, history, and residue proof

A fresh Python `tomllib` census and direct Git/source inspection reproduced:

```text
tracked TOML parsing:             193/193, zero errors
top-level manifests:              32
ready executable population:      30 plans / 144 tasks
superseded excluded population:   2 plans / 66 tasks
runnable standalone roots:        24 / 3 / 2 tasks
deleted current roots:            3 absent; 0 index rows
Q14 anchors:                      1
DeFi Q14 source_ref consumers:    3
historical parent task counts:    10 / 2 / 2 / 24
changed-control Markdown links:   16/16 resolve
cumulative candidate paths:       exactly 5
```

The current recovered architecture manifest is byte-identical to
`7899494d^:plans/architecture-core-queue/tasks.toml` and has SHA-256:

```text
3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5
```

The residue inspection found exactly two dry-run data structs and the
`pub mod dry_run` export, but no top-level `roko run` dry-run field, runtime
config field, historical builder, or named proposal tests. It found the one
greeting function but no `roko-std` module export, greeting test, farewell
function, or farewell test. Existing unrelated dry-run modes such as
`roko plan run --dry-run` do not satisfy or contradict that precise historical
top-level workflow proposal.

## Independent strict validation and index proof

I exported the exact candidate with `git archive` into one disposable tree and
ran the integrated validator binary reporting Git `7303d2f87`. The disposable
tree prevents validator side effects from touching the candidate:

```text
backlog strict:   exit 0; 0 diagnostics in 55 plans
self-heal strict: exit 0; 0 diagnostics in 6 plans
top-level strict: exit 1; 94 diagnostics in 32 plans
top-level codes:  94 PLAN_031; no other PLAN code
```

The generated and tracked indexes were byte-identical. Their SHA-256 is:

```text
27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
```

The bounded top-level `PLAN_031` population is unchanged intended-future-file
evidence, not a parse, plan-root, dependency-ID, or index failure. The source
index stayed unchanged. The disposable archive, generated state, test fixtures,
logs, and reviewer harnesses were removed; the review worktree was clean before
adding this record.

## Verdict and required next action

**ACCEPTED** for exact cumulative candidate
`82598e68ef2d3837e26594842b4a492325c7d927`.

There is no required implementation correction. The integration owner may merge
this exact candidate plus review record, rerun the focused Bash/path matrix and
canonical validation/index gates on the integrated head, and only then reconcile
CTRL-16 status.
