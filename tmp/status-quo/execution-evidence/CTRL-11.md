# CTRL-11 integrated CLI rebuild evidence

## Assignment

- Control item: Wave 0 `CTRL-11`, rebuild `target/debug/roko` from integrated
  current source.
- Evidence base: `d0942fc63ef734017736294843e9112b78e8a656`.
- Production-source build commit:
  `128dc950c1659b49b85dc7d052ee0ea0dbc7bb12`.
- Branch/worktree: `agent/CTRL-11-12-build-validation` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-11-12`.
- Integration branch:
  `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved write scope: this evidence record and the separate `CTRL-12.md`
  validation record only.

## Requirement

CTRL-06 changed production validator behavior, so strict validation must use an
integration-owned `target/debug/roko` freshly linked after a package-scoped
`roko-cli` cleanup, not an older worker or sealed-root binary. The binary must expose
its exact Git/rustc/target provenance, and every commit after its embedded source
commit must be proved non-production before the binary is treated as current.

Acceptance requirements:

- `cargo clean -p roko-cli` is followed by a successful
  `cargo build -p roko-cli --bin roko` in the integration target.
- the linked binary's build-script record and `--version` agree on source commit,
  rustc, target, package version, and executable;
- the build source is an ancestor of current integration and the intervening diff
  contains no production, test, manifest, lockfile, build-script, or CLI-version
  delta;
- no worker target artifact is substituted for the integration-owned binary.

Explicit non-goals: no master, manifest, index, production, test, lockfile, target,
or integration-branch edit; no CTRL-13 dependency-resolution claim; no remote or
external action.

## Rebuild and provenance

The integration owner performed package-scoped cleanup and rebuild from committed
production source at `128dc950c`:

```sh
cargo clean -p roko-cli
cargo build -p roko-cli --bin roko
```

The fresh build-script output retained in the integration target is:

```text
target/debug/build/roko-cli-726bef00be2ed685/output
mtime: 2026-07-14T12:07:53+0200 (epoch 1784023673)

cargo:rustc-env=ROKO_GIT_HASH=128dc950c
cargo:rustc-env=ROKO_RUSTC_VERSION=rustc 1.96.1 (31fca3adb 2026-06-26)
cargo:rustc-env=ROKO_TARGET=aarch64-apple-darwin
```

The linked executable was produced after that build-script run and after the
`128dc950c` commit timestamp (`2026-07-14T12:06:27+0200`):

```text
path: /Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target/debug/roko
size: 99073384 bytes
mtime: 2026-07-14T12:08:58+0200 (epoch 1784023738)
SHA-256: 726b5841c3e02ce3bc61861a6a5c347b139982de4ffbfe22145cf28e00695096
```

The executable is byte-identical to its freshly linked Cargo dependency artifact
`target/debug/deps/roko-c383a838a54f96b9`, which has the same size and timestamp.
Only one `roko-cli-*/output` build-script record exists in the integration target,
and it names the committed build source above.

The production version path explains why this is direct provenance rather than a
filename or timestamp inference:

- `crates/roko-cli/build.rs` executes `git rev-parse --short HEAD`, captures
  `rustc --version` and `TARGET`, and exports `ROKO_GIT_HASH`,
  `ROKO_RUSTC_VERSION`, and `ROKO_TARGET` with `cargo:rustc-env`.
- `crates/roko-cli/src/main.rs::long_version` combines those compile-time values
  with `CARGO_PKG_VERSION` for the CLI's long version string.
- `cargo metadata --no-deps --format-version 1` identifies package `roko-cli`
  version `0.1.0` and binary target `roko`.

Exact runtime reproduction:

```text
$ target/debug/roko --version
roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 128dc950c)
```

The integration binary existed, was executable, and matched this output twice. Its
SHA-256 was unchanged before and after CTRL-12 validation.

## Current-source proof

Git ancestry proves `128dc950c` is an ancestor of evidence base `d0942fc63`. The
complete intervening path set is exactly:

```text
M tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
A tmp/status-quo/execution-evidence/CTRL-10-REVIEW.md
A tmp/status-quo/execution-evidence/CTRL-10.md
A tmp/status-quo/execution-evidence/README.md
```

That delta is only control/evidence Markdown: 544 inserted and four removed lines.
The following production-scope comparison exits zero with no differences:

```sh
git diff --quiet 128dc950c..d0942fc63 -- \
  Cargo.toml Cargo.lock crates apps demo tests .github Dockerfile docker contracts
```

`crates/roko-cli/build.rs` and `crates/roko-cli/src/main.rs` are byte-identical
between the two commits. Therefore no production-source delta after `128dc950c`
makes the embedded binary stale at `d0942fc63`; rebuilding again solely to embed a
documentation-only commit would not change executable behavior and was unnecessary.

## Hygiene and review readiness

- Candidate changes are evidence-only and do not include `target/` artifacts.
- `git diff --check` passes.
- The worker worktree was clean before these two evidence files were created.
- Implementation/evidence commit: recorded at handoff after commit.
- Required independent review: reproduce the build-script output, linked-artifact
  identity, exact `--version`, binary SHA-256, source ancestry, four-path
  documentation-only delta, and absence of any production delta.

## Integration

- Candidate: `989dc65497d8dd501484d77dd569b9128063d638`.
- Independent review: `ACCEPTED` in `CTRL-11-CTRL-12-REVIEW.md`; review commit
  `b9881905582e63c1dad802bb0b5426d648a77746`.
- Integration merge: `1e478eaf1a6e9b277ad9d890cd6e9805d59a6872`.
- Post-merge reproduction: exact binary provenance remains Git `128dc950c`; the
  intervening integrated delta is evidence/control Markdown only, so production
  source is byte-identical. The binary runs and the integration worktree is clean.
- Final status: `DONE` for the required current-source rebuild proof.
