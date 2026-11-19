+++
id = "gap-80d9e5"
kind = "gap"
title = "prompt_assembled.estimated_tokens counts only the system prompt at 4 bytes a token: 29-37% below the first call's real input on phases 5.1-5.3 (the user prompt and the tool definitions are left out, and these prompts run about 3 bytes a token)"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, §11 and the telemetry gaps T13-T16), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::assemble"]
links = { depends_on = [], blocks = [], related = ["gap-ece700"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn estimated_tokens_count_the_whole_request' crates/roko-cli/src && cargo test -p roko-cli --lib estimated_tokens_count_the_whole_request"
+++

## Problem

`prompt_assembled.estimated_tokens` is `system_prompt.len() / 4`: it counts only the system prompt, at 4 bytes a token.
On Undertow's last three runs it was 29–37% below the first call's real input, counted as fresh plus cached tokens
from `call_cost`:

| Run | Estimated | First call's input | Short by |
|---|---|---|---|
| Phase 5.1's resume, 4728d22e | 10,065 | 14,152 | 29% |
| Phase 5.2, 13260e60 | 10,659 | 16,931 | 37% |
| Phase 5.3, 2d5103db | 7,575 | 12,119 | 37% |

Phase 4.3 (3752b8ff) showed the same: 10.3–11.3k estimated against 15.4–16.7k.

There are two reasons:
- **What it leaves out:** the user prompt and the tool definitions.
- **The divisor:** Undertow's prompts run about 3.0 bytes a token, with line-numbered code and box-drawing text,
  not 4. That's measured on the prompt logs in `tmp/hermes-roko-v7/run-analyst/prompt_logs.py`.

## Why it matters

- Anyone who sizes a prompt or forecasts a try's cost from the record reads a third low. That includes dashboards,
  the run card's estimate, and a context-budget decision.
- The real size is only in `call_cost`, after the call.
- Goal: truth, for honest records.

## Where

`crates/roko-cli/src/dispatch/prompt_builder.rs::assemble`, at `let estimated_tokens = (system_prompt.len() / 4)`,
in the diagnostics block that feeds `prompt_assembled`.

## Plan

1. Count the whole request: the system prompt, the user prompt and the tool definitions as sent.
2. Use the model's tokenizer when one is at hand, or a measured bytes-a-token ratio (3.0 for these prompts), and
   say which in the record (`estimate_method`).
3. Optionally record `tool_schema_tokens` apart.
4. Test: `estimated_tokens_count_the_whole_request`.

## Done when

- The `[[verify]]` passes.
- The next run's `prompt_assembled.estimated_tokens` is within 10% of its first `call_cost` input.
