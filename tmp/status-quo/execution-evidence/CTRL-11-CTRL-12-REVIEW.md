# CTRL-11 / CTRL-12 independent review

## Verdict

**ACCEPTED**

- Candidate: `989dc65497d8dd501484d77dd569b9128063d638`
- Exact candidate parent/base:
  `d0942fc63ef734017736294843e9112b78e8a656`
- Production-source build commit:
  `128dc950c1659b49b85dc7d052ee0ea0dbc7bb12`
- Review branch: `review/CTRL-11-12-989dc654`
- Review worktree:
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-11-12-989dc654`
- Integration branch/head checked:
  `status-quo/integration-status-quo-20260714T073140Z` at the exact candidate base
- Confidence: high

The integration-owned executable is a freshly linked `roko-cli` binary whose
physical build record, linked dependency artifact, version output, timestamps, and
SHA-256 agree on production source `128dc950c`, Rust 1.96.1, target
`aarch64-apple-darwin`, package 0.1.0, and binary `roko`. The complete Git delta from
that embedded commit to the candidate base contains only four control/evidence
Markdown paths and no production, test, manifest, lockfile, build-script, or version
change. Independent immutable-archive validation reproduced all 193 TOML parses,
backlog strict 0/55, self-heal strict 0/6, the expected fixture-only generated
effects, and an unchanged clean source tree. No candidate correction remains.

This verdict accepts only CTRL-11's rebuild/current-production-source proof and
CTRL-12's two named validation roots. It does not accept or infer CTRL-13's combined
execution-root dependency-resolution scope.

## Independence, context, and exact scope

I read the complete 1,164-line master, both candidate records, the execution-evidence
convention and CTRL-10 review, and the relevant CTRL-05, CTRL-06, CTRL-07, and
CTRL-14 implementation/rejection/final-review chains. I inspected the current
validator production tree, build script, version path, Cargo metadata, candidate
diff, prerequisite ancestry, integration target, and current integration worktree.
I did not use a worker target or a worker-created validation artifact. The only
pre-existing executable artifact inspected and run was the integration-owned binary
under review.

The candidate is a direct child of its stated base and its cumulative diff is
exactly:

```text
A tmp/status-quo/execution-evidence/CTRL-11.md
A tmp/status-quo/execution-evidence/CTRL-12.md

2 files changed, 271 insertions(+)
```

There is no production, test, manifest, master, index, lockfile, target, runtime, or
integration-branch change. `git diff --check d0942fc63..989dc6549` passes. The review
worktree and current integration worktree were clean before this review record was
added.

## CTRL-11 binary and current-source proof

The exact integration artifacts independently report:

```text
target/debug/roko
size: 99073384 bytes
mtime: 2026-07-14T12:08:58+0200 (epoch 1784023738)
SHA-256: 726b5841c3e02ce3bc61861a6a5c347b139982de4ffbfe22145cf28e00695096

target/debug/deps/roko-c383a838a54f96b9
size: 99073384 bytes
mtime: 2026-07-14T12:08:58+0200 (epoch 1784023738)
SHA-256: 726b5841c3e02ce3bc61861a6a5c347b139982de4ffbfe22145cf28e00695096
```

Thus the installed executable is byte-identical to the freshly linked Cargo
dependency artifact. There is exactly one `roko-cli-*/output` record in the
integration target. It has mtime `2026-07-14T12:07:53+0200`, after the production
source commit at `2026-07-14T12:06:27+0200`, and contains:

```text
cargo:rustc-env=ROKO_GIT_HASH=128dc950c
cargo:rustc-env=ROKO_RUSTC_VERSION=rustc 1.96.1 (31fca3adb 2026-06-26)
cargo:rustc-env=ROKO_TARGET=aarch64-apple-darwin
```

The linked executable is newer than that build-script run and reports exactly:

```text
roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 128dc950c)
```

Source inspection establishes the provenance path rather than relying on the
filename or timestamps alone: `crates/roko-cli/build.rs` obtains Git with
`git rev-parse --short HEAD`, obtains `rustc --version` and `TARGET`, and exports the
three compile-time variables; `src/main.rs::long_version` combines them with
`CARGO_PKG_VERSION`. Parsed `cargo metadata --no-deps --format-version 1` identifies
package `roko-cli` 0.1.0 and its `roko` binary at `src/main.rs`.

Git ancestry confirms `128dc950c` is an ancestor of `d0942fc63`. Independent
enumeration of the complete intervening path set gives only:

```text
M tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
A tmp/status-quo/execution-evidence/CTRL-10-REVIEW.md
A tmp/status-quo/execution-evidence/CTRL-10.md
A tmp/status-quo/execution-evidence/README.md
```

That is 544 insertions and four deletions, all Markdown. A separate quiet comparison
of `Cargo.toml`, `Cargo.lock`, `crates`, `apps`, `demo`, `tests`, `.github`,
`Dockerfile`, `docker`, and `contracts` found no difference. The build script and
CLI main source are byte-identical at both commits. Therefore the rebuilt binary's
embedded production commit is current for executable behavior at the evidence base;
the later documentation-only commits do not make it stale.

## CTRL-12 immutable validation reproduction

I exported the exact candidate with `git archive` into a fresh
`/private/tmp/ctrl11-12-review.*` directory. Before running the validator, an
independently written Python 3 `tomllib` traversal parsed every TOML in that tracked
archive:

```text
TOML_PARSE files=193 errors=0
```

Using only the integration-owned executable, both exact strict commands passed:

```text
roko plan validate --strict tmp/status-quo/backlog/plans --color never
exit 0; 0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans --color never
exit 0; 0 diagnostics in 6 plans
```

The source worktree was clean before and after, and its tracked index remained:

```text
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

The fixture began with that same hash and, after validation, its disposable
`plans/INDEX.md` had the independently reproduced hash
`27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8`.
The only fixture runtime files were tracked `.roko/GAPS.md` plus generated
`.roko/INDEX.md`, `.roko/prd/INDEX.md`, `.roko/research/INDEX.md`, and
`.roko/roko.log`. The complete fixture was removed; a subsequent scan found zero
remaining `ctrl11-12-review.*` directories. Repeating the integration binary hash
after validation returned the same `726b5841...` SHA-256.

## Evidence lineage and integration compatibility

Every full prerequisite SHA cited by the candidate resolves as a commit and is an
ancestor of the evidence base: CTRL-05 merge
`4f7e0e34ceba3e021223881d3ac31ddd6a984123`, CTRL-06 merge
`d4749f9c708eac02f01d0a4d9ee5a3dd84cdcf84`, CTRL-07 prerequisite and ledger
merges `206e9079812b27f738d95f91d1135d0f663c836f` and
`f0cf7e769306b3217b30de797e2698b1a673326e`, CTRL-14 merge
`9b3822b727441ba925ccbb0e92427aa4c17ba9e6`, and production source
`128dc950c1659b49b85dc7d052ee0ea0dbc7bb12`. This is consistent with the retained
rejection/correction history and with the current production validator behavior.

Current integration is clean at the exact candidate base, so candidate integration
has no newer semantic delta to assess. The following completed without conflict:

```sh
git merge-tree --write-tree \
  d0942fc63ef734017736294843e9112b78e8a656 \
  989dc65497d8dd501484d77dd569b9128063d638
```

The two evidence additions remain compatible with current integration and require
no renewed worker candidate.

## Required next action

The integration owner may merge this exact candidate with this immutable ACCEPTED
review record, prove candidate and review ancestry, rerun the binary hash/version,
193-TOML parse, both isolated strict roots, source index/status hygiene, and exact
scope on the merged integration commit, and only then reconcile CTRL-11 and CTRL-12
in the master. CTRL-13 remains separately pending.
