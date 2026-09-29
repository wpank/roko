+++
id = "gap-e33fea"
kind = "gap"
title = "[cybernetic ML-02] Predict-Publish-Correct is not a universal Bus protocol"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/bus"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-02: Predict-Publish-Correct as Universal Bus Protocol"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-02: Predict-Publish-Correct as Universal Bus Protocol"
anchors = ["CalibrationPolicy", "CascadeRouter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
CalibrationPolicy exists but is not Bus-mediated and only CascadeRouter uses it. Scorer/Composer/Gate/Policy operators should publish predictions to the Bus and be corrected by outcomes. Audit rates HIGH: the architectural thesis depends on it.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-02: Predict-Publish-Correct as Universal Bus Protocol`

How to verify: grep CalibrationPolicy usages; check whether Bus carries prediction/correction events for non-router operators.
