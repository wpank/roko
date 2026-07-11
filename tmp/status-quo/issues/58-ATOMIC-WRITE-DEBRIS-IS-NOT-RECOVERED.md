# Atomic-write debris is not recovered

- Severity: medium

`.roko/learn` contains numerous abandoned `cascade-router.json.tmp.*` files, many zero-byte, plus `cascade-router.json.corrupted`. Successful startup/persistence does not clean, quarantine with metadata, or reconcile these artifacts.

Atomic writers should fsync, rename, clean their own stale temp files, and expose a clear recovery decision rather than accumulating ambiguous debris.

