+++
id = "gap-3022ca"
kind = "gap"
title = "Dream scheduling lacks bus-reactive and intensive-backlog triggers"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-dreams/scheduling"]
created = 2026-08-15
updated = 2026-09-28
source = "gaps-md#dream-automatic-scheduling----resolved-2026-08-15"
anchors = ["crates/roko-dreams/src/runner.rs:435"]
links = { depends_on = [], blocks = [], related = ["gap-4d1bff"], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

Daemon mode runs `DreamSchedulePolicy` (`crates/roko-dreams/src/runner.rs:435`), which supports cron with idle queuing, episode-count and adaptive-idle triggers, and checkpoint restore. Two trigger kinds were left as future refinements: bus-reactive scheduling (dream on specific events) and intensive-backlog scheduling (catch up when unconsolidated episodes pile up). That serve never starts its resident scheduler is tracked separately (gap-4d1bff).

Fix: add both trigger kinds to the policy, with tests.
