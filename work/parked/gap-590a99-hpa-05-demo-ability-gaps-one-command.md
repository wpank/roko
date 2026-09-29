+++
id = "gap-590a99"
kind = "gap"
title = "HPA-05: Demo-ability gaps (one-command demo launch, shareable session link, conversation history endpoint, live-mode switch)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["apps/portal"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/05-roko-demoability.md#1. Current Demo-ability Score: 6 / 10"
discovered_from = "audit:tmp/hermes-product-audit/05-roko-demoability.md#1. Current Demo-ability Score: 6 / 10"
anchors = ["/api/runs/{id}/share", "/api/shared/{token}"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Demo-ability scored 6/10; missing: one-command demo launch opening the UI, wiring shareable run links (POST /api/runs/{id}/share exists) into the app, conversation history endpoint, tool-call cards, demo-app live mode switch.

Imported without verification from:
- `tmp/hermes-product-audit/05-roko-demoability.md#1. Current Demo-ability Score: 6 / 10`
- `tmp/hermes-product-audit/05-roko-demoability.md#2. Top 10 Things to Make Roko Most Demo-able`
- `tmp/hermes-product-audit/05-roko-demoability.md#4. Architectural Changes Needed`

Warning: every file this item cites is gone (`/api/runs/{id}/share`, `/api/shared/{token}`) — likely obsolete or moved.

How to verify: Check for a demo launch script and share-link usage in the portal.
