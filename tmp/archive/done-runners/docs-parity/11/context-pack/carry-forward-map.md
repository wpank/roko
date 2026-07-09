# Carry-Forward Map — 11 Safety

These topics are real, but they are not part of the audit-constrained `tmp/docs-parity/11` refresh.

| Topic | Better Home | Why |
|-------|-------------|-----|
| NIST AI RMF / MITRE ATLAS / STRIDE-AI / OWASP Agentic / CSA MAESTRO mappings | later compliance/program pass | informational taxonomy, not shipped runtime parity |
| Kelly sizing / Beta-Binomial / 5D safety budgets / delegation math | later adaptive-risk pass | design-heavy and outside the current status refresh |
| MEV detection / LTL-Buchi / witness DAG / formal-verification pipeline | later chain activation pass | frontier chain work, not batch-11 parity scope |
| CaMeL / Ventriloquist / other frontier prompt-security patterns | later prompt-security or chain pass | not required to describe the shipped safety system honestly |
| Cognitive kernel / namespaces / scheduling / syscall model | later kernel redesign pass | much broader than a parity-note rewrite |
| Regulator-facing forensic export packaging | later compliance packaging pass | positioning and export design, not current shipped surface |
| Full Denning / FIDES / RTBAS / PFI / PCAS taint algebra | later taint-deepening pass | shipped `TaintTracker` is narrower and that is acceptable to state plainly |

Working rule:

Keep `M1` and `M2` to shipped-safety wording and coverage-status fixes, let `M3` defer chain material cleanly, and do not turn `M4` context refresh into a roadmap rewrite.
