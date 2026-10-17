+++
id = "bug-0d0d37"
kind = "bug"
title = "work/README.md tells workers to export one CARGO_TARGET_DIR for every worktree, so their checks can build against another worktree's crates"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["work"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "4b9f7355e"
source = "bug-be2a53 worker 2026-10-17: the README's step 4 sets up the shared target dir that bug-be2a53 is about"
discovered_from = "work:bug-be2a53"
anchors = ["work/README.md"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53", "bug-df2b12"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'export CARGO_TARGET_DIR=<main checkout>/target' work/README.md"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T04:33:29Z"
commit = "4b9f7355e"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T04:27:06Z"
forced = false
evidence = "4b9f7355e: work/README.md 'For agents' step 4 no longer tells workers to export a shared CARGO_TARGET_DIR=<main checkout>/target. It now says to give each worktree its own target/, explains why sharing is unsafe (cargo names a workspace crate's build alike in every checkout and treats a cached build as fresh for any checkout whose sources are all older than it, even one that never produced it -- bug-be2a53, whose fix strips an inherited shared target dir from roko's own gate environment only, so a worker's own manual cargo commands, which no gate stands between, were still exposed), gives the APFS copy-on-write seed/delete pattern (cp -c -R <recent>/target ./target, with a free-disk check first; gap-7c3e13, still open, tracks doing this safely and automatically for attempt worktrees instead), and routes cargo through the shared machine-load limiter (tmp/hermes-roko-v5/tooling/heavy.sh cargo cargo <command...>) when the round has one set up, keeping the 4-5-worker cap but for machine load rather than cargo's lock on one shared dir. Verify passes: '! grep -qF \"export CARGO_TARGET_DIR=<main checkout>/target\" work/README.md'. Not fixed here (outside this item's anchors, work/README.md only, and .claude/ is local-only per CLAUDE.md): .claude/skills/work-batch/SKILL.md:33 and .claude/skills/work-next/SKILL.md:25 give workers the identical shared-target-dir instruction ('export CARGO_TARGET_DIR=$MAIN/target'); work-sweep/SKILL.md does not mention a target dir."
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

- [x] `work/README.md` no longer tells workers to share one `CARGO_TARGET_DIR`.
