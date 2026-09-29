+++
id = "gap-b0f2bd"
kind = "gap"
title = "[provider F189] Onboarding docs present config as fully authoritative; no add-a-model checklist covers the ~25 slug-heuristic surfaces"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/model-registry"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F189"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F189"
anchors = ["examples/adding-a-provider.md", "examples/adding-a-custom-protocol.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`examples/adding-a-provider.md` documents `[providers.*]` and `[models.*]` fields in detail without mentioning slug heuristics, so a reader would reasonably conclude config fields are fully authoritative. `examples/adding-a-custom-protocol.md` goes further, listing "adding a slug heuristic when c...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F189`
- `tmp/archive/provider-audit/09-model-registry.md`
- `tmp/archive/provider-audit/31-slug-heuristic-map.md`

How to verify: Roadmap SH-6 (add-a-model checklist in examples/adding-a-provider.md) deferred. Confirm in examples/adding-a-provider.md, examples/adding-a-custom-protocol.md whether still true: Onboarding docs present config as fully authoritative; no add-a-model checklist covers the ~25 slug-heuristic surfaces
