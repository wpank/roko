+++
id = "gap-8636f8"
kind = "gap"
title = "No roko release has ever been published: no tagged build, no checksums a client can verify, no Homebrew tap, so people without Rust cannot get roko"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
hold = "Publishing (pushing main and a v* tag) needs Will's go"
subsystem = ["release"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "roko-autostart round research, 2026-10-19 (roko 3b34aed92)"
anchors = [".github/workflows/release.yml"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

No roko release has ever been published. Someone without a Rust toolchain cannot get roko, and the Hermes plugin has nothing to download. In detail:

- `.github/workflows/release.yml` builds four targets and creates a GitHub Release on a `v*` tag, but no tag exists and local main has not been pushed.
- There is no checksum file a client can verify.
- There is no Homebrew tap or formula.
- cargo-dist settings exist in `Cargo.toml` (~109-120) but are unused.

## Why it matters

This is the parent spec's phase 3. Will's answer on 2026-10-19 was that roko should work for anyone who installs the plugin.

## Where

`.github/workflows/release.yml` and `Cargo.toml` (`[workspace.metadata.dist]`, `[profile.release]`).

## Plan

1. Make the release artifacts predictable: name them `roko-<version>-<target>.tar.gz` and publish `SHA256SUMS`.
2. Add a `roko-latest.json` manifest (version, per-target URL, sha256) that the plugin reads.
3. Add a Homebrew tap formula.
4. Rehearse locally: build the artifacts and serve them from a local HTTP server that the plugin's download test uses.
5. Publish (a push and a tag) only with Will's go.

## Done when

- The release workflow produces the named artifacts and checksums, proven on a local rehearsal.
- The plugin downloads and verifies one from a local server.
- Publishing is done when Will says so.

## Notes

Held: publishing needs Will's decision (main has never been pushed).
