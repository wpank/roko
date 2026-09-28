+++
id = "gap-4f8af2"
kind = "gap"
title = "Attention-bidder feedback is clean-shutdown-only and lacks provider-cost attribution"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/dispatch", "roko-compose/bidders"]
created = 2026-08-16
updated = 2026-09-28
source = "gaps-md#runner-vcg-feedback-loop----resolved-2026-08-16"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/dispatch/factory.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

VCG/density-greedy composition and bidder posteriors are recorded as resolved. Bidder state is saved only on clean shutdown rather than through an attempt receipt, so a crash after a terminal gate loses the newest observation. Exact provider cost is not fed into bidder updates. The attention-bidder code now lives in `crates/roko-cli/src/dispatch/factory.rs` and `dispatch/prompt_builder.rs`. Backlog #380 (bidder persistence loop) is archived without a status. Whether the Graph path updates bidders at all has not been re-checked.

Fix: persist bidder updates through the attempt receipt or feedback facade, and attribute actual provider cost to each included section.
