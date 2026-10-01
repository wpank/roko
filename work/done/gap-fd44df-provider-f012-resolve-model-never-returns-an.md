+++
id = "gap-fd44df"
kind = "gap"
title = "resolve_model() never returns an error; silently returns incomplete profiles for unknown models"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012"
anchors = ["crates/roko-core/src/agent.rs::resolve_model", "crates/roko-core/src/config/model_registry.rs::builtin_model"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'fn (try_)?resolve_model\\([^)]*\\) -> ([a-z_]+::)?Result<' crates/roko-core/src/agent.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:54Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:14Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
When a model key is not found in the registry or `roko.toml`, `resolve_model()` returns a partially-constructed `ModelProfile` with `profile: None` rather than an error. Callers proceed with an unconfigured profile that may cause silent failures or incorrect API calls.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012`
- `tmp/archive/provider-audit/06-model-config-resolution.md`

How to verify: Confirm in crates/roko-core/src/config/model_registry.rs whether still true: `resolve_model()` never returns an error; silently returns incomplete profiles for unknown models

Verified 2026-09-28: resolve_model lives in crates/roko-core/src/agent.rs:373, not model_registry.rs, and returns a bare ResolvedModel rather than a Result. After config key/slug/prefix lookups and a builtin_model registry fallback (agent.rs:400-431), unknown keys fall through to deprecated AgentBackend::from_model inference with no profile (agent.rs:432+). The builtin fallback now covers catalog models, so only unknown or typo keys reach the silent path. Severity p2.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `roko_core::agent::try_resolve_model` returns `RokoError::Config` naming a key that no `[models.*]` entry,
  configured slug or builtin model resolves, instead of the heuristic guess with `profile: None`. `resolve_model`
  keeps its infallible signature for its ~40 callers. Wired into `create_agent_for_model`: an unknown key with no
  command configured now fails with that reason instead of running `cat` as the agent (which echoed the prompt
  back as a successful answer); a configured command keeps the ExecAgent fallback. Not changed: roko-cli's
  `ProviderDispatchResolver::resolve` still picks a provider by the heuristic kind for an unknown key. Tests:
  `try_resolve_model_rejects_a_key_nothing_configures`, `an_unknown_model_without_a_command_is_an_error`.
