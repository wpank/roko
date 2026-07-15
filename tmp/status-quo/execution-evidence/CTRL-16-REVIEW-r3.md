# CTRL-16 r3 independent review

## Assignment and identity

- **Verdict:** `REJECTED`
- **Task:** `CTRL-16-r3`, independent review
- **Programme base:** `7f5221e9da762f51d2ab4056f0989b49de76bdea`
- **Content-equivalent r1 replay:**
  `bc11d75a84d1d4d90ad1cf988f41d97346c45c1e`
- **Rejected r2 candidate:**
  `a9ac1f25e99ad80805b5dc266d3b626632291afe`
- **Exact cumulative candidate reviewed:**
  `9044be08f5e732dc376bd0c3c7bd35f55742e5ab`
- **Review branch/worktree:** `review/CTRL-16-r3-9044be08` /
  `reviews/CTRL-16-r3-9044be08`
- **Confidence:** high

I did not implement the candidate. I reread the complete master, both prior rejection
records, the worker evidence, the exact cumulative diff and all five cumulative
candidate paths, the relevant current Rust and shell sources, and the historical
plan/source blobs used by the disposition. The candidate is a direct child of r2.
Its cumulative diff from the assigned programme base contains exactly the five
reserved paths:

```text
plans/_meta/IMPLEMENTATION_ORDER.md
scripts/demo-knowledge-feedback.sh
tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md
tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md
tmp/status-quo/execution-evidence/CTRL-16.md
```

There is no cumulative manifest, generated-index, ownership-ledger, master,
task-status, production Rust, Cargo metadata, or lockfile change. `git diff --check`
and `cargo fmt --all -- --check` both pass.

## Prior finding disposition

The r1 findings remain corrected. No active command invokes either deleted live-demo
root; `--live` fails before state setup. The control documents distinguish the exact
partial dry-run and greeting residue from complete or accepted outcomes, and the
candidate-SHA placeholders are absent.

The r2 outward-symlink finding is corrected for the cases it identified. In fresh
exact-candidate archive trees, all of the following exited 2 before mutation and left
the watched plan/target trees unchanged:

- absolute and relative exact removed roots and descendants;
- lexical `ordinary/../live-demo-phase1/nested` and a nonexistent descendant;
- an external dangling or chained symlink resolving into a removed root;
- a removed-root symlink resolving outward, including a dangling outward target;
- an explicit repository symlink alias resolving outward;
- a `/tmp` to `/private/tmp` repository alias resolving outward; and
- `--live`.

An ordinary external state directory still ran twice with exit 0, created exactly two
JSONL records, and reproduced SHA-256
`ed729b8bf452ba56c3b7bdb61090ddf75ef6038d27bba337e7cc4b21df35a01e`.
Those accepted corrections do not cure the two release blockers below.

## Release-blocking findings

### High — case-insensitive aliases bypass the removed-root guard

The guard compares case-sensitive Python strings. On this review host, the underlying
filesystem is case-insensitive, so a differently cased spelling names the same
physical removed root but is not contained by either string comparison.
`os.path.realpath` preserves the configured spelling when the leaf does not yet
exist, so the canonical-path check does not close the gap.

Reproduction in a fresh `git archive` of the exact candidate:

```text
filesystem probe: case_insensitive
ROKO_DEMO_STATE_DIR=<archive>/plans/LIVE-DEMO-PHASE1/nested \
  /opt/homebrew/bin/bash scripts/demo-knowledge-feedback.sh
exit: 0
forbidden physical file: plans/live-demo-phase1/nested/learn/episodes.jsonl
records: 2
stderr: empty
plans tree digest before: ba65fe76d969ff2b09d53080ae612f0537bb66f0b984a95f1d11db3b8f2030b1
plans tree digest after:  f841801702ac96265cd4b45028e54c650556046907d5061eebc729721a737ccf
```

Expected: every filesystem alias of either removed root or descendant is rejected
before mutation. Actual: the simulation exits successfully and creates state under
the physically identical forbidden root. This contradicts the worker evidence's
claim that lexical, canonical, and alias projections make the guard fail closed.

Required correction: compare according to the containing filesystem's path identity,
including case-insensitive aliases of nonexistent leaf paths. Do not assume
`os.path.normcase` folds case on macOS. Add a disposable-archive regression for
differently cased exact roots and descendants on a case-insensitive volume, while
preserving all accepted r2 symlink and repository-alias cases. Both forbidden cases
must exit nonzero without creating a plan directory or external target.

### High — the current script does not parse with macOS system Bash 3.2

The candidate embeds a quoted Python heredoc inside command substitution at
`scripts/demo-knowledge-feedback.sh:57-124`. Homebrew Bash 5.3 parses it, but the
stock macOS `/bin/bash` 3.2 parser treats single quotes inside the heredoc body as
shell syntax and reaches end-of-file looking for a matching quote.

Independent reproduction:

```text
/opt/homebrew/bin/bash --version: GNU bash 5.3.3
/opt/homebrew/bin/bash -n scripts/demo-knowledge-feedback.sh: exit 0

/bin/bash --version: GNU bash 3.2.57(1)-release
/bin/bash -n scripts/demo-knowledge-feedback.sh: exit 2
scripts/demo-knowledge-feedback.sh: line 203: unexpected EOF while looking for matching `''
scripts/demo-knowledge-feedback.sh: line 213: syntax error: unexpected end of file

ROKO_DEMO_STATE_DIR=<temporary>/state \
  /bin/bash scripts/demo-knowledge-feedback.sh
exit: 2
state created: no
```

The script's shebang is `#!/usr/bin/env bash`, and its usage explicitly instructs
users to run `bash scripts/demo-knowledge-feedback.sh`; neither declares a Bash 4+
requirement. The repository and review host are macOS, where `/bin/bash` is the
normal fallback. This is a candidate-introduced portability regression that makes
the advertised simulation unusable before the guard runs.

Required correction: restructure the Python guard/capture into syntax accepted by
macOS Bash 3.2, or explicitly and consistently establish a newer-Bash requirement in
the executable contract and release environment. The smallest compatible correction
is preferable. Add `/bin/bash -n`, `--help`, `--live`, a normal external simulation,
and at least one rejected removed-root case to the regression matrix.

## Independently reproduced canonical gates

The reconciliation and generated-state claims otherwise reproduce:

```text
tracked TOML parse: 193/193
top-level manifests: 32
ready executable: 30 plans / 144 tasks
superseded excluded: 2 plans / 66 tasks
standalone roots: architecture-core=24 / architecture-defi=3 / e2e-smoke=2
deleted current roots: dry-run=absent / live phase 1=absent / live phase 2=absent
historical parent tasks: dry-run=10 / live phase 1=2 / live phase 2=2 / architecture=24
Q14: one current task / three DeFi source_ref consumers
local Markdown links in cumulative changed docs: 16/16 resolve
architecture current/historical SHA-256:
  3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5
backlog strict: exit 0 / 0 diagnostics / 55 plans
self-heal strict: exit 0 / 0 diagnostics / 6 plans
top-level strict: exit 1 / 94 diagnostics / 32 plans
top-level diagnostic census: exactly 94 PLAN_031 / no other PLAN code
plans/INDEX.md SHA-256 before and after disposable generation:
  27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
```

The strict validator reported Git `7303d2f87`. Validation and behavioral execution
used only fresh exact-candidate archives and temporary state; they were removed after
each run. No candidate path was edited during review.

## Verdict and required next action

**REJECTED** for exact cumulative candidate
`9044be08f5e732dc376bd0c3c7bd35f55742e5ab`.

Do not merge this review as acceptance. Submit a new immutable cumulative candidate
that fixes both the case-insensitive physical-path bypass and the Bash 3.2 parse
regression, updates the worker evidence without weakening the pre-mutation contract,
preserves the accepted r1/r2 corrections and canonical count/history/index results,
reruns the full adversarial matrix and canonical gates, and receives fresh independent
review.
