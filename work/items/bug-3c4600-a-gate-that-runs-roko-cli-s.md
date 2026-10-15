+++
id = "bug-3c4600"
kind = "bug"
title = "A gate that runs roko-cli's unit tests publishes a public gist with the person's gh login (share_run_scrubs_output_text_and_prompt), and the default whole-plan check now runs them in the roko repo"
status = "open"
triage = "unverified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/roko-gates.md (roko-gates, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: fixing bug-a7e018 restores the default whole-plan check's cargo test -p in the roko repo"
anchors = ["crates/roko-cli/src/share.rs::share_run", "crates/roko-core/src/child_env.rs::GATE_ENV_NAMES", "crates/roko-cli/src/graph_execution/plan_verify.rs::default_plan_verify"]
links = { depends_on = [], blocks = [], related = ["bug-a7e018", "bug-c63cca"], supersedes = [], duplicate_of = "" }
+++

## Problem

roko-cli's unit test `share::tests::share_run_scrubs_output_text_and_prompt` calls `share_run`, which uploads with
`gh gist create --public`. Gate commands keep `HOME`, `USER` and `PATH` (`roko_core::child_env::GATE_ENV_NAMES`), so
`gh` finds the person's login (keychain or `~/.config/gh`) and the upload succeeds: every unfiltered
`cargo test -p roko-cli` that a gate runs on the person's machine publishes a public gist ("roko run transcript
abc12345"). 513 of them were found on Will's account by 2026-10-08, some from wave gates.

Since bug-a7e018 (2026-10-15) roko reads the roko workspace's package graph again, so a plan in the roko repo that
writes a roko-cli file and has no `[meta] verify` gets the default whole-plan check (gap-60233f)
`cargo fmt -p roko-cli -- --check`, `cargo clippy -p roko-cli --no-deps -- -D warnings`, `cargo test -p roko-cli`.
The last step is unfiltered, so it runs the test above (and builds all ~70 roko-cli integration test binaries; about a
dozen need a real claude CLI, bug-c63cca). Before bug-a7e018 the broken metadata turned that check into fmt + clippy
of the workspace, with no tests, which hid this.

## Why it matters

A check roko runs on its own, in the background, from Hermes or `roko serve`, publishes content to a public account
of the person without asking. It also lets any agent-written test act on GitHub as the person.

## Where

- `crates/roko-cli/src/share.rs::share_run` and its test `share_run_scrubs_output_text_and_prompt`.
- `crates/roko-core/src/child_env.rs::GATE_ENV_NAMES` (`HOME`, `USER` reach gate commands).
- `crates/roko-cli/src/graph_execution/plan_verify.rs::default_plan_verify` (the unfiltered `cargo test -p …`).

## Plan

Options (Will decides; the memory note `share_test_publishes_public_gists` says to leave the test and the gists
alone until he does):
1. The test never uploads: `share_run` takes its uploader, and the test passes a fake.
2. Gates hide `gh`'s login: gate commands get `GH_CONFIG_DIR` pointing at an empty directory (and no
   `GH_TOKEN`/`GITHUB_TOKEN`, which they already lose), so no check can act on GitHub as the person.
3. The default whole-plan check runs lighter tests (`--lib`, or none above N affected packages).

## Done when

A gate-run `cargo test -p roko-cli` on a machine with a logged-in `gh` publishes nothing.

## Notes

Found by roko-gates (hermes-roko-v5) while fixing bug-a7e018; not changed here.
