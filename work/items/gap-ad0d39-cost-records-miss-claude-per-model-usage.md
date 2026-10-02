+++
id = "gap-ad0d39"
kind = "gap"
title = "Cost records miss Claude per-model usage and reasoning tokens, and price unknown models as Sonnet"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude-cli", "roko-learn/cost-table"]
created = 2026-09-28
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::parse_stream_usage", "crates/roko-learn/src/cost_table.rs:25"]
links = { depends_on = [], blocks = [], related = ["find-af6b7f", "bug-b9cb83", "find-e16c23", "bug-c30f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'modelUsage' crates/roko-agent/src/claude_cli_agent.rs && ! sed -n '/fn usage_from_stream/,/^    }/p' crates/roko-agent/src/claude_cli_agent.rs | grep -q 'reasoning_tokens: 0' && ! grep -q 'None if total_tokens > 0 => &SONNET_FALLBACK' crates/roko-learn/src/cost_table.rs && ! grep -q 'SONNET_FALLBACK' crates/roko-agent/src/task_runner.rs"
+++
The Claude CLI usage parser (`claude_cli_agent.rs:441-506`) reads `total_cost_usd` and `usage.{input, output, cache_creation, cache_read}`. It ignores:
- `modelUsage` (per-model usage, including subagents);
- the cost basis and the cache-write TTL split.

`reasoning_tokens` is always 0, and cost is cast to `f32`. The cost table has one cache-write rate and prices any unknown model slug at the Sonnet rate (`roko-learn/src/cost_table.rs:25-31`).

Also reported by the assessment but not re-checked here: the OpenAI-compatible path ignores cached tokens, and the built-in Opus 4.6 cache-read price is 0.25× input instead of 0.1×.

Fix: parse the full usage record, price from one dated snapshot, and flag unknown models instead of guessing.

Re-checked 2026-09-29 (static): the two assessment claims not re-checked on 2026-09-28 also hold. The built-in claude-opus-4-6 entry prices cache reads at 3.75 against 15.00 input, i.e. 0.25x (asserted by the test at crates/roko-learn/src/cost_table.rs:325-329). No parsing of prompt_tokens_details.cached_tokens was found in provider/openai_compat.rs or tool_loop/backends/; openai_compat_backend.rs:826-834 only emits that field.

## Notes

- 2026-10-01 (wk-model-truth): partial on work/bug-3aa61f; cargo verification deferred to the batch check.
  Done: the Claude CLI parser sums the `result` event's `modelUsage` over every model (background turns and
  subagents included, as `total_cost_usd` is) and takes its `thinkingTokens` as reasoning tokens; roko-learn's
  `CostTable` no longer prices an unknown model at Sonnet's rates: `price` returns `None` and `calculate` records the
  unknown `0.0` (`Usage::has_known_cost`) and warns once per slug. Already true at BASE: the OpenAI-compatible usage
  parser reads `prompt_tokens_details.cached_tokens` (`translate/openai.rs::parse_usage_observation`).
  Left: roko-agent's `task_runner::CostTable`, a duplicate that `ModelCallService` uses for budget prediction and
  unpriced results, still guesses Sonnet rates (changing the prediction changes budget admission); pricing from one
  dated snapshot and the built-in claude-opus-4-6 cache-read rate (3.75 = 0.25x its 15.00 input) need a verified price
  source; the cache-write TTL split and a per-model breakdown are not recorded.
  Tests: `parse_stream_usage_counts_every_model_in_model_usage`, `an_unknown_model_is_unpriced_rather_than_priced_as_sonnet`.
- 2026-10-02 (wk-model-truth): the rest implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  roko-agent's `task_runner::CostTable` (the table `ModelCallService` prices calls and per-call budgets with) no
  longer guesses Sonnet rates: `calculate` takes the table's row (exact, or a dated snapshot via `is_snapshot_of`),
  else the shared registry's (`builtin_pricing`), else returns the unknown `0.0` and warns once. The warning moved to
  roko-core (`model_registry::warn_unpriced_model`, re-exported by roko-learn), so all three cost tables share it.
  Consequences: a provider-reported cost is no longer overwritten by a Sonnet guess for an unknown model, and the
  per-call `max_cost_usd` check cannot price, so does not block, a call to one. The verify now also checks
  task_runner. Tests: `an_unknown_model_is_unpriced_rather_than_priced_as_sonnet` (task_runner),
  `cost_predict_returns_zero_for_unknown_model` (now asserts the zero). Still not recorded: the cache-write TTL split
  and a per-model breakdown in cost rows.
