+++
id = "gap-b040f4"
kind = "gap"
title = "Examples: Add Core Workflow and Tasks.toml Authoring Examples"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/99-examples-documentation-gaps.md#99 — Examples: Add Core Workflow and Tasks.toml Authoring Examples"
discovered_from = "audit:tmp/backlog/archive/99-examples-documentation-gaps.md#99 — Examples: Add Core Workflow and Tasks.toml Authoring Examples"
anchors = ["examples/adding-a-provider.md", "roko.toml", "examples/adding-a-custom-protocol.md", "examples/adding-custom-tools.md", "examples/roko-*.toml", "examples/graphs/", "tasks.toml", "examples/plan-execution/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
developer experience; new contributors cannot learn the core self-hosting workflow by example. The `examples/` directory exists at `/Users/will/dev/nunchi/roko/roko/examples/` and contains useful but narrow content: provider configuration files (`roko-ollama.toml`, `roko-openrouter.toml`, etc.)…

Imported without verification from:
- `tmp/backlog/archive/99-examples-documentation-gaps.md#99 — Examples: Add Core Workflow and Tasks.toml Authoring Examples`

Some cited files are gone: `examples/roko-*.toml`.

How to verify: Check: `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/tasks.toml` exists and passes `roko plan validate examples/plan-execution/`.; `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/README.md` exists with prerequisites, run… [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): M | 7 |]
