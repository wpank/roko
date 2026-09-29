+++
id = "q-bc6cc5"
kind = "question"
title = "Confirm licence coverage for another contributor's code in apps/mirage-rs"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["release/licensing"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "work:gap-ae2f55"
anchors = ["apps/mirage-rs/src/precompiles/hdc.rs", "apps/mirage-rs/src/rpc.rs"]
links = { depends_on = [], blocks = [], related = ["gap-ae2f55"], supersedes = [], duplicate_of = "" }
+++
roko is now licensed MIT OR Apache-2.0 by Will Pankiewicz. `git blame` at HEAD (2026-09-28) attributes 1,357 surviving lines to another contributor, JaeLeex (commits from 2026-04-16 to 04-23). They are almost all in `apps/mirage-rs`: `precompiles/hdc.rs` (839), `rpc.rs` (320), `provider.rs` (105), `fork.rs` (47) and a few others, plus single lines in roko-serve and roko-demo. None of simp-son's 5 lines survive.

Settle one of:
- the contributor agrees to the licence;
- the code was work for hire or otherwise already covered;
- carve out `apps/mirage-rs`, or rewrite those lines.

Record the outcome here.
