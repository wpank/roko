+++
id = "gap-13bbbd"
kind = "gap"
title = "Sampling parameters (temperature, seed) are never set or recorded"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-agent/providers"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '\"seed\"|seed: ' crates/roko-agent/src/provider/openai_compat.rs && grep -qiE 'temperature|sampling' crates/roko-cli/src/runtime_feedback/episodes.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:32Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:14Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
The OpenAI-compatible provider sets no temperature or seed except in two special cases: Kimi's thinking mode and an OpenRouter parameter requirement. No run record stores the sampling parameters used, so runs cannot be reproduced or compared across providers with known settings.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  Set: `[models.*]` takes `temperature` and `seed` (`ModelProfile`, with schema sentinels so a load keeps them);
  `openai_compat::build_extra_body_params` sends them, and owns Cerebras's temperature-0 default, which the backend
  factory duplicated. Record: `ExecutedModel.sampling` (the verdict's `executed`, left out when empty) holds what
  `openai_compat::request_sampling` says was sent, and each episode's `extra.sampling` names it (`{}` = provider
  defaults). Other providers send no sampling. Tests: `the_profile_sampling_reaches_the_request_and_its_record`,
  `a_models_sampling_settings_survive_a_load`, `episodes_record_the_sampling_sent`.
