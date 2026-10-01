+++
id = "gap-e9660f"
kind = "gap"
title = "Whole-project reads such as grep -r, rg or cat * can still show agents a secret stored in roko.toml"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-std/sandbox", "roko-agent/claude_cli_guard", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-34c16c, branch work/bug-ceab60 at f80d4eb50)"
anchors = ["crates/roko-std/src/tool/builtin/sandbox.rs", "crates/roko-agent/src/claude_cli_guard.py", "crates/roko-core/src/config/serve.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-34c16c"], blocks = [], related = ["bug-34c16c", "bug-524a3b", "bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_secret_in_the_project_roko_toml_is_moved_or_refused' crates/roko-core/src/ && cargo test -p roko-core --lib a_secret_in_the_project_roko_toml_is_moved_or_refused"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in e1e6159f7. The loader and serve's PUT refuse a readable config file holding a literal secret (empty values and ${VAR} references allowed). Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

bug-34c16c's fix (branch `work/bug-ceab60`, bc9725965, not merged at d2cc43346) refuses agents a roko config file while it holds a secret. It covers roko-std's file tools, grep's walk and the bash check, SafetyLayer's path policy, and the Claude guard. Its own notes report what it can't cover: a Bash command that reads the project wholesale (`grep -r`, `rg`, `cat *`) can still reach a secret in `roko.toml`. The Claude permission deny rules are static, so only the hooks apply the content check.

## Why it matters

Secrets and guard (epic spec-ba7bea): agents grep the whole repository all the time, so the serve API key would land in a transcript. The durable fix, as the branch notes say, is to keep secrets out of `roko.toml`.

## Where

The refusal code listed above, and the config loader and serve config that allow the secret in `roko.toml`.

## Plan

1. Keep secrets out of agent-readable files. On load, a secret found in the project `roko.toml` is refused with a message, or moved to `.roko/.env` as `ROKO__SERVE__AUTH__API_KEY` by an explicit command (`roko config migrate-secrets`). bug-524a3b stops roko from telling users to put it there.
2. Until then, keep the content check, and document that wholesale reads bypass it.
3. Add `a_secret_in_the_project_roko_toml_is_moved_or_refused`.

## Done when

- [ ] No agent-readable file holds a roko secret, so wholesale reads find none.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise held at 8a88c6267: the loader accepted a secret in `roko.toml` (bug-34c16c only warned), so `grep -r`, `rg` or `cat *` could show it to an agent.
- Decision: refuse, with no automatic migration. The loader (`parse_from_resolved_path`, and the global merge for the legacy `~/.config/roko/config.toml`) refuses any config file agents can read that holds a literal secret: `LoadConfigError::SecretInConfig` names each field and the `ROKO__` variable to set in `.roko/.env` (or a `${VAR}` reference for provider headers and `agent.env`), never the value. `${VAR}` references, `*_file` headers and key files such as `~/.roko/config.toml` pass. serve's `PUT /api/config` refuses too. No `roko config migrate-secrets`: `roko config set <field> <value>` moves one (bug-524a3b).
- The content check stays for a secret added while roko runs (`child_env::is_config_with_secrets`, the guard's config scan, now matching the loader's `secret_fields`). Wholesale reads (`grep -r`, `rg`, `cat *`) still bypass it, as the guard header and the `is_config_with_secrets` doc say.
- Behaviour change for operators: a secret in `roko.toml` now stops roko until it moves. `docker/RAILWAY.md` shows `railway_api_token = "..."` and docs/v3 show `api_key` in `roko.toml`; both need updating (not done here).
- Implemented on `work/gap-e9660f` at `33366967e`; cargo verification deferred to the batch check.
