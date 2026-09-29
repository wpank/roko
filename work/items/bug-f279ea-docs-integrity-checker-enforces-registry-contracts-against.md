+++
id = "bug-f279ea"
kind = "bug"
title = "Docs integrity checker enforces registry contracts against deleted files"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["tooling/docs-integrity"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["tools/docs_integrity/check_markdown_links.py::check_status_disposition_registry", "tools/docs_integrity/check_markdown_links.py::check_source_coverage_registry", ".github/workflows/docs-lint.yml:79"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/docs_integrity/check_markdown_links.py && ! grep -q \"must select --engine runner-v2\" .github/workflows/docs-lint.yml"
+++

`tools/docs_integrity/check_markdown_links.py` enforces fixed-count registry contracts (exactly 109 files in `tmp/status-quo/`, fixed coverage-ledger and source-manifest counts), but `tmp/status-quo/` no longer exists and `tmp/` is gitignored, so no clone can satisfy them.
Decided 2026-09-28 (Will): drop the registry contracts rather than track the registries in git. Also drop the docs-lint rule that plan-run examples must select `--engine runner-v2`: that engine was deleted, so the four bare `roko plan run plans/` examples it flags in docs/v2 are correct. Keep the link, anchor and stale-phrase checks. Fix the two remaining link findings: README.md:12 should point at `work/STATUS.md`; docs/v2/ARCHITECTURE-GUIDE.md:22 has a dead anchor. Planned as `03-tooling-fixes` T04/T05, parked in `tmp/work-management/plans/work-graph/`.
