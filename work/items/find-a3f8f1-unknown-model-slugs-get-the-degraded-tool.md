+++
id = "find-a3f8f1"
kind = "finding"
title = "Unknown model slugs get the degraded tool profile: no native tool calling and at most 3 tools"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-core/tool-format"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-core/src/tool/format.rs::ToolFormatProfile::unknown_default"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -A12 "fn unknown_default" crates/roko-core/src/tool/format.rs | grep -q "max_tools_before_degrade: 3"'
+++

`ToolFormatProfile::unknown_default` gives any model slug missing from the profile table `supports_tools: false`, a JSON-mode fallback and `max_tools_before_degrade: 3`. A newly added or renamed model therefore runs with a three-tool subset (the e5 audit found `bash` was not among them) and no native tool calling, which looks like poor model quality rather than a configuration gap.

Decide whether unknown slugs should inherit their provider's profile, fail with a clear message, or at least warn once.
