+++
id = "bug-3c4600"
kind = "bug"
title = "A gate that runs roko-cli's unit tests publishes a public gist with the person's gh login (share_run_scrubs_output_text_and_prompt), and the default whole-plan check now runs them in the roko repo"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "fb7e3df93"
source = "tmp/hermes-roko-v5/findings/roko-gates.md (roko-gates, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: fixing bug-a7e018 restores the default whole-plan check's cargo test -p in the roko repo"
anchors = ["crates/roko-cli/src/share.rs::share_run", "crates/roko-core/src/child_env.rs::GATE_ENV_NAMES", "crates/roko-cli/src/graph_execution/plan_verify.rs::default_plan_verify"]
links = { depends_on = [], blocks = [], related = ["bug-a7e018", "bug-c63cca", "gap-ebd5e0", "gap-3a5ce8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn verify_steps_get_the_check_environment' crates/roko-gate/ && cargo test -p roko-gate --lib -- verify_steps_get_the_check_environment a_verify_step_without_its_environment_does_not_run"

[[verify]]
command = "grep -rqw 'fn plan_checks_get_the_check_environment' crates/roko-cli/ && cargo test -p roko-cli --lib -- plan_checks_get_the_check_environment regression_checks_get_the_check_environment red_on_base_steps_get_the_check_environment"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T00:31:45Z"
commit = "fb7e3df93"
by = "roko-credfix"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-15T22:02:02Z"
model = "claude-opus-5-5"
forced = false
evidence = "roko_core::child_env::apply_check_env, reached via roko_gate::inherit_gate_env/_from/_std, now builds every check's env: the gate allowlist, GH_CONFIG_DIR = an empty read-only dir per run (begin_check_run in run_graph_plan_body), no GH_TOKEN/GITHUB_TOKEN/GH_ENTERPRISE_TOKEN/GITHUB_ENTERPRISE_TOKEN, provider keys roko loaded from .env only by exact name in [gates] env_passthrough, new [gates] strip_env, fail-closed, logged once per run. It is wired into ShellGate, GatePayload::apply_env, the integration/verify-chain/benchmark/generated gates, fix commands, plan_verify, the delivery regression check, and (moved off roko's whole env) spec_red_on_base, audit A2/B1/B2 and bench swe. Tests: roko-core 2032, roko-gate 701, roko-acp 237, roko-cli 3678 lib tests pass (gist test skipped), incl. a child-process test per path. Live CLI check (scratch project, Will's real HOME): gh auth status was 'logged in' with main's binary and 'not logged in' with this one on red-on-base, task verify and whole-plan check; the .env provider keys reached red-on-base before, none after. Details: tmp/hermes-roko-v5/findings/roko-credfix.md"
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

## Decision (2026-10-16)

The round's orchestrator chose option 2, widened to every check roko runs, and left the test and the gists alone
(they wait for Will). Every check command gets one environment, `roko_core::child_env::apply_check_env`:

- the gate allowlist;
- `GH_CONFIG_DIR` pointing at an empty, read-only directory per run;
- no `GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN` or `GITHUB_ENTERPRISE_TOKEN`;
- no provider key roko loaded from `.env` unless `[gates] env_passthrough` names it exactly;
- a new `[gates] strip_env`.

A check whose environment cannot be set up does not run. Under a check, `gh` finds no login, so
`share_run`'s `gh gist create` fails and the test falls back to its local file: it passes and publishes nothing.

Agent tools stay as they were (gap-ebd5e0). Git's own credentials (gap-3a5ce8) are not covered.
