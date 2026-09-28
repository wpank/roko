+++
id = "gap-115f1a"
kind = "gap"
title = "[provider F178] Streaming parity: only 5 of 11 providers truly stream"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F178"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F178"
anchors = ["Multiple provider adapters"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Of 11 provider kinds, only OpenAiCompat, GeminiApi (native), GeminiCli, ClaudeCli, and CursorCli produce streaming output. AnthropicApi produces a synthetic stream; ACP providers, Hermes, OpenClaw, Perplexity, and Cerebras use non-streaming paths.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F178`
- `tmp/archive/provider-audit/28-parity-matrix.md`

How to verify: Related to deferred P3-4/AR-1 streaming unification; count providers with native stream_turn. Confirm in Multiple provider adapters whether still true: Streaming parity: only 5 of 11 providers truly stream
