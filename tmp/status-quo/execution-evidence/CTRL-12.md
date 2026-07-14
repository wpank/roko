# CTRL-12 strict-validation evidence

## Assignment

- Control item: Wave 0 `CTRL-12`, confirm both canonical strict validators exit
  zero using the rebuilt integration CLI.
- Evidence base/current archive:
  `d0942fc63ef734017736294843e9112b78e8a656`.
- Binary dependency: CTRL-11 integration-owned binary built from production source
  `128dc950c1659b49b85dc7d052ee0ea0dbc7bb12`.
- Branch/worktree: `agent/CTRL-11-12-build-validation` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-11-12`.
- Reserved write scope: this evidence record and `CTRL-11.md` only.

## Requirement

CTRL-05 repaired the real architecture prerequisite, CTRL-06 corrected
output-versus-prerequisite validation, and CTRL-07 corrected the remaining stale
read paths and producer edges. CTRL-14 subsequently depended on both canonical
strict roots staying green. CTRL-12 must now reproduce those two exact strict gates
from immutable current source with the rebuilt integration binary, while containing
the validator's known generated-index/runtime side effects outside the source tree.

Acceptance requirements:

- immutable current archive, not a worker-mutated source tree;
- exact integration-owned binary provenance from CTRL-11;
- backlog strict exits zero with `0 diagnostics in 55 plans`;
- self-heal strict exits zero with `0 diagnostics in 6 plans`;
- all tracked TOML files in the archive parse;
- source `plans/INDEX.md` remains at the reviewed sealed SHA-256 and source Git
  state remains unchanged before/after validation;
- fixture-only generated output is reported rather than mistaken for source drift.

Explicit non-goals: no manifest, master, index, production, test, lockfile, target,
or integration edit; no warning suppression or placeholder; no CTRL-13 claim about
the combined execution root or dependency identities.

## Immutable validation procedure

The exact current integration commit was exported to a new disposable directory:

```sh
fixture=$(mktemp -d /private/tmp/ctrl11-12-current.XXXXXX)
git archive d0942fc63ef734017736294843e9112b78e8a656 | tar -x -C "$fixture"
(
  cd "$fixture"
  /Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target/debug/roko \
    plan validate --strict tmp/status-quo/backlog/plans --color never
  /Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target/debug/roko \
    plan validate --strict tmp/status-quo/self-heal/plans --color never
)
rm -rf "$fixture"
```

The binary reported immediately before validation:

```text
roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 128dc950c)
SHA-256 726b5841c3e02ce3bc61861a6a5c347b139982de4ffbfe22145cf28e00695096
```

## Results

Both exact strict commands passed:

```text
backlog:   exit 0; 0 diagnostics in 55 plans
self-heal: exit 0; 0 diagnostics in 6 plans
```

Before either validator ran, Python 3 `tomllib` parsed every `*.toml` included by
the immutable tracked archive:

```text
TOML_PARSE_OK files=193 errors=0
```

Source worktree invariants before and after both commands:

```text
plans/INDEX.md before:
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44

plans/INDEX.md after:
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44

git status --porcelain before: empty
git status --porcelain after:  empty
```

The immutable fixture started with the same sealed index. As expected from the
reviewed validator behavior, its disposable `plans/INDEX.md` changed to
`27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8`, and fixture
runtime files appeared under `.roko/` (`INDEX.md`, `prd/INDEX.md`,
`research/INDEX.md`, and `roko.log`; tracked `.roko/GAPS.md` remained present).
The complete fixture was then removed. These are contained disposable effects, not
source drift; neither canonical index bytes nor source status changed.

## Evidence lineage

- CTRL-05 final integration proved the canonical architecture queue and E11
  prerequisite (`4f7e0e34ceba3e021223881d3ac31ddd6a984123`).
- CTRL-06 final accepted validator merge is
  `d4749f9c708eac02f01d0a4d9ee5a3dd84cdcf84`.
- CTRL-07 prerequisite repair and ledger merge are
  `206e9079812b27f738d95f91d1135d0f663c836f` and
  `f0cf7e769306b3217b30de797e2698b1a673326e`.
- CTRL-14 terminal supersession proof is integrated at
  `9b3822b727441ba925ccbb0e92427aa4c17ba9e6`.
- Git ancestry at `d0942fc63` contains each named integration commit.

This record confirms only the two CTRL-12 validation roots. It intentionally does
not infer CTRL-13's combined-root dependency-resolution outcome.

## Hygiene and review readiness

- Candidate scope is limited to `CTRL-11.md` and `CTRL-12.md`.
- No source validator invocation occurred; all generated effects stayed in the
  removed fixture.
- `git diff --check` passes.
- Implementation/evidence commit: recorded at handoff after commit.
- Required independent review: rerun from a fresh archive; verify exact binary
  provenance and SHA-256, 193 TOMLs, strict 0/55 and 0/6, fixture-only index drift,
  sealed source index, clean source state, and explicit CTRL-13 exclusion.

## Integration

- Independent review: pending.
- Integration commit: pending.
- Post-merge reproduction: pending integration-owner action.
- Current evidence state: `IMPLEMENTED_UNREVIEWED`; CTRL-12 is not marked `DONE`
  by this worker.
