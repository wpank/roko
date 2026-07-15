# CTRL-16 r2 independent review

## Assignment and identity

- Task: `CTRL-16-r2`, independent review.
- Corrected-candidate base: `7f5221e9da762f51d2ab4056f0989b49de76bdea`.
- Content-equivalent r1 replay: `bc11d75a84d1d4d90ad1cf988f41d97346c45c1e`.
- Exact cumulative candidate reviewed:
  `a9ac1f25e99ad80805b5dc266d3b626632291afe`.
- Review branch/worktree: `review/CTRL-16-r2-a9ac1f25` /
  `reviews/CTRL-16-r2-a9ac1f25`.

I reread the complete master, the rejected r1 review, the corrected evidence,
the five-file cumulative diff, the relevant current Rust and shell sources,
and the historical blobs at `7899494d^` and `236686c7`. The cumulative diff
from the assigned base contains exactly the five reserved paths:

```text
plans/_meta/IMPLEMENTATION_ORDER.md
scripts/demo-knowledge-feedback.sh
tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md
tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md
tmp/status-quo/execution-evidence/CTRL-16.md
```

There is no manifest, generated index, ownership ledger, master, task-status,
production Rust, Cargo metadata, or lockfile change in the candidate.

## R1 finding disposition

All three r1 findings are corrected in the submitted bytes:

1. The script no longer invokes either deleted live-demo root. `--live` exits
   2 before repository or state-path setup, and a search of active shell,
   Python, Rust, and TOML command sites finds no remaining plan-run invocation
   of either absent root.
2. The implementation order and evidence now record the exact partial residue
   from `236686c7`: the two exported dry-run data types without CLI/runtime/
   workflow/test wiring, and `format_greeting` without module export, tests, or
   farewell implementation. This matches the current source and the ten/two/
   two historical task blobs; it does not manufacture completion or
   supersession.
3. The literal candidate-SHA template tokens are gone. The evidence uses a
   self-reference-safe immutable-candidate statement, while this independent
   record supplies the exact SHA above.

The historical/current mapping is otherwise accurate: `7899494d` removed
`dry-run-flag`, `live-demo-phase1`, and `live-demo-phase2`; none is a current
manifest or index row. The recovered architecture queue remains separate and
non-empty, and `e2e-smoke` is not represented as an equivalent replacement.

## Release-blocking finding

### High: removed-root state guard is bypassable through an outward symlink

The new simulation guard is not fail-closed for the assigned external-path
case. At `scripts/demo-knowledge-feedback.sh:58-64`, the configured state path
is canonicalized with `os.path.realpath` and only the canonical result is
compared with the two deleted lexical roots. If a deleted root path itself is a
symlink to an external directory, canonicalization removes the forbidden
prefix before the comparison.

I reproduced this only in a fresh `git archive` tree:

```text
external_target=<temporary>/external-target
ln -s "$external_target" <archive>/plans/live-demo-phase1
ROKO_DEMO_STATE_DIR=<archive>/plans/live-demo-phase1/nested \
  bash <archive>/scripts/demo-knowledge-feedback.sh

exit: 0
created: <temporary>/external-target/nested/learn/episodes.jsonl
```

This mutates external state through a configured descendant of the deleted
root. It contradicts both the assignment's descendant/external pre-mutation
gate and the candidate evidence's statement that no script mode can mutate
either removed root or a descendant. The evidence's
`SCRIPT_STATE_GUARD_OK` line exercises only ordinary absent paths and therefore
does not prove the stronger claim.

Required correction: validate both a normalized lexical configured path and
the canonical path against both removed roots before any source-anchor check,
directory creation, or write. Re-run the ordinary exact/descendant cases, an
external symlink into a removed root, and the outward-symlink reproduction
above in disposable archives. The outward case must exit nonzero and leave the
external target empty. Update the implementation evidence to record that
adversarial result; do not broaden the change beyond the already reserved
script/evidence scope.

## Independent gates

All other assigned gates passed:

```text
git diff --check: exit 0
tracked TOML parse: 193/193
current manifests: 32
ready executable: 30 plans / 144 tasks
superseded excluded: 2 plans / 66 tasks
standalone roots: architecture-core=24 / architecture-defi=3 / e2e-smoke=2
Q14: one task / three DeFi source_ref consumers
local Markdown links in cumulative changed docs: 16/16 resolve
architecture manifest SHA-256:
  3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5
plans/INDEX.md SHA-256 before/after disposable generation:
  27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
backlog strict: exit 0 / 0 diagnostics / 55 plans
self-heal strict: exit 0 / 0 diagnostics / 6 plans
top-level strict: exit 1 / 94 warnings / 32 plans
top-level warning census: exactly 94 PLAN_031, no other rule
bash -n: exit 0
--help: exit 0 / no state created
--live: exit 2 / no state created
unknown option: exit 2 / no state created
ordinary removed root: exit 2 / no root created
ordinary removed descendant: exit 2 / no root created
external symlink into removed root: exit 2 / no root created
simulation twice: exit 0/0 / byte-identical output
simulation JSONL: 2 valid records, ep-001 and ep-002
simulation JSONL SHA-256 both runs:
  ed729b8bf452ba56c3b7bdb61090ddf75ef6038d27bba337e7cc4b21df35a01e
outward removed-root symlink: exit 0 / external JSONL created (FAIL)
```

The strict validator and script executions ran only in disposable archive
trees and temporary state directories. Those artifacts were removed. The
review source worktree remained unchanged until this review record was added.

## Verdict

**REJECTED** for exact cumulative candidate
`a9ac1f25e99ad80805b5dc266d3b626632291afe`.

Confidence: high. The r1 corrections, root/count/history reconciliation, links,
strict validation, deterministic simulation, and five-path scope are sound.
The single remaining defect is a reproducible pre-mutation safety failure in
the new executable behavior. Correct the lexical-plus-canonical guard and its
evidence, then submit a fresh immutable candidate for independent review.
