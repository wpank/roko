+++
id = "bug-f9ca2f"
kind = "bug"
title = "Helper calls (retry error enrichment, gate reflections, the judge) pick their model without provider health or the run's failed rung probes: with cerebras answering 402 (circuit open, probe failed), every enrichment in run 82b7e069 still went to gpt-oss-120b and fell back to the deterministic digest"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "26820193b"
source = "round-v7-hermes-pilot#K-cerebras-402"
discovered_from = "agent:hermes-pilot (round v7 phase 2 run 82b7e069, roko.log 18:40-18:46Z)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::select_cheap_model_key", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn helper_model_passes_over_a_provider_with_an_open_circuit' crates/roko-cli/src && cargo test -p roko-cli --lib -- helper_model_passes_over_a_provider_with_an_open_circuit"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T07:09:06Z"
commit = "26820193b"
via = "manual"
size = "S"
forced = true
evidence = "26820193b: cheap_agent (error enrichment, gate reflections) and judge_model_key now choose only among models whose provider the run's health registry holds available (ProviderHealthRegistry::is_available, the check task routing's failover uses) and that did not fail their tool-use probe this run: SharedAgentFactory keeps the failed probes skip_failed_rungs is given (failed_probe, by key or slug). With none healthy, no helper is called and its callers keep their deterministic fallback. Test helper_model_passes_over_a_provider_with_an_open_circuit (run 82b7e069's shape: oss on cerebras $0.35/M, mini on openai $0.50/M whose probe failed, glm on zai $0.60/M): oss while cerebras is healthy; with cerebras's circuit open (3 Billing failures) red at 08:25 still picked oss, green picks glm, and so does the judge; with zai's open too, no helper. Checks: roko-cli lib 3975 passed, 0 failed (share test skipped); the targeted run's one failure, run::tests::prompt_run_on_a_red_base_judges_only_new_failures at load average 160, passed 4 of 4 alone at lower load with the same binary (its baseline check is load-sensitive); clippy -D warnings (roko-cli) clean; nightly rustfmt clean."
+++

## Problem

Round v7, Undertow phase 2, run `82b7e069` (roko d7d1fbb8b). The cheap rung's provider, cerebras (gpt-oss-120b),
answered every call with `402 Payment Required` ("Payment required to access this resource. Visit your billing
tab.", `payment_required_error`, param `quota`). roko already knew:

- the rung probe at plan start (18:33:27Z) failed it: `.roko/learn/rung-probes.json` holds `gpt-oss-120b: passed
  false, reason "insufficient_credits: … http 402"`. The plan runner dropped the rung (`skip_failed_rungs`), and
  every `route_decided` marked cerebras `provider_unhealthy`;
- after the first helper call failed, the provider health registry opened cerebras's circuit (Billing, 24 h
  cooldown).

The helper calls still went to cerebras. After each of the 8 failed tries (T2 and T3, 4 each, 18:40:53Z–18:46:44Z),
the retry's error enrichment did this:

```
INFO roko_agent::provider: creating agent via provider adapter model_key="gpt-oss-120b" slug=gpt-oss-120b provider=openai_compat base_url=Some("https://api.cerebras.ai/v1")
INFO roko_learn::provider_health: provider failure recorded … provider=cerebras error_class=Billing
```

Each failed in about 0.2 s. `enrich_error_digest` then returned its deterministic `fallback_diagnosis`, so each retry
lost the model's diagnosis, while healthy models (glm-4.7 on zai, gpt-5.4-mini on openai) were available.

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs` (~line 926): error enrichment before a retry, through
  `self.cheap_agent()` and `roko_learn::error_enrichment::enrich_error_digest`.
- `crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent`: `select_cheap_model_key` among `side_call_models()`.
  The same helper agent runs gate reflections and the opt-in judge (`judge_model_key` → `select_judge_model_key`).
- `crates/roko-cli/src/graph_task_dispatch/routing_context.rs::select_cheap_model_key` (line 97): its availability
  check is `config.provider_available_for_model_key(key)`, which is configuration only (enabled, credentialed). It
  doesn't consult the health registry (`ProviderHealthRegistry::is_available`, which task routing's guards use,
  `dispatch/model_routing.rs::ineligible_reason`) or the run's failed rung probes.
- The setting: `[routing] fast_task_model`. Undertow's `roko.toml:117` says `"claude-haiku-4-5"`, which names no
  `[models.*]` entry there, so the selection falls through to the cheapest model by price: gpt-oss-120b ($0.35/$0.75).

## Plan

Pass the health registry and the run's failed probes to the helper selection. A model whose provider's circuit is
open, or whose rung probe failed in this run, isn't a candidate, so the next cheapest healthy one is chosen (here
glm-4.7). If none is healthy, keep today's behaviour: the deterministic fallback, with no call.

## Done when

With cerebras's circuit open (or its probe failed) and glm-4.7 available, the helper agent `cheap_agent` builds uses
glm-4.7, not gpt-oss-120b. Checked by the test in `[[verify]]`.
