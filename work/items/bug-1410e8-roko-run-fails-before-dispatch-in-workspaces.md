+++
id = "bug-1410e8"
kind = "bug"
title = "`roko run` fails before dispatch in workspaces with no Cargo.toml or go.mod and no configured gate"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/run"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-cli/src/run.rs::prompt_verify_steps"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "fn prompt_plan_without_build_manifest" crates/roko-cli/src/run.rs && cargo test -p roko-cli --lib prompt_plan_without_build_manifest'
+++

`roko run "<prompt>"` builds a one-task implementer plan and executes it with `run_graph_plan`. Its verify steps come from `prompt_verify_steps`: the configured required gate rungs; else `cargo check --workspace` when `Cargo.toml` exists; else `go build ./...` when `go.mod` exists; else nothing (run.rs:527-532). Implementer tasks must have a verify step, so in any other workspace (JavaScript, Python or docs, without a configured gate) the plan is rejected before an agent is dispatched (reported by w3d).

Fix: fall back to a language-neutral check, or allow verify-less prompt runs with the result marked unverified. Add a test named `prompt_plan_without_build_manifest`.
