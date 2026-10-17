+++
id = "bug-0d0d37"
kind = "bug"
title = "work/README.md tells workers to export one CARGO_TARGET_DIR for every worktree, so their checks can build against another worktree's crates"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["work"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-be2a53 worker 2026-10-17: the README's step 4 sets up the shared target dir that bug-be2a53 is about"
discovered_from = "work:bug-be2a53"
anchors = ["work/README.md"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53", "bug-df2b12"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'export CARGO_TARGET_DIR=<main checkout>/target' work/README.md"
+++

## Problem

`work/README.md`, "For agents", step 4, says: "Rust: `export CARGO_TARGET_DIR=<main checkout>/target` to reuse the
warm build cache". A worker who follows it builds its worktree into the main checkout's target dir. Cargo names a
workspace crate's build alike in every checkout and takes it as fresh whenever the checkout's copy is older. So a
worker's `cargo check` or test can compile against another worker's crates. On 2026-09-29 a worker's check failed
on a type that existed only in another worker's roko-core.

A `roko plan run` started with that variable exported inherits it too. Since bug-be2a53, roko's checks no longer
build there, but the agents' own builds still do (bug-df2b12).

## Why it matters

Workers' own checks pass or fail on the wrong code, and those results decide what gets merged. The work-batch
brief already says the opposite: never export `CARGO_TARGET_DIR`, use your own APFS clone.

## Where

`work/README.md`, "For agents", step 4.

## Current state

The README still gives the advice. bug-be2a53 covers roko's own checks only.

## Plan

Replace the advice with a target dir per worktree, and say why sharing one is wrong. Either take an APFS clone of a
warm target (`cp -c -R <warm>/target ./target`, with enough free disk), or use the worktree's own `target/`.

## Done when

- [ ] `work/README.md` no longer tells workers to share one `CARGO_TARGET_DIR`.
