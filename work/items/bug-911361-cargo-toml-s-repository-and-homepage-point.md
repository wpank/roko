+++
id = "bug-911361"
kind = "bug"
title = "Cargo.toml's repository and homepage point at an unrelated GitHub account"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["workspace/cargo"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["Cargo.toml:106", "Cargo.toml:107"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q \"github.com/nunchi/roko\" Cargo.toml"
+++
`repository` and `homepage` are `https://github.com/nunchi/roko`. `nunchi` is an unrelated person's account; roko lives at `github.com/wpank/roko` (Nous decision D-02 recommends keeping `wpank`). Every crate inherits these URLs, so published metadata and docs links point at the wrong owner.
