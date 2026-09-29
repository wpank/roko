+++
id = "gap-3cd428"
kind = "gap"
title = "HPA-07 §4.6: Classified provider error kind not surfaced in InferenceFailed events"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.6 Provider audit and inference gateway monitor"
discovered_from = "audit:tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.6 Provider audit and inference gateway monitor"
anchors = ["crates/roko-agent/src/provider/error_classify.rs", "InferenceFailed"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
error_classify.rs classifies provider errors but InferenceFailed events do not carry the kind, so UIs cannot show why inference failed.

Imported without verification from:
- `tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.6 Provider audit and inference gateway monitor`

How to verify: Inspect InferenceFailed payload fields.
