+++
id = "gap-13bbbd"
kind = "gap"
title = "Sampling parameters (temperature, seed) are never set or recorded"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-agent/providers"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '\"seed\"|seed: ' crates/roko-agent/src/provider/openai_compat.rs && grep -qiE 'temperature|sampling' crates/roko-cli/src/runtime_feedback/episodes.rs"
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
