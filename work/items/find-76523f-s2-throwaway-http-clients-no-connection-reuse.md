+++
id = "find-76523f"
kind = "finding"
title = "S2: Throwaway HTTP Clients — No Connection Reuse"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S2. Throwaway HTTP Clients — No Connection Reuse"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S2. Throwaway HTTP Clients — No Connection Reuse"
anchors = ["dispatch_direct.rs:372", "dispatch_direct.rs:290", "openai_compat_backend.rs:318", "model_call_service.rs:1047", "dispatch_direct.rs:74-107", "dispatch_direct.rs:371-389", "dispatch_direct.rs", "reqwest::Client::new()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 5 open checklist fixes: S2.1-2 Session-scoped client in `dispatch_direct; S2.3 Streaming path uses `self.poster` client; S2.4 Session-scoped agent reuse in ModelCallServ; S2.5 Config loaded once, cached; S2.6-7 Apply timeout config to client builder

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S2. Throwaway HTTP Clients — No Connection Reuse`

How to verify: Check each open sub-item (S2.1-2, S2.3, S2.4, S2.5, S2.6-7). Check dispatch_direct / ModelCallService for per-request reqwest::Client construction and config caching.
