+++
id = "bug-7973a6"
kind = "bug"
title = "Whole-plan and regression checks run under sh while task verify steps run under bash -o pipefail, so a copied step behaves differently"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify", "crates/roko-cli/src/graph_execution/delivery.rs"]
links = { depends_on = [], blocks = [], related = ["bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_verify_runs_steps_like_task_verify' crates/roko-cli/ && cargo test -p roko-cli plan_verify_runs_steps_like_task_verify"
+++

## Problem

Task verify steps run as `bash -o pipefail -c` (`graph_task_dispatch/verification.rs`), but a plan's whole-plan
check (`[meta] verify`, `plan_verify.rs::run_plan_verify`) and the delivery's post-merge regression check run as
`sh -c`. A step copied from a task into `[meta] verify` that uses bash syntax (`[[ … ]]`, `<<<`, as several demos'
task steps do) works on macOS, where `sh` is bash, and fails on Linux, where `sh` is dash; nothing warns.

## Why it matters

The same command means different things in two places of one plan, and the difference shows up only on another OS.

## Where

`crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify`, `crates/roko-cli/src/graph_execution/delivery.rs`.

## Plan

Run whole-plan and regression steps the way task steps run (`bash -o pipefail -c`), or have `plan validate` warn
about bash syntax in `[meta] verify`. Test with a `[[ ]]` step.

## Done when

`plan_verify_runs_steps_like_task_verify` passes (the `[[verify]]` command).
