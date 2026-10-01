+++
id = "find-a3f8f1"
kind = "finding"
title = "Unknown model slugs get the degraded tool profile: no native tool calling and at most 3 tools"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-core/tool-format"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e5-failover"
anchors = ["crates/roko-core/src/tool/format.rs::ToolFormatProfile::unknown_default"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -A12 'fn unknown_default' crates/roko-core/src/tool/format.rs | grep -qE 'supports_tools: false|max_tools_before_degrade: 3' || grep -B4 '^    ToolFormatProfile::unknown_default()$' crates/roko-core/src/tool/format.rs | grep -q 'warn!'"
+++

`ToolFormatProfile::unknown_default` gives any model slug missing from the profile table `supports_tools: false`, a JSON-mode fallback and `max_tools_before_degrade: 3`. A newly added or renamed model therefore runs with a three-tool subset (the e5 audit found `bash` was not among them) and no native tool calling, which looks like poor model quality rather than a configuration gap.

Decide whether unknown slugs should inherit their provider's profile, fail with a clear message, or at least warn once.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  Took the item's minimum option: `profile_for_model` warns once per slug per process when it falls back to
  `unknown_default`, saying the model is capped at 3 tools (and gets ReAct text unless its `[models.*]` entry sets a
  `tool_format`) and that `max_tools` fixes it. The default itself is unchanged. Inheriting the provider family's
  profile for an unknown slug is still open: a configured model gets `supports_tools`/`tool_format` from its entry but
  its tool cap from this table (`capabilities_from_profile`), and choosing a family cap is a product call.
  Test: `an_unknown_slug_is_warned_about_once`.
