+++
id = "find-12836d"
kind = "finding"
title = "[running #5] Stale incremental-compilation artifacts cause arm64 linker errors"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["ci/build"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#5. Incremental compilation linker errors"
discovered_from = "audit:tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#5. Incremental compilation linker errors"
anchors = ["CARGO_INCREMENTAL", "crates/roko-cli"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Initial cargo build failed with 'Undefined symbols for architecture arm64' from stale incremental artifacts; required cargo clean -p roko-cli. Suggested mitigation CARGO_INCREMENTAL=0 for CI builds (not applied).

Imported without verification from:
- `tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#5. Incremental compilation linker errors`

How to verify: Check CI workflow env for CARGO_INCREMENTAL=0.
