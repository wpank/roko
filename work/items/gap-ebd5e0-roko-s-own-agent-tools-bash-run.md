+++
id = "gap-ebd5e0"
kind = "gap"
title = "roko's own agent tools (bash, run_tests, ACP bash, approved staged calls, ShellExec cells) still let an agent's command use the person's gh login"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-std"]
created = 2026-10-16
updated = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-credfix.md (roko-credfix, 2026-10-16)"
discovered_from = "bug-3c4600: checks got a hardened environment; agent tools were left out of scope"
anchors = ["crates/roko-std/src/tool/builtin/bash.rs", "crates/roko-std/src/tool/builtin/run_tests.rs", "crates/roko-acp/src/builtin_tools.rs", "crates/roko-cli/src/effects_apply.rs", "crates/roko-graph/src/cells/shell_exec.rs", "crates/roko-gate/src/gate_env.rs::inherit_tool_env_from"]
links = { depends_on = [], blocks = [], related = ["bug-3c4600"], supersedes = [], duplicate_of = "" }
+++

## Problem

bug-3c4600 gave every command roko runs as a check a hardened environment (`roko_core::child_env::apply_check_env`):
`GH_CONFIG_DIR` points at an empty directory, and GitHub tokens and the provider keys roko loaded from `.env` reach a
check only by exact name in `[gates] env_passthrough`. It left roko's own agent tools as they were, on purpose (out of
that item's scope). Their commands still get the plain gate policy (`child_env::apply_gate_env`), which keeps `HOME`
and `USER`. So `gh` in an agent's command finds the person's `~/.config/gh/hosts.yml` and keychain token, and acts on
GitHub as them, for example with `gh gist create --public`, `gh pr create` or `gh api`.

Which commands:

- roko-std `bash` and `run_tests` (`crates/roko-std/src/tool/builtin/bash.rs`, `run_tests.rs`): the tool loop of API
  models;
- ACP's `bash` (`crates/roko-acp/src/builtin_tools.rs`, through `roko_gate::gate_env::inherit_tool_env_from`);
- a staged tool call that `roko effects approve` runs (`crates/roko-cli/src/effects_apply.rs`);
- roko-graph's `ShellExec` cell (`crates/roko-graph/src/cells/shell_exec.rs`, `roko graph run`).

CLI agents (claude_cli, codex_cli, ...) run their own Bash under `CredentialScrub`. That keeps `HOME` too, so the same
applies to them. Whether a CLI agent should be allowed to use `gh` is a product decision, and it is not part of this
item.

## Why it matters

An agent's command is model-written, and a prompt injection in a file, issue or web page can ask for it. A
`gh gist create --public` or `gh api` in an agent command publishes as the person, which is what bug-3c4600 stopped for
checks.

## Plan (recommendation)

1. Give the four paths above the GitHub part of the check environment. Add a `child_env::apply_tool_env` that does
   `apply_gate_env`, then sets `GH_CONFIG_DIR` to `check_gh_config_dir()` unless `[agent] env_passthrough` names
   `GH_CONFIG_DIR` exactly, and drops `GITHUB_TOKEN_VARS` unless named exactly. Use it in place of `apply_gate_env` at
   the four call sites. A tool command whose environment cannot be set up fails like a spawn error.
2. Leave the rest of the agent policy as it is: `[agent] env_passthrough`, and no `strip_env` (the agent tools never
   had it).
3. Document in `docs/v2/19-CONFIG.md` (*Child process environments*) how a person who wants agents to use `gh` opts
   in: `[agent] env_passthrough = ["GH_CONFIG_DIR"]`.
4. Decide separately, with Will, whether CLI agents keep the person's `gh` login.

## Done when

`gh auth status` run through roko-std's `bash` tool, ACP's `bash`, an approved staged call and a `ShellExec` cell
reports "not logged in" on a machine where the person's `gh` is logged in. A unit test per path asserts the command's
environment has no GitHub token and an empty `GH_CONFIG_DIR`, as the bug-3c4600 tests do (`check_env_probe`).

## Notes

Filed by roko-credfix while fixing bug-3c4600 (hermes-roko-v5 round, 2026-10-16). See
`tmp/hermes-roko-v5/findings/roko-credfix.md`.
