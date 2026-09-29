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
last_verified_rev = "a17d9d766"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Cargo.toml", "demo/speed-test/Cargo.toml", ".github/workflows/ci.yml", ".github/workflows/msrv.yml", ".github/workflows/deny.yml", ".github/workflows/coverage.yml", ".github/workflows/docs-lint.yml", ".github/workflows/plan-validate.yml", ".github/workflows/tui-parity-dry-run.yml", ".github/workflows/docker-publish.yml", ".github/workflows/deploy-fly.yml", "deny.toml"]
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

- `Cargo.toml` (`[workspace] members`, line ~92): `main` lists `demo/speed-test`, but that crate exists only on the
  working branch (added in `9c6ec420c`). On `main` every cargo command fails while loading the workspace.
- `.github/workflows/ci.yml`: push to main + every PR. `RUSTFLAGS=-D warnings`. Jobs: `Test + Clippy` (toolchain
  1.96.1: `cargo clippy --workspace --no-deps -- -D warnings`, `cargo test --workspace`), `Formatting` (nightly
  `cargo fmt --all --check`), `Layer Check` (`cargo run -p roko-cli -- layer-check`), `Lean CLI + Alloy opt-in`
  (`cargo check -p roko-cli`; `cargo tree` must not show `alloy-provider|network|rpc-client`;
  `cargo check -p roko-cli --features alloy-backend,acp`).
- `.github/workflows/msrv.yml`: push + PR. `Minimum Supported Rust Version` runs `cargo check --workspace` on 1.91.
- `.github/workflows/deny.yml`: push + PR. `Supply-chain audit` runs `cargo deny check` with `deny.toml`
  (advisories `ignore = []`, licence allow-list, `unknown-registry = "deny"`).
- `.github/workflows/coverage.yml`: push + PR. `cargo llvm-cov` over the whole workspace on stable, 45-minute limit.
- `.github/workflows/docs-lint.yml`: path-filtered (README, CLAUDE.md, docs, `tmp/status-quo/**`, the workflow
  files). Runs `tools/docs_integrity/` tests and checker, then grep rules for stale phrases.
- `.github/workflows/plan-validate.yml`: path-filtered (`**/tasks.toml`, `plans/INDEX.md`, some roko-cli sources).
  Builds roko-cli, runs `roko plan validate` on every tracked `tasks.toml` directory and `roko plan index --check`.
- `.github/workflows/docker-publish.yml`, `.github/workflows/deploy-fly.yml`: push to main. Held behind the repository
  variable `ALLOW_PUBLIC_DEPLOY == 'true'` since `b02812036`, so they skip until `bug-7eef96` is fixed.
- `.github/workflows/tui-parity-dry-run.yml`: PRs touching `tmp/tui-parity/**` or `tmp/ux-followup-runner/**`.
  Runs `tmp/tui-parity/run-tui-parity.sh` and `tmp/ux-followup-runner/run-ux-followup.sh`; neither script is
  tracked or on disk.
- `.github/workflows/release.yml`: tags `v*` and manual dispatch only; not run on `main` pushes.

## Current state

- `main` is still `244f564e1` (checked 2026-09-29). The working branch `chore/commit-pending-work-2026-09-25` is 32
  commits ahead with `main` as the merge base, so it can be merged as a fast-forward. It carries
  `demo/speed-test/` (fixes the workspace load) and the deploy guard `b02812036`.
- The workspace-load failure is the only confirmed root cause. The other failure causes were never inspected,
  because the workspace load failed first.
- Known failures that will remain after the merge (static check at HEAD):
  - Docs Lint: the grep rule "Plan-run examples must select --engine runner-v2" matches four correct examples
    (`docs/v2/04-EXECUTION.md:511`, `docs/v2/INTEGRATION-GUIDE.md:82`, `docs/v2/25-DEPLOYMENT.md:117`,
    `docs/v2/CLI-REFERENCE.md:154`). Runner-v2 was deleted and `--engine runner-v2` is now rejected. The checker
    also enforces registry counts against `tmp/status-quo/`, which no longer exists. `bug-f279ea` owns both fixes.
  - tui-parity dry-run: fails on any PR touching those paths, because both scripts are missing.
  - cargo-deny (likely): `demo/speed-test/Cargo.toml` is the only workspace member with no `license` field
    (every other member sets one, most through `license.workspace = true`), and `deny.toml` has no `[licenses.private]` exception.
- MSRV is pinned: `Cargo.toml:103` `rust-version = "1.91"`; `rust-toolchain.toml` pins channel `1.96.1`.
- Unknown: whether `cargo deny check`, the MSRV job, coverage, Plan Validate (index drift; `plans/INDEX.md` is
  modified in the working tree) and the full test suite pass on the merged tree.

## Plan

1. Merge the working branch into `main`. This is a GitHub action: ask Will first, and use a PR, never a direct
   push (see Notes). The PR's `pull_request` runs of CI, MSRV, cargo-deny and coverage show the remaining
   failures before anything reaches `main`.
2. Reproduce each job locally on the merged tree and fix or prune it:
   - `cargo +nightly fmt --all --check`; `cargo clippy --workspace --no-deps -- -D warnings`;
     `RUSTFLAGS="-D warnings" cargo test --workspace`; `cargo run -p roko-cli -- layer-check`;
     `cargo check -p roko-cli` plus the `cargo tree` check; `cargo check -p roko-cli --features alloy-backend,acp`.
   - `cargo +1.91 check --workspace`. `rust-toolchain.toml` pins 1.96.1 and a toolchain file overrides rustup's
     default, so confirm the MSRV job really compiles with 1.91 (look for `1.91` in its log). If not, add
     `RUSTUP_TOOLCHAIN: "1.91"` to the job's `env` or run `cargo +1.91 check --workspace`.
   - `cargo deny check`: add `license.workspace = true` (and `publish = false`) to `demo/speed-test/Cargo.toml`; add
     reviewed advisories to `[advisories] ignore` with a reason, or bump the crates.
   - Plan Validate: `cargo build -p roko-cli`, then `target/debug/roko plan validate <dir>` for each directory from
     `git ls-files | grep 'tasks.toml$'`, then `target/debug/roko plan index --check --workdir .`.
   - Docs Lint: land `bug-f279ea` (drop the runner-v2 rule and the `tmp/status-quo` registry contracts, fix the two
     dead links).
   - Coverage: if it cannot finish in 45 minutes or is flaky, run it only on push to `main` and do not make it required.
3. Delete `.github/workflows/tui-parity-dry-run.yml`, or restore the two scripts to tracked paths. Recommended:
   delete it, because it guards `tmp/` content.
4. Publish the required-checks list, for example as `.github/REQUIRED_CHECKS.md` linked from the README.
   Recommended list: `Test + Clippy`, `Formatting`, `Layer Check`, `Lean CLI + Alloy opt-in`,
   `Minimum Supported Rust Version`, `Supply-chain audit`. Do not require the path-filtered workflows (Docs Lint,
   Plan Validate): a required check that never starts leaves the PR waiting forever. Either keep them optional, or
   make them always run and skip internally when no relevant path changed.
5. Configure branch protection (or a ruleset) on `main` with that list. This needs repository admin rights and
   Will's approval.

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

## Original notes

A local audit (2026-09-26) found workflows under `.github/workflows` failing on main and no published list of required checks; not re-checked here (needs the GitHub API).
Fix: make every workflow green or prune it, publish the required-checks list, and keep MSRV pinned at 1.91.

Verified 2026-09-28 with a read-only `gh run list --branch main`: worse than stated. Every workflow that ran on main's head commit 244f564e1 (2026-09-21) failed: CI, MSRV, cargo-deny, coverage, Docs Lint, Plan Validate, Docker Publish and Deploy to Fly.io. The earlier heads (a560d1027 and the 2026-09-04 commits) show the same failures, except that MSRV passed. There is no required-checks list anywhere in .github/ or docs/. MSRV is pinned (Cargo.toml:103 has rust-version = "1.91"; rust-toolchain.toml sets channel 1.96.1). The causes of the failures were not inspected.

Update 2026-09-28, root cause on main: main's `Cargo.toml` lists `demo/speed-test` as a workspace member, but `demo/speed-test/Cargo.toml` exists only on the working branch (added in 9c6ec420c), so every cargo-based job fails when the workspace loads. Merging the branch fixes that. Before it did, `deploy-fly.yml` and `docker-publish.yml` were guarded by `ALLOW_PUBLIC_DEPLOY` (b02812036), so the first green main cannot deploy or publish the Privy hole (bug-7eef96).

Re-checked 2026-09-29 (static, no GitHub API): main is still 244f564e1 and still lists demo/speed-test as a workspace member without the crate, so the merge that fixes the cargo jobs has not happened. No required-checks list has been published yet.
