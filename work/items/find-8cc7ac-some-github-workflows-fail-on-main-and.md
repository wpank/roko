+++
id = "find-8cc7ac"
kind = "finding"
title = "Some GitHub workflows fail on main and required checks are undefined"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["ci"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = [".github/workflows/ci.yml", ".github/workflows/msrv.yml", ".github/workflows/deny.yml", ".github/workflows/coverage.yml", ".github/workflows/docs-lint.yml", ".github/workflows/plan-validate.yml", ".github/workflows/docker-publish.yml", ".github/workflows/deploy-fly.yml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(gh run list --branch main --limit 10 --json conclusion --jq '[.[]|select(.conclusion==\"failure\")]|length')\" = 0"
+++

A local audit (2026-09-26) found workflows under `.github/workflows` failing on main and no published list of required checks; not re-checked here (needs the GitHub API).
Fix: make every workflow green or prune it, publish the required-checks list, and keep MSRV pinned at 1.91.

Verified 2026-09-28 with a read-only `gh run list --branch main`: worse than stated. Every workflow that ran on main's head commit 244f564e1 (2026-09-21) failed: CI, MSRV, cargo-deny, coverage, Docs Lint, Plan Validate, Docker Publish and Deploy to Fly.io. The earlier heads (a560d1027 and the 2026-09-04 commits) show the same failures, except that MSRV passed. There is no required-checks list anywhere in .github/ or docs/. MSRV is pinned (Cargo.toml:103 has rust-version = "1.91"; rust-toolchain.toml sets channel 1.96.1). The causes of the failures were not inspected.
