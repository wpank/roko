+++
id = "bug-661796"
kind = "bug"
title = "The route-inventory doc still cites 376 canonical routes; the tree has 402"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bc5d93e79"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d1dea8"
anchors = ["docs/v3/depth/26-http/01-route-inventory.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["bug-d1dea8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/v3/depth/26-http/01-route-inventory.md && ! grep -nE '376|421' docs/v3/depth/26-http/01-route-inventory.md && grep -q 'http_route_inventory.snapshot.json' docs/v3/depth/26-http/01-route-inventory.md && ! git grep -n '01-route-inventory-376' -- docs/v3 && python3 tools/docs_integrity/check_markdown_links.py docs/v3/depth/26-http"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:08:04Z"
commit = "bc5d93e79"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T19:02:24Z"
forced = false
evidence = "Renamed docs/v3/depth/26-http/01-route-inventory-376.md to 01-route-inventory.md; it no longer quotes a route count and points at tools/http_route_inventory.py and tools/http_route_inventory.snapshot.json (CI --check-snapshot); nav updated; the 26-http depth pages' chapter link fixed. The verify (renamed page, no 376/421, snapshot named, no links to the old name, check_markdown_links.py docs/v3/depth/26-http) passes at bc5d93e79."
+++

## Problem

bug-d1dea8 refreshed `tools/http_route_inventory.snapshot.json` (453 registrations, 402 canonical) and added a CI check. `docs/v3/depth/26-http/01-route-inventory-376.md` still cites about 376 canonical routes.

## Plan

Regenerate the doc's numbers from the tool, or point the doc at the snapshot. Rename the file if its name carries the count, and fix the links.

## Done when

- The verify passes, and docs link checks pass.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on bug-d1dea8, during the evening close-out round.
- 2026-10-01 (wk-climain): renamed the page to `docs/v3/depth/26-http/01-route-inventory.md` and dropped the counts:
  its Counting Methodology section now points at `tools/http_route_inventory.py` and the committed
  `tools/http_route_inventory.snapshot.json` (`total_registrations`, `canonical_registrations`), which CI checks with
  `--check-snapshot`. The VitePress nav entry follows the rename. The five depth pages' "Depth file for" link pointed
  at a missing `../../26-HTTP.md`; it now points at `26-HTTP-API.md`. The old verify grepped the old path, which no
  longer exists, so it now checks the renamed page, the old name's links and the Markdown link checker.
- Other docs still quote "~376 canonical routes (~421 incl. aliases)": `docs/v3/26-HTTP-API.md:3`,
  `docs/v3/00-INDEX.md:467,879`, `docs/v3/28-CLI.md:1691`, `docs/v3/32-DEPLOYMENT.md:4,67`, two VitePress components,
  and older v1/v2 status lines. Not changed here.
