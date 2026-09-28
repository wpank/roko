+++
id = "bug-f279ea"
kind = "bug"
title = "Docs integrity checker enforces registry contracts against deleted files"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["tooling/docs-integrity"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["tools/docs_integrity/check_markdown_links.py"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'python3 tools/docs_integrity/check_markdown_links.py'
+++

`tools/docs_integrity/check_markdown_links.py` enforces fixed-count registry contracts (exactly 109 files in `tmp/status-quo/`, fixed coverage-ledger and source-manifest counts), but `tmp/status-quo/` no longer exists and `tmp/` is gitignored, so no clone can satisfy them.
Fix: drop those contracts or move their registries into tracked files; keep the link, anchor and INDEX checks.
