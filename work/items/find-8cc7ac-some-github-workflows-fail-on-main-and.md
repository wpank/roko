+++
id = "find-8cc7ac"
kind = "finding"
title = "Some GitHub workflows fail on main and required checks are undefined"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "release"
subsystem = ["ci"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "641a030ff"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Cargo.toml", "demo/speed-test/Cargo.toml", ".github/workflows/ci.yml", ".github/workflows/msrv.yml", ".github/workflows/deny.yml", ".github/workflows/coverage.yml", ".github/workflows/docs-lint.yml", ".github/workflows/plan-validate.yml", ".github/workflows/docker-publish.yml", ".github/workflows/deploy-fly.yml", "deny.toml"]
links = { depends_on = ["bug-f279ea"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sha=\"$(gh api repos/{owner}/{repo}/commits/main --jq .sha)\" && test \"$(gh run list --commit \"$sha\" --json conclusion --jq length)\" -gt 0 && test \"$(gh run list --commit \"$sha\" --json conclusion --jq '[.[]|select(.conclusion==\"failure\" or .conclusion==\"startup_failure\" or .conclusion==\"timed_out\")]|length')\" = 0 && test \"$(gh api repos/{owner}/{repo}/branches/main/protection/required_status_checks --jq '.contexts|length')\" -gt 0"
+++

## Problem

Every GitHub Actions workflow that ran on `main`'s head commit `244f564e1` (2026-09-21) failed: CI, MSRV,
cargo-deny, coverage, Docs Lint, Plan Validate, Docker Publish and Deploy to Fly.io (read-only `gh run list
--branch main`, 2026-09-28). Earlier heads (`a560d1027`, the 2026-09-04 commits) show the same failures, except
that MSRV passed. There is also no list of required checks: nothing in `.github/` or `docs/` says which checks
must pass before a merge, and no branch-protection rule is known to exist.

Expected: the head of `main` is green (or a workflow is deliberately skipped), and a short, published list names
the checks that must pass.

## Why it matters

Goal `release`: roko goes public for the Nous application and must not ship with failing CI. A red `main` hides
real regressions, and with no required checks nothing stops a broken merge.
Related: `bug-f279ea` (Docs Lint rules that cannot pass), `bug-7eef96` (Privy JWT admin hole; the reason the deploy
and image workflows are held), `gap-2122bd` (release/Docker builds skip the portal export), `bug-09690f` (README).

## Where

- `Cargo.toml` (`[workspace] members`, line ~92): lists `demo/speed-test`. Until PR #79 (`0494f2ba1`) that crate
  was missing on `main`, so every cargo command failed while loading the workspace. That is fixed on `main`.
- `.github/workflows/ci.yml`: push to main + every PR. `RUSTFLAGS=-D warnings`. Jobs: `Test + Clippy` (toolchain
  1.96.1: `cargo clippy --workspace --no-deps -- -D warnings`, `cargo test --workspace`), `Formatting`
  (`cargo +nightly fmt --all --check`), `Layer Check` (`cargo run -p roko-cli -- layer-check`, 30-minute limit),
  `Lean CLI + Alloy opt-in` (`cargo check -p roko-cli`; `cargo tree` must not show `alloy-provider|network|rpc-client`;
  `cargo check -p roko-cli --features alloy-backend,acp`), `Portal (types, tests, export)` (npm), and
  `Portal browser check (fake agent)` (`continue-on-error`, so it cannot fail the run).
- `.github/workflows/msrv.yml`: push + PR. `Minimum Supported Rust Version` runs `cargo +1.91 check --workspace`.
- `.github/workflows/deny.yml`: push + PR. `Supply-chain audit` runs `cargo deny check` with `deny.toml`
  (advisories `ignore = []`, licence allow-list, `unknown-registry = "deny"`).
- `.github/workflows/coverage.yml`: push + PR. `cargo llvm-cov` over the default members (roko-cli and the two MCP
  servers) on stable (`rustup override set stable` bypasses `rust-toolchain.toml`), 45-minute limit.
- `.github/workflows/docs-lint.yml`: path-filtered (README, CLAUDE.md, docs, the workflow files). Runs
  `tools/docs_integrity/` tests and checker, then grep rules for stale phrases.
- `.github/workflows/plan-validate.yml`: path-filtered (`**/tasks.toml`, `plans/INDEX.md`, some roko-cli sources).
  Builds roko-cli, runs `roko plan validate` on every tracked `tasks.toml` directory outside `fixtures/` and
  `plans/archive/`, then `roko plan index --check`.
- `.github/workflows/docker-publish.yml`, `.github/workflows/deploy-fly.yml`: push to main. Held behind the repository
  variable `ALLOW_PUBLIC_DEPLOY == 'true'` since `b02812036`, so they skip until `bug-7eef96` is fixed.
- `.github/workflows/tui-parity-dry-run.yml`: deleted in `641a030ff`. It ran `tmp/tui-parity/run-tui-parity.sh` and
  `tmp/ux-followup-runner/run-ux-followup.sh` on PRs touching `tmp/`; neither script was ever committed.
- `.github/workflows/release.yml`: tags `v*` and manual dispatch only; not run on `main` pushes.

## Current state

Re-checked 2026-09-29 by wk-ci: read-only `gh` queries of the runs on `main`, plus a static check of the working
branch at `2a9312985`. "Fixed" marks what branch `work/bug-f279ea` changes (`a52b528c9`, `641a030ff`).

- `main` is now `de94e4402` (PR #80, 2026-09-29), after PR #79 (`0494f2ba1`), so the workspace-load failure is gone.
  Both PRs merged while CI was red: `main` has no branch protection and no rulesets
  (`gh api repos/wpank/roko/branches/main/protection` returns 404; `gh api repos/wpank/roko/rulesets` returns `[]`).
- Runs on `de94e4402`: MSRV passed; Docker Publish and Deploy to Fly.io skipped (`ALLOW_PUBLIC_DEPLOY` is unset);
  CI, cargo-deny, coverage and Plan Validate failed. Docs Lint did not run there (path filter); it failed on
  `0494f2ba1`. The workflow files on `main` match `2a9312985` apart from the Docs Lint fixes.
- Per job:
  - Docs Lint: fixed by bug-f279ea (`a52b528c9`); bug-09690f had already removed the runner-v2 rule. The checker,
    its unit tests and the grep step pass locally.
  - cargo-deny: `licenses` fixed (`641a030ff`): `speed-test` had no `license` field. `advisories`: 8 errors
    (remaining 4).
  - CI `Test + Clippy`: two Linux-only clippy errors (remaining 1). Clippy fails first, so `cargo test` did not run
    in any of the runs checked.
  - CI `Lean CLI + Alloy opt-in` and `Layer Check`: `cargo check` / `cargo build -p roko-cli` stop on the Linux-only
    `sandbox` warning under `RUSTFLAGS=-D warnings` (remaining 1). Layer Check was also cancelled at its 15-minute
    limit on `a560d1027` and `de94e4402` (cold build): the limit is now 30 minutes (fixed).
  - CI `Portal browser check (fake agent)` fails the same way, but it is `continue-on-error`, so the run does not
    fail on it. `Formatting` and `Portal (types, tests, export)` pass.
  - coverage: builds without `-D warnings` and runs roko-cli's lib tests; 6 of 3,050 fail on Linux (remaining 3).
    It stopped after 20 minutes at that first failing test binary; a full run takes longer, and the limit is
    45 minutes.
  - Plan Validate: discovery took all 153 tracked `tasks.toml` directories, including test fixtures that are invalid
    on purpose (8 fail) and `plans/archive/` (15 fail). Both are now skipped (fixed). 4 active plans still fail
    (remaining 5). CI stops at the first failure, so these counts come from running every directory locally with a
    `roko` binary built at `33e107da1`, whose `plan_validate.rs` and `commands/plan.rs` match `2a9312985`.
    `plan index --check` is unchecked, because that binary predates the `index.rs` changes.
  - MSRV passed on the wrong toolchain. The job installs 1.91, but `rust-toolchain.toml` overrides rustup's default,
    so it compiled with 1.96.1 (its log: "the toolchain '1.96.1-x86_64-unknown-linux-gnu' is currently in use
    (overridden by '…/rust-toolchain.toml')"). No MSRV run has used 1.91 since `rust-toolchain.toml` was added
    (`e44af1709`, 2026-05-04, pinning `stable`; 1.96.1 since `72e0a76b8`). Now `cargo +1.91 check --workspace`
    (fixed; it may turn red). `cargo metadata` shows no resolved dependency with a `rust-version` above 1.91;
    whether roko's own code builds on 1.91 is unknown.
  - Formatting: the same override made it check with rustfmt 1.96.1, not the nightly it installs and CLAUDE.md
    names. Now `cargo +nightly fmt --all --check` (fixed; it may turn red).
  - tui-parity dry-run: deleted (fixed).

### Remaining failures

Each needs code, a dependency change or a decision. The Rust files named here are identical on `main` and
`2a9312985`.

1. Linux-only clippy errors under `-D warnings`:
   - `crates/roko-cli/src/runner/extension_loader.rs:280`: `unused variable: sandbox`. Only the
     `#[cfg(target_os = "macos")]` arm uses it. It breaks Test + Clippy, Lean CLI, Layer Check and the portal
     browser job.
   - `crates/roko-cli/src/runner/agent_stream.rs:236`: `clippy::large_enum_variant` on `AgentWait`. `AgentHandle`
     holds a `tokio::process::Child`, which makes the variant 240 bytes on Linux. The sibling `AgentTermination`
     already allows this lint.
2. `crates/roko-cli/src/runner/gate_dispatch.rs:2305`: `unused import: super::super::gate_report::*` in the test
   module. It will fail CI's `cargo test` under `RUSTFLAGS=-D warnings`. CI's clippy has no `--all-targets`, so it
   never lints test code.
3. Linux test failures in `-p roko-cli --lib` (coverage run 36575337223), not yet diagnosed:
   `doctor::tests::doctor_skips_mcp_allowlist_when_no_config` and
   `doctor::tests::run_doctor_passes_bootstrapped_workspace_without_serve_probe` (`assertion failed: report.healthy`);
   `model_selection::tests::{cascade_router_is_consulted_when_no_explicit_selection_exists,
   config_default_is_used_when_cascade_is_absent, display_line_and_json_are_canonical,
   role_with_empty_model_falls_through}` (the model source resolves to `BuiltInDefault` where the tests expect
   `ProjectDefault` or `CascadeRouter`). They pass locally on macOS, so the cause depends on the environment.
4. cargo-deny advisories (run 36575337065):
   - Lockfile bump only: RUSTSEC-2026-0285, `rustls` 0.23.43, fixed in 0.23.45 (`cargo update -p rustls`).
   - A dependency upgrade: RUSTSEC-2026-0002 and RUSTSEC-2026-0253, `lru` 0.12.5 via `ratatui` 0.29 and `mirage-rs`
     (fixed in 0.16.3 and 0.18.2, both semver-incompatible); RUSTSEC-2025-0119, `number_prefix` is unmaintained,
     via `indicatif` 0.17 in roko-cli; RUSTSEC-2025-0134, `rustls-pemfile` is unmaintained, a direct dependency
     of roko-serve.
   - No fixed version: RUSTSEC-2024-0436 `paste` and RUSTSEC-2026-0173 `proc-macro-error2` (unmaintained, via
     alloy); RUSTSEC-2023-0071 `rsa` (Marvin timing attack, via `jsonwebtoken` 10 and `octocrab`). Each needs a
     reviewed `[advisories] ignore` entry with its reason, or the dependency removed.
5. Plan Validate: 4 active plans whose context anchors went stale after they ran.
   - `plans/portal-plan-execution`: line ranges past the end of `apps/portal/src/stores/dashboard.ts`; symbols
     `StatusLED` and `TaskEditorRow` not found.
   - `plans/portal-programme/08d-portal-legibility` and `08f-final-polish`: line ranges past the end of
     `RunBand.tsx` and `StreamPane.tsx`.
   - `plans/workspace-doctor-improvements`: symbol `run_checks` not found.

   There are three options: re-anchor them, archive the finished ones, or have `plan validate` skip context checks
   for tasks that are already done. Every executed plan will rot the same way, so only the last option lasts. The
   portal-programme plans back the whitepaper's portal-build numbers, so editing them changes cited evidence.

## Plan

1. Merge `work/bug-f279ea` into the working branch, and later into `main` through a PR (ask Will first; see Notes).
2. Fix remaining items 1 and 2 (small Rust changes). Check them with
   `cargo clippy --workspace --no-deps -- -D warnings` and `RUSTFLAGS="-D warnings" cargo test --workspace` on
   Linux (a PR run or a Linux container), because neither error reproduces on macOS.
3. Diagnose remaining item 3 on Linux.
4. Advisories (remaining item 4): run `cargo update -p rustls`. For each other crate, upgrade it or add a reviewed
   `[advisories] ignore` entry with its reason.
5. Decide remaining item 5 and apply the decision.
6. After the merge, check that MSRV (now really 1.91) and Formatting (now really nightly) stay green. If 1.91 fails,
   fix the code; do not bump `rust-version`.
7. Required checks: publish the list, then configure branch protection or a ruleset (Will's decision; the
   recommendation is in Notes). Enable it only once `main` is green, or every PR will block.

## Done when

- The workflow runs for `main`'s head commit have no `failure` conclusions. Deploy and image publishing still
  skip while `ALLOW_PUBLIC_DEPLOY` is unset.
- `tui-parity-dry-run.yml` is removed or its scripts exist in git.
- A tracked document lists the required checks, and `main`'s branch protection requires the same checks.
- Verify (needs `gh` authenticated for the repository):
  `sha="$(gh api repos/{owner}/{repo}/commits/main --jq .sha)" && test "$(gh run list --commit "$sha" --json conclusion --jq length)" -gt 0 && test "$(gh run list --commit "$sha" --json conclusion --jq '[.[]|select(.conclusion=="failure" or .conclusion=="startup_failure" or .conclusion=="timed_out")]|length')" = 0 && test "$(gh api repos/{owner}/{repo}/branches/main/protection/required_status_checks --jq '.contexts|length')" -gt 0`

## Notes

- Git rules from Will's global instructions: never push directly to `main`; ask before every commit, push, PR or
  merge. Changing branch protection is a public-facing GitHub action, so ask for that too.
- Do not set `ALLOW_PUBLIC_DEPLOY=true` or remove the guards in `deploy-fly.yml` / `docker-publish.yml` until
  `bug-7eef96` is closed. The first green `main` must not deploy or publish the Privy hole.
- Keep `rust-version = "1.91"` (decided). Do not bump it to make MSRV pass: fix the code, or ask.
- Do not touch `.gitignore` without asking. Removing tracked `tmp/` files is open decision D-03.
- The old verify command counted failures in the last 20 runs on `main`, so old failures kept it red after a fix.
  The command above checks only the head commit.
- Parallel work: the workflow edits are independent of Rust work, but step 1 (the merge) touches everything.
  Do this item after the other agents' branches have been merged.
- 2026-09-29 (wk-ci): branch `work/bug-f279ea` fixes the failures that need no code change (`641a030ff`) and
  closes bug-f279ea (`a52b528c9`). verify=partial: the verify command needs a green `main` and branch protection,
  and a branch can provide neither. The coordinator may file remaining items 1-2 (Rust lints), 3 (Linux test
  failures), 4 (advisories) and 5 (plan anchors, or the validator policy) as separate items.
- Required checks, recommended but not applied (Will decides):
  - Require the jobs that always run: `Test + Clippy`, `Formatting`, `Layer Check`, `Lean CLI + Alloy opt-in`,
    `Portal (types, tests, export)`, `Minimum Supported Rust Version` and `Supply-chain audit`.
  - Do not require these:
    - Docs Lint and Plan Validate: they are path-filtered, and a required check that never starts blocks the PR
      forever.
    - coverage: it takes about 20 minutes and is informational.
    - `Portal browser check (fake agent)`: it is `continue-on-error`.
    - Docker Publish and Deploy to Fly.io: they run on push only.
  - `Supply-chain audit` fails whenever RustSec publishes a new advisory, whatever the PR changes. To keep that from
    blocking unrelated merges, run `cargo deny check advisories` as a separate job that is not required (or on a
    schedule), and require only `cargo deny check bans licenses sources`.
- 2026-09-29 (wk-ci2), on `work/find-8cc7ac` from `407ce30d5`. Causes were confirmed from the logs of CI run
  36575337397, coverage run 36575337223 and cargo-deny run 36575337065 on `de94e4402`.
  - Fixed, remaining 1: `extension_loader.rs`: the firejail arm now reads `let _ = sandbox;`, with a comment that
    firejail gets no per-path write rules. `agent_stream.rs`: `AgentWait` allows `clippy::large_enum_variant`, as
    `AgentTermination` does. Nothing in the workspace depends on `roko-cli`, so every other crate already passed
    Linux clippy in that run.
  - Fixed, remaining 2: removed `use super::super::gate_report::*;`. The parent module imports all five
    `pub(super)` items, and `use super::*` passes them to the tests.
  - Fixed, remaining 3 (model_selection, 4 tests): `config_with_claude_models()` resolves `claude` on `PATH`, and
    runners don't have it, so the cascade-router and project-default steps fell through to `BuiltInDefault`. The
    tests now use `config_with_available_claude_models()`.
  - Not fixed, remaining 3 (doctor, 2 tests; needs a decision). The cause is reproduced, not read from the log: the
    prebuilt `roko doctor --json` ran in a fresh workspace with no `claude` on `PATH`, no API keys and an empty
    `HOME`. Exactly two checks fail: `shared_credentials_none` (roko-execution `check_credentials`, which accepts
    only API-key env vars or `claude` on `PATH` and ignores the config) and `provider_usable` (`auth_detect`, which
    runs `claude --version`). With a stub `claude` on `PATH`, doctor is healthy. Both tests assert
    `report.healthy`, so they need a host with a provider. The options are to inject the credential and auth
    probes into `run_doctor`, or to have the tests allow only these two host-credential failures.
  - Fixed, remaining 4, 8 advisories:
    - `rustls` 0.23.45 via `cargo update -p rustls --precise 0.23.45`. Plain `-p rustls` locks nothing, because
      0.23.45 needs `rustls-webpki` 0.103.15 and `aws-lc-rs` 1.18.1 / `aws-lc-sys` 0.45.0 (all rust-version 1.71).
    - `indicatif` 0.18 (rust-version 1.85) drops `number_prefix`.
    - `trigger_tls.rs` parses PEM with `rustls::pki_types::pem::PemObject`, and `rustls-pemfile` is removed.
    - `deny.toml` ignores the 5 advisories that have no fix, each with its reason: `lru` (2; ratatui 0.29 and
      mirage-rs call neither `iter_mut()`, and their keys have no `Drop` that can panic), `paste`,
      `proc-macro-error2`, and `rsa` (octocrab is built with `personal_token()` only).
    - cargo-deny 0.20.2, the version CI installs: `cargo deny check` passes (advisories, bans, licenses, sources).
  - Lockfile: every `cargo update` re-resolves the `tempfile` edge `getrandom >=0.3, <0.5` from 0.4.3 to 0.3.4.
    The BASE edge was restored by hand, so the diff holds only the intended packages. `--locked` builds accept it.
  - Local checks (macOS): `cargo +nightly fmt --all --check` is clean.
    `cargo clippy -p roko-cli -p roko-serve --no-deps --locked -- -D warnings` and
    `cargo check -p roko-cli --locked` pass. These don't compile the Linux-only arms or the test code.
  - Only CI on Linux can show: the rest of `cargo test --workspace` (no run has reached it), the Lean CLI
    `--features alloy-backend,acp` step, Layer Check, and MSRV on a real 1.91.
  - Still open: the doctor tests, 5 (plan anchors; Will decides) and branch protection (Will decides).
  - Implemented on `work/find-8cc7ac` at `6e98eb87b`; cargo verification deferred to the batch check.

## Original notes

A local audit (2026-09-26) found workflows under `.github/workflows` failing on main and no published list of required checks; not re-checked here (needs the GitHub API).
Fix: make every workflow green or prune it, publish the required-checks list, and keep MSRV pinned at 1.91.

Verified 2026-09-28 with a read-only `gh run list --branch main`: worse than stated. Every workflow that ran on main's head commit 244f564e1 (2026-09-21) failed: CI, MSRV, cargo-deny, coverage, Docs Lint, Plan Validate, Docker Publish and Deploy to Fly.io. The earlier heads (a560d1027 and the 2026-09-04 commits) show the same failures, except that MSRV passed. There is no required-checks list anywhere in .github/ or docs/. MSRV is pinned (Cargo.toml:103 has rust-version = "1.91"; rust-toolchain.toml sets channel 1.96.1). The causes of the failures were not inspected.

Update 2026-09-28, root cause on main: main's `Cargo.toml` lists `demo/speed-test` as a workspace member, but `demo/speed-test/Cargo.toml` exists only on the working branch (added in 9c6ec420c), so every cargo-based job fails when the workspace loads. Merging the branch fixes that. Before it did, `deploy-fly.yml` and `docker-publish.yml` were guarded by `ALLOW_PUBLIC_DEPLOY` (b02812036), so the first green main cannot deploy or publish the Privy hole (bug-7eef96).

Re-checked 2026-09-29 (static, no GitHub API): main is still 244f564e1 and still lists demo/speed-test as a workspace member without the crate, so the merge that fixes the cargo jobs has not happened. No required-checks list has been published yet.
