+++
id = "bug-8723fc"
kind = "bug"
title = "Plan generation's defaults are Rust-only: the auto-added compile verify is always cargo (infer_crate_from_paths: cargo check -p <crate> or --workspace) and the generator prompt's examples and rules teach only cargo, so in a C++/CMake repo every task got cargo checks (Undertow phase 2, op 9fa7849f)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "9f0870e9a"
source = "round-v7-hermes-pilot#E-p2-plan2"
discovered_from = "agent:hermes-pilot (round v7 phase 2, op 9fa7849f)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::infer_crate_from_paths", "crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt"]
links = { depends_on = [], blocks = [], related = ["bug-574c45", "bug-72dc0d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cmake_workspace_gets_no_cargo_defaults' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_cmake_workspace_gets_no_cargo_defaults"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T15:08:56Z"
commit = "6cbd4e8a4"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
size = "L"
claimed_at = "2026-10-20T11:42:02Z"
forced = false
evidence = "Merged 9f0870e9a (work/bug-72dc0d: 6cbd4e8a4). The TOML fix-up's auto-added compile check is a cargo check only for a task that writes Rust; runnable::repair_build_checks gives an implementer task that writes C/C++ files and has no step `cmake --preset X && cmake --build --preset X`, X the preset named for its files' first directory (else the first); a workspace whose build roko does not know gets none. plan_generate/build_system.rs: for a CMake workspace with no Cargo.toml, the generator prompt's Rust passages are rewritten in the workspace's own preset commands (both example plans, the minimum structure, principle 6's example, the context check, the language list, the gate wording, the Rust accept example, file-path rules 1 and 6), and a \"This workspace's build\" note says how it builds and that no step runs cargo; a Rust workspace's prompt is unchanged (a test holds every rewritten passage to occurring once). The language list gains CMake, the runner note says a workspace with no Cargo.toml runs no cargo check, and rule 5 and the checklist say the project's build, test and gate. Verify: a_cmake_workspace_gets_no_cargo_defaults passes (in a CMake workspace with Undertow's presets the system prompt has no \"cargo check\", \"cargo test\", \"crates/roko-core\" or \"FundingRate\", has `cmake --preset core && cmake --build --preset core` and `ctest --preset core -R gain`; a Rust workspace keeps `cargo check -p roko-core`; a C++ task with no step gets no cargo from the fix-up and gets the core preset build from repair_build_checks; the runner note says no cargo), with every_rust_passage_is_in_the_prompt_once, the_commands_follow_the_presets and the_cpp_examples_parse. In the plan-write replays (see bug-72dc0d) T4's auto-added `cargo check --workspace` is gone. Branch: as bug-72dc0d."
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
