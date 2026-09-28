+++
id = "spec-58ca20"
kind = "spec"
title = "Domain profiles as single installable TOML bundle (proposed)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/lifecycle"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/05-agent/16-domain-profiles.md:3"
discovered_from = "audit:docs/v3/depth/05-agent/16-domain-profiles.md:3"
anchors = ["crates/roko-agent/src/lifecycle.rs::DomainPlugin"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Domain profile concept is Proposed: extension points and DomainPlugin/lifecycle manifests exist in roko-agent lifecycle.rs, but installing a full profile as one TOML bundle is not wired.

Imported without verification from:
- `docs/v3/depth/05-agent/16-domain-profiles.md:3`
- `docs/v3/depth/05-agent/domain-profiles.md:3`

How to verify: Check CLI for a profile install command; compare with config profiles (coding/research/review).
