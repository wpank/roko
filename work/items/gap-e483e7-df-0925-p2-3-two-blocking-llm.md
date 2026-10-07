+++
id = "gap-e483e7"
kind = "gap"
title = "Two blocking LLM calls on every gate failure; cheap-agent timeout hardcoded"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -B14 'error_enrichment::enrich_error_digest(' crates/roko-cli/src/graph_task_dispatch.rs | grep -q 'tokio::spawn' && cargo test -p roko-cli --lib cheap_agent_uses_llm_call_timeout_and_the_selected_model_key"
+++
quality_judge and enrich_error_digest are awaited inline (~43s per failure, ~86 of 96s overhead) though neither gates the retry; CheapFactoryAgent hardcodes a 30s timeout ignoring timeouts.llm_call_secs.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-3. Two blocking LLM calls on every gate failure`

How to verify: Time retry latency after a gate failure.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed parts (committed at HEAD): judge_quality now runs in a detached tokio::spawn (graph_task_dispatch.rs:2190, call at :2197), so it no longer blocks the retry. CheapFactoryAgent's timeout now comes from timeouts.llm_call_secs, not a hardcoded 30s (cheap_agent :1296-1307, test :6420; 725f21e05). Remaining: enrich_error_digest is still awaited inline on every gate failure, bounded by timeouts.llm_call_secs (:2371-2380). The code keeps it inline on purpose because the diagnosis feeds the retry prompt, so one blocking LLM call per failure remains.

Rechecked 2026-09-29 at d9e79e9d8: the judge_quality detach and the llm_call_secs timeout for CheapFactoryAgent (725f21e05) are still in place. One blocking LLM call per gate failure remains: enrich_error_digest is awaited inline in settle_task_verification (graph_task_dispatch.rs:2493-2512 at HEAD), bounded by timeouts.llm_call(). The code keeps it inline on purpose because the diagnosis feeds the retry prompt; if that is accepted, close the remainder as wontfix, otherwise run it concurrently with retry preparation.

## Notes

- 2026-10-01 (wk-honestbench): blocked on a decision; no code changed. At BASE `ebdc0f5d5` the fixed parts hold:
  `quality_judge` runs in a detached `tokio::spawn` (`graph_task_dispatch/verification.rs:646-700`), and
  `cheap_agent` uses `timeouts.llm_call_secs` (test `cheap_agent_uses_llm_call_timeout_and_the_selected_model_key`,
  `routing_context.rs:932`). One call remains inline: `enrich_error_digest` (`verification.rs:859-878`), bounded by
  `timeouts.llm_call()`, because its diagnosis goes into the retry prompt. The engine retries a failed node at once
  (no backoff), so overlapping the call with the next attempt's setup saves only that setup time, unless the retry
  may start without the diagnosis. Next step: decide whether to (a) keep it inline and close this item as won't-fix,
  or (b) overlap it: `RetryFeedbackBook` holds a pending diagnosis that the next attempt's prompt build awaits for
  what is left of the timeout, and the feedback kept on disk for resumed runs is rewritten when the diagnosis lands.
  Either way the `[[verify]]` needs updating: it greps `graph_task_dispatch.rs`, but the call now lives in
  `graph_task_dispatch/verification.rs`.
