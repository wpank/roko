+++
id = "gap-80b6d6"
kind = "gap"
title = "Named surfaces (E37): full TUI rendering and live runtime sources/command path"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui+roko-serve/surfaces"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/22-SURFACES.md:9"
discovered_from = "audit:docs/v3/22-SURFACES.md:9"
anchors = ["crates/roko-serve/src/routes/run.rs", "crates/roko-cli/src/tui/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
E37 contract/backend is 9/9 but rendering is partial: legacy TUI doesn't render every named surface, SurfaceEvents don't enter a command path (serve run.rs accepts them as 'P1-44 not yet wired'), no Inbox publisher/consumer, pending_human/autonomy store have no live source.

Imported without verification from:
- `docs/v3/22-SURFACES.md:9`
- `docs/v2/20-SURFACES.md:5`
- `docs/v3/39-ROADMAP.md#7.1 Full Named-Surface TUI Rendering`
- `crates/roko-serve/src/routes/run.rs:410`

How to verify: Inspect TUI tabs for Workbench/Inbox/Canvas/Minimap/Autonomy; POST a surface event and check handling.
