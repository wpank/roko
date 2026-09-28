+++
id = "gap-5ee40c"
kind = "gap"
title = "Environment variable registry: call sites not migrated to shared parsers; no generated reference or CI check"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "gaps-md#from-cli-audit-tmpcli-auditsummarymd/env-vars"
anchors = ["crates/roko-core/src/config/env_registry.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The env registry (`crates/roko-core/src/config/env_registry.rs`, 105+ variables, exposed by `roko config env list`) and its shared parsers exist; backlog #339 is done. Remaining work:
- migrate consumer call sites to the shared parsers and alias-deprecation helpers;
- generate the environment reference doc from the registry;
- add a CI check that compares `std::env::var` call sites against the registry.
