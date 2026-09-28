+++
id = "gap-85816b"
kind = "gap"
title = "Plugin WIT/Component-model hostcalls and OpenClaw/legacy one-shot parity are not implemented"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-plugin"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e32"
anchors = ["crates/roko-plugin/src"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

E32 is complete against its manifest: signed dependency graphs, bounded WASM hooks, strict admission, verified install and publish. Component-model Store/Bus hostcalls (WIT) and one-shot parity for OpenClaw and legacy adapters are separate roadmap work.

Fix: define the WIT world for Store/Bus hostcalls and port the OpenClaw and legacy one-shot plugins to it.
