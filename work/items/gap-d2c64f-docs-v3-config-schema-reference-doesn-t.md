+++
id = "gap-d2c64f"
kind = "gap"
title = "docs/v3 config schema reference doesn't document the [pricing] section"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK12 gap-08120e)"
discovered_from = "gap-08120e"
anchors = ["docs/v3/depth/21-config/01-schema-sections.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qi 'pricing' docs/v3/depth/21-config/01-schema-sections.md"
+++

## Problem

`crates/roko-core/src/config/schema.rs:131` defines a real, loaded config field,
`pub pricing: crate::pricing_snapshot::PricingConfig` (part of `RokoConfig`, defaulted at :454), but
`docs/v3/depth/21-config/01-schema-sections.md` — the per-section schema reference — has zero mentions of
"pricing" anywhere in the file. An operator reading the config schema docs has no way to learn that a
`[pricing]` section exists, what it controls, or how it relates to the dated snapshot files in `config/prices/`
(`crates/roko-core/src/pricing_snapshot.rs`).

## Why it matters

Config-schema docs that silently omit a real, live section are worse than no docs: a reader trusts the omission
as completeness. `[pricing]` controls cost accounting for every model call (see the related price-snapshot gap
filed separately from this same backlog wave), so getting it wrong or missing it entirely has real budget
consequences for anyone configuring roko from the docs alone.

## Where

- `docs/v3/depth/21-config/01-schema-sections.md` — the file that should document it, and currently doesn't.
- `crates/roko-core/src/config/schema.rs:131,454` — the real field and its default.
- `crates/roko-core/src/pricing_snapshot.rs::PricingConfig` — the type to document (its fields, defaults, and how
  it resolves to a dated snapshot file).

## Current state

Checked at HEAD (2026-10-02): confirmed via `grep -n pricing docs/v3/depth/21-config/01-schema-sections.md`
(zero matches) and `grep -n 'pub pricing' crates/roko-core/src/config/schema.rs` (one match, a live field).
`work/done/gap-08120e-*.md` (PK12's own package, which added a dated price-snapshot test) does not touch this
doc file either.

## Plan

1. Read `PricingConfig`'s actual fields in `pricing_snapshot.rs` and add a `[pricing]` subsection to
   `01-schema-sections.md` following the file's existing per-section format (field name, type, default,
   one-line meaning).
2. Cross-reference `config/prices/<date>.toml` and how the built-in snapshot id is chosen, so the doc explains
   where the numbers actually come from, not just the TOML key.

## Done when

- `01-schema-sections.md` documents every field of `[pricing]`.
- The `[[verify]]` command passes.

## Notes

- This is docs-only; don't change `schema.rs` or `pricing_snapshot.rs` to close this item — only to discover what
  to write.

2026-10-03 (wave-5 follow-up, PK13/gap-9e3134): the same file also has no mention of `stream_usage`
(`crates/roko-core/src/config/provider.rs:332`, `pub stream_usage: Option<bool>`, part of the per-provider config,
not `[pricing]`) — a second, separate omission in `01-schema-sections.md`. Document both in the same pass if
convenient, but they're independent: `[pricing]` is `PricingConfig`; `stream_usage` is a provider-level field.
