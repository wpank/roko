+++
id = "bug-f279ea"
kind = "bug"
title = "Docs integrity checker enforces registry contracts against deleted files"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["tooling/docs-integrity"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a52b528c9"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["tools/docs_integrity/check_markdown_links.py::check_status_disposition_registry", "tools/docs_integrity/check_markdown_links.py::check_source_coverage_registry", ".github/workflows/docs-lint.yml:79"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/docs_integrity/check_markdown_links.py && ! grep -q \"must select --engine runner-v2\" .github/workflows/docs-lint.yml"

[closed]
at = 2026-09-29
commit = "a52b528c9"
by = "wk-ci"
evidence = "a52b528c9 removed check_status_disposition_registry, check_source_coverage_registry and their registry-only helpers and tests from tools/docs_integrity, dropped the tmp/status-quo path filters from docs-lint.yml, and fixed the dead section-9 anchor in docs/v2/ARCHITECTURE-GUIDE.md:22. The runner-v2 rule and the README link had already been fixed by bug-09690f. Proof: the item verify passes (check_markdown_links.py reports no findings, no runner-v2 rule), python3 -m unittest tools.docs_integrity.test_check_markdown_links runs 12 tests OK, and docs-lint.yml's stale-phrase and executor.json grep step passes locally."
+++

`tools/docs_integrity/check_markdown_links.py` enforces fixed-count registry contracts (exactly 109 files in `tmp/status-quo/`, fixed coverage-ledger and source-manifest counts), but `tmp/status-quo/` no longer exists and `tmp/` is gitignored, so no clone can satisfy them.
Decided 2026-09-28 (Will): drop the registry contracts rather than track the registries in git. Also drop the docs-lint rule that plan-run examples must select `--engine runner-v2`: that engine was deleted, so the four bare `roko plan run plans/` examples it flags in docs/v2 are correct. Keep the link, anchor and stale-phrase checks. Fix the two remaining link findings: README.md:12 should point at `work/STATUS.md`; docs/v2/ARCHITECTURE-GUIDE.md:22 has a dead anchor. Planned as `03-tooling-fixes` T04/T05, parked in `tmp/work-management/plans/work-graph/`.
