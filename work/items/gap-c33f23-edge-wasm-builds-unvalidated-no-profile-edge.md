+++
id = "gap-c33f23"
kind = "gap"
title = "Edge & WASM builds unvalidated (no [profile.edge], no wasm-bindgen crate)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/wasm"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/32-deployment/edge-embedded.md:273"
discovered_from = "audit:docs/v3/depth/32-deployment/edge-embedded.md:273"
anchors = ["Cargo.toml [profile.*]", "roko-core wasm feature"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Edge deployment is P3: ~500KB binary target never validated, no [profile.edge]; WASM feature flags in roko-core/roko-std lack end-to-end WASM builds and the wasm-bindgen wrapper crate does not exist.

Imported without verification from:
- `docs/v3/depth/32-deployment/edge-embedded.md:273`
- `docs/v3/depth/32-deployment/wasm-browser-edge.md:271`

How to verify: cargo build -p roko-core --target wasm32-unknown-unknown --no-default-features; check workspace profiles.
