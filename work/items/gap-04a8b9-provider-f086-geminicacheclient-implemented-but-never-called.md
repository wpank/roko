+++
id = "gap-04a8b9"
kind = "gap"
title = "[provider F086] GeminiCacheClient implemented but never called in production"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F086"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F086"
anchors = ["crates/roko-agent/src/gemini/cache.rs", "GeminiCacheClient"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The Gemini explicit context caching client (`GeminiCacheClient` with `create_cache()`/`delete_cache()`) is fully implemented but has no call site outside tests. Explicit Gemini caching requires manual invocation that has not been wired into any dispatch path.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F086`
- `tmp/archive/provider-audit/19-caching.md`

How to verify: Roadmap P4-3 deferred. Check whether GeminiCacheClient is called from any production path. Confirm in crates/roko-agent/src/gemini/cache.rs whether still true: `GeminiCacheClient` implemented but never called in production
