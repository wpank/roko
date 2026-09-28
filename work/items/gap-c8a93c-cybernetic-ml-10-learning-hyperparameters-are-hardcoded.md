+++
id = "gap-c8a93c"
kind = "gap"
title = "[cybernetic ML-10] Learning hyperparameters are hardcoded; no meta-tuning"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-10: Meta-Hyperparameter Tuning"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-10: Meta-Hyperparameter Tuning"
anchors = ["AdaptiveThresholds ema_alpha", "CascadeRouter stage transitions", "[learning.gate_thresholds] config"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
EMA alpha=0.1, UCB exploration, CUSUM thresholds, cascade stage transitions at 50/200 are constants; no post-run adjustment via grid search/Bayesian optimization against prediction error.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-10: Meta-Hyperparameter Tuning`

How to verify: Check whether any code adjusts these hyperparameters from observed outcomes (config-only values do not count).
