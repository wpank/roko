+++
id = "gap-878ec1"
kind = "gap"
title = "[provider F083] ThinkingPreference::PreferThinking only fires at level=\"high\"/\"max\" AND complexity=Complex"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F083"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F083"
anchors = ["crates/roko-learn/src/cascade/helpers.rs", "ThinkingPreference::PreferThinking"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The cascade router's thinking-aware routing only activates for the most extreme combination. Intermediate thinking levels ("low", "medium") always return `Neutral` preference, meaning thinking-aware routing never fires for typical "medium" thinking configurations.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F083`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-learn/src/cascade/helpers.rs whether still true: `ThinkingPreference::PreferThinking` only fires at level="high"/"max" AND complexity=Complex
