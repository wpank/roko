+++
id = "bug-a1cff1"
kind = "bug"
title = "roko demo setup exits 0 when its release build fails"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/demo"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-171238 (2026-10-27)"
discovered_from = "gap-171238"
anchors = ["crates/roko-cli/src/demo_cmd.rs::cmd_demo_setup"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-171238"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn demo_setup_fails_when_the_build_fails' crates/roko-cli/ && cargo test -p roko-cli demo_setup_fails_when_the_build_fails"
+++

## Problem

`roko demo setup` (`crates/roko-cli/src/demo_cmd.rs::cmd_demo_setup`) runs
`cargo build -p roko-cli --release`. When the build fails it prints `build failed` and returns
`Ok(())`, and the `Command::Demo` arm of `crates/roko-cli/src/main.rs` turns that into
`EXIT_SUCCESS`: a script such as `roko demo setup && roko serve` carries on with no new release
binary. Three smaller defects in the same function:

- the build runs in the process's current directory, not in `--workdir` (the `Command` sets no
  `current_dir`), while the rest of the setup uses `--workdir`;
- the `roko init` it runs for a workspace without `.roko/` has its exit status ignored
  (`let _ = Command::new(roko).arg("init")…status()`);
- the module doc lists a `roko demo serve` that `DemoCmd` does not have.

## Why it matters

Exit status is how scripts and the docs' examples chain commands. A setup that fails and reports
success hides the failure until the demo starts.

## Where

- `crates/roko-cli/src/demo_cmd.rs::cmd_demo_setup` and the module doc.
- `crates/roko-cli/src/main.rs`: the `Command::Demo` arm, which returns `EXIT_SUCCESS` after
  `cmd_demo_setup`.

## Current state

Checked 2026-10-27 at 0191f7858. `docs/v3/28-CLI.md` §`roko demo` (gap-171238) says that a failed
build still exits 0.

## Plan

1. Return an error, or an exit code the arm passes on, when the build or `roko init` fails.
2. Run the build in the `--workdir` workspace, or say in the help that it builds the current
   checkout.
3. Drop `roko demo serve` from the module doc.
4. Test it: make the build program injectable (for example `cmd_demo_setup_with(workdir, cargo)`),
   point it at a stand-in that exits 1, and check the command fails.

## Done when

A failing release build makes `roko demo setup` exit non-zero: test
`demo_setup_fails_when_the_build_fails` passes. Then drop the "still exits 0" sentence from
28-CLI.md §`roko demo`.

## Notes

Found while writing 28-CLI.md's `roko demo` section (gap-171238). The test must not run a real
release build.
