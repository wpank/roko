+++
id = "gap-702150"
kind = "gap"
title = "Artifact marketplace HTTP/CLI are stubs; no durable storage, publish/install pipeline or anchoring"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain/marketplace", "roko-serve/marketplace"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e38"
anchors = ["crates/roko-chain/src/marketplace.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

E38 delivered the artifact, package, publish, economics, fork and capability contracts. The serve routes return structured 501 responses and the seven `roko market` commands are stubs. Still product work: durable storage and search, executable publish/install, ratings and indexing, and ERC-8004 anchoring. The job-market logic and local fork/economics contracts in `crates/roko-chain/src/marketplace.rs` have no durable or on-chain adapter.

Fix: build a durable local marketplace store behind the existing routes before adding any on-chain anchoring.
