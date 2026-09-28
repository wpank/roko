+++
id = "spec-31909e"
kind = "spec"
title = "PRD: plugin discovery at runtime (E30-T09, .roko/plugins/ into runner init)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-plugin/discovery"]
created = 2026-08-13
updated = 2026-09-28
source = ".roko/prd/drafts/plugin-discovery-runtime.md"
discovered_from = "audit:.roko/prd/drafts/plugin-discovery-runtime.md"
anchors = ["discover_plugins"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Draft PRD: plugin manifests load but plugins are never discovered at startup; add discover_plugins() to runner init. E32 now claims fail-fast startup/strict admission.

Imported without verification from:
- `.roko/prd/drafts/plugin-discovery-runtime.md`

How to verify: Likely implemented under E32; verify startup discovery then archive PRD.
