# Carry-Forward Map — 11 Safety

These findings are real, but should usually be handed to later passes.

| Finding | Better Home | Why |
|---------|-------------|-----|
| NIST AI RMF / MITRE ATLAS / STRIDE-AI / OWASP Agentic / CSA MAESTRO mapping depth | later compliance/program pass | documentation taxonomy, not shipping runtime |
| Kelly sizing / Beta-Binomial / 5D safety budgets / hierarchical delegation math | later adaptive-risk implementation pass | mostly design surface today |
| MEV detection / LTL-Buchi / witness-DAG expansion / full formal-verification toolchain | later chain activation pass | blocked on Tier 6 chain work |
| CaMeL dual-LLM / Ventriloquist enforcement | later prompt-security or chain pass | frontier patterns, not topic-11 runtime blockers |
| Cognitive namespaces / scheduling / Engram syscalls | later kernel redesign pass | deeper than doc parity |
| Regulator-facing forensic export templates | later compliance packaging pass | positioning today, not productized runtime |
| Advanced Denning/FIDES/RTBAS/PFI/PCAS taint algebra | later taint-deepening pass | shipping tracker is narrower but sufficient |

Working rule:

If the task stops being "make the docs describe the real safety stack"
and starts becoming "design or implement the next safety architecture",
capture the seam and defer it.
