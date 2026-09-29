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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '\"seed\"|seed: ' crates/roko-agent/src/provider/openai_compat.rs && grep -qiE 'temperature|sampling' crates/roko-cli/src/runtime_feedback/episodes.rs"
+++
The OpenAI-compatible provider sets no temperature or seed except in two special cases: Kimi's thinking mode and an OpenRouter parameter requirement. No run record stores the sampling parameters used, so runs cannot be reproduced or compared across providers with known settings.
