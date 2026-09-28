+++
id = "bug-1cb461"
kind = "bug"
title = "Aggregator truncates content by byte index and can panic on non-ASCII text"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/aggregator"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/aggregator.rs:455"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "content\[\.\.80\]" crates/roko-serve/src/routes/aggregator.rs'

[[verify]]
command = 'cargo test -p roko-serve routes::aggregator'
+++

`format!("{}…", &e.content[..80])` (`routes/aggregator.rs:455`) slices a `String` at byte 80.
If byte 80 falls inside a multi-byte UTF-8 character (accents, CJK, emoji) the slice panics and the request fails.
Fix: truncate on a char boundary (e.g. `char_indices().nth(80)`) and add a test with multi-byte content.
