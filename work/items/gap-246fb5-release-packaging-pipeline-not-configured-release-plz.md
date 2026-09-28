+++
id = "gap-246fb5"
kind = "gap"
title = "Release & packaging pipeline not configured (release-plz, cargo-dist, git-cliff, GH Releases, Homebrew)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["release"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/32-deployment/packaging-and-distribution.md:378"
discovered_from = "audit:docs/v3/depth/32-deployment/packaging-and-distribution.md:378"
anchors = [".github/workflows", "Cargo.toml [workspace.metadata.dist]"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Packaging docs: release-plz/cargo-dist/git-cliff pipeline designed but not configured; no crates.io publishes; no Homebrew tap; binary distribution via GitHub Releases not configured. Overlaps nous-backlog-v2 NB2-004 (cargo-dist + ghcr).

Imported without verification from:
- `docs/v3/depth/32-deployment/packaging-and-distribution.md:378`
- `docs/v3/depth/32-deployment/native-x86-arm.md:265`

How to verify: Check for release-plz.toml, dist config, cliff.toml and release workflows.
