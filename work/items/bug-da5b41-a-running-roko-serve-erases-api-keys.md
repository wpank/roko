+++
id = "bug-da5b41"
kind = "bug"
title = "A running roko serve erases API keys created by the CLI"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/auth.rs:207", "crates/roko-serve/src/routes/auth.rs:237"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve routes::auth'
+++

The serve key store loads `.roko/api-keys.json` once (`routes/auth.rs:207`), and every later mutation persists its whole in-memory list (`persist_registry_file(&api_keys_path(..), &updated)` at `:237`, `:249`, `:284`, `:296`).
A key created with the CLI while serve runs is overwritten by the next server-side write, locking that client out.
Fix: make every server-side write a locked read-merge-write of a single delta against the file on disk, and test that an out-of-band key survives a server write.
