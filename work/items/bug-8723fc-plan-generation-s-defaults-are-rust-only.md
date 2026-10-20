+++
id = "bug-8723fc"
kind = "bug"
title = "Plan generation's defaults are Rust-only: the auto-added compile verify is always cargo (infer_crate_from_paths: cargo check -p <crate> or --workspace) and the generator prompt's examples and rules teach only cargo, so in a C++/CMake repo every task got cargo checks (Undertow phase 2, op 9fa7849f)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#E-p2-plan2"
discovered_from = "agent:hermes-pilot (round v7 phase 2, op 9fa7849f)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::infer_crate_from_paths", "crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt"]
links = { depends_on = [], blocks = [], related = ["bug-574c45", "bug-72dc0d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cmake_workspace_gets_no_cargo_defaults' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_cmake_workspace_gets_no_cargo_defaults"
+++

## Problem

Plan generation's defaults assume a Rust workspace, whatever the project is. In round v7's Undertow phase 2 (a C++17 /
JUCE / CMake repo with no `Cargo.toml`; roko 5de243ec8, operation `9fa7849f`), this showed up twice:

- **roko's own repair.** T4 came back with no verify step, so `generate_plan` "auto-added one focused compile verify":
  `cargo check --workspace`. The code is in `pipeline.rs`, around `infer_crate_from_paths`:
  ```rust
  let compile_cmd = match &crate_name {
      Some(c) => format!("cargo check -p {c}"),
      None => "cargo check --workspace".to_string(),
  };
  ```
- **The planner's examples.** The generator system prompt teaches only cargo. Its worked examples are `roko-core`,
  `FundingRate` and `cargo test -p roko-cli status_shows_funding_rate`. Its rule on weak steps says "Prefer the crate's
  own gate (e.g. `cargo check -p roko-core`, `cargo test -p roko-core parse_config`)". The planner copied them: T1 to
  T3's checks are `cargo check -p undertow_core` and `cargo test -p undertow_core rumble` / `output_stage`.

bug-574c45 fixed one sentence (cmake/ctest instead of "pytest is NOT installed" in a CMake repo). The examples and the
auto-verify are still Rust-only.

## Why it matters

roko is pitched as provider- and model-agnostic, and the person brings their own project. A C++, Go, TypeScript or
Python project gets checks for a build system it doesn't have. They fail on every try, and bug-72dc0d shows that
nothing flags them.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: the auto-added compile verify (`infer_crate_from_paths`).
- `crates/roko-cli/src/plan_generate.rs`: `build_generator_system_prompt` and its examples.

## Plan

1. The auto-added compile verify comes from the workspace's build system, using the probe `runnable` already uses:
   cargo where there is a `Cargo.toml`, `cmake --build` with the project's preset where there are CMake presets,
   `go build ./...`, `npm test`, and so on. Where none is known, add no step, and leave the missing check as a finding.
2. Write the prompt's examples for the workspace's language, or language-neutral, and say which build and test commands
   this workspace has (the CMake sentence from bug-574c45, generalised).

## Done when

In a CMake workspace, the auto-added verify and the prompt's examples name no cargo command. Checked by the test in
`[[verify]]`.

## Notes

- Evidence is in `tmp/hermes-roko-v7/pilot/evidence/p2-plan2/`.
