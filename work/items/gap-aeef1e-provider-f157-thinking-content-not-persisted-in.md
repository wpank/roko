+++
id = "gap-aeef1e"
kind = "gap"
title = "[provider F157] Thinking content not persisted in episode logs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/episode_logger"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F157"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F157"
anchors = ["crates/roko-learn/src/episode_logger.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`EpisodeLogger` records usage metadata and prompt/output text but not reasoning content. Extended thinking traces from Kimi, GLM, or Anthropic are lost after the agent turn completes.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F157`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-learn/src/episode_logger.rs whether still true: Thinking content not persisted in episode logs
