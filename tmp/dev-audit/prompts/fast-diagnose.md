# FAST_DIAGNOSE prompt

Use this after the harness has collected a bounded run bundle. Do not give the diagnosis agent the
entire repository or global logs by default.

---

MODE: FAST_DIAGNOSE

Goal: identify the first causal failure and the smallest next experiment from one run bundle.

Time budget: 60 seconds.

Rules:

1. Treat manifest, monotonic spans, command exits, and terminal events as facts.
2. Separate facts, inferences, and unknowns.
3. Diagnose the first causal failure, not every downstream symptom.
4. Do not search global logs until the run-scoped bundle is shown incomplete.
5. Do not change code.
6. Do not recommend a full workspace rebuild/test unless the evidence shows cross-workspace impact.
7. Prefer one discriminating experiment under 60 seconds.
8. Flag metric contradictions such as multiple dispatches per attempt, elapsed time after terminal,
   or exit code/outcome disagreement.

Inputs:

RUN_ID:
<run id>

GOAL:
<expected outcome>

RISK_TIER:
<T0 | T1 | T2 | T3>

SUMMARY_JSON:
<summary.json>

TIMINGS:
<timings.json>

FIRST_FAILURE_WINDOW:
<bounded events/commands/log excerpts around the first failure>

DIFF_STAT:
<diff stat and authorized files>

RUNTIME_EVIDENCE:
<endpoint/screenshot/CLI results>

Required final JSON:

    {
      "outcome": "confirmed_root_cause | likely_root_cause | insufficient_evidence",
      "first_causal_failure": "...",
      "facts": ["..."],
      "inferences": ["..."],
      "unknowns": ["..."],
      "smallest_next_experiment": {
        "command_or_action": "...",
        "time_budget_seconds": 60,
        "expected_discriminator": "..."
      },
      "recommended_owner": "runner | build | provider | plan | harness | human"
    }
