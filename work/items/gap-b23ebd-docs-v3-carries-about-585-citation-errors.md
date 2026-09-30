+++
id = "gap-b23ebd"
kind = "gap"
title = "docs/v3 carries about 585 citation errors; fix them before publishing the docs"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["docs"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4cf2e329b"
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["docs/v3/", "tools/docs_integrity/check_citation_errata.py", "tools/docs_integrity/citation_errata.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/docs_integrity/check_citation_errata.py && python3 -m unittest tools.docs_integrity.test_check_citation_errata"
+++
The companion audit checked the design docs' citations. An estimated 3.0% are fabricated (CI 1.2–5.1%), 12.0% have major errors and 38.7% have some error. 585 errata locations are in `docs/v3`, the tree the Nous plan would publish on GitHub Pages (NB2-006). The errata list is `tmp/cybernetic-harness/companion-audit/CITATION-ERRATA.md`, in the gitignored workspace.

Do not edit `docs/` until the companion audit's evidence has been re-anchored (Track E, E1). The 2026-09-28 evidence snapshot is in `roko-evidence/audit-2026-09-28/`, next to the repo.

## Notes

- 2026-09-30 (wk-rp-cite): the errata are bibliographic (fabricated works, wrong arXiv IDs, wrong or placeholder
  authors, invented titles, wrong years), not code references. E1 had been re-derived (gap-cdd5f4, tag
  `audit/baseline-2026-09-28`), so the docs could change.
- `tools/docs_integrity/citation_errata.json` holds the audit's 241 adjudicated major and minor errata: what the docs
  wrote wrong and the correct work. `check_citation_errata.py` reports each one still written in `docs/v3`. It found
  649 at `4cf2e329b` (215 works in 49 files) and none after the fix. The 47 mentions of 24 fabricated works were
  removed, except two Polya-urn references, which now cite the real paper (Tria et al. 2014, doi:10.1038/srep05890).
  The rest were corrected from the audit's registry data; the manifest's `notes` list the corrections made by hand.
- Left for follow-up: the audit adjudicated a sample (483 of 873 works), so unadjudicated citations were not checked;
  prose that describes a paper from its invented title (the routing sections of `20-GATEWAY.md` and
  `depth/20-gateway/06-routing-research.md`, the CLEAR framework in `depth/06-composition/`) needs a content review;
  and the checker is not yet in the docs-lint workflow.
