# /prd-list Missing Slugs — Fixed + Actionable Hints Added

## Problem

`/prd-list` in Zed ACP shows PRD titles but no slugs. The slug is required by
`/enhance-prd <slug>`, `/prd-plan <slug>`, and `/prd-draft <slug>`, making `/prd-list`
useless as a discovery tool — you see items but can't act on them.

### Before
```
═══ Published PRDs ═══
  Dry-Run Flag: Workflow Execution Preview Without LLM Dispatch   coverage: —

═══ Drafts ═══
  Cursor Composer 2 ACP Backend     [draft]
  self-developing-workflow           [draft]
  test-quick                         [draft]
```

User sees titles but has to guess the slug or browse the filesystem.

## Root Cause

`prd.rs:683` prints title but not slug:
```rust
println!("  {:<35} coverage: {cov}", entry.title);  // slug not shown
```

`prd.rs:695` same for drafts:
```rust
println!("  {:<35} [draft]", entry.title);  // slug not shown
```

The `PrdEntry` struct (line 614) has a `slug` field — it's computed from the filename
stem in `read_prd_entry` (line 626-630) — but `cmd_list` never prints it.

## Fix Applied

**File:** `crates/roko-cli/src/prd.rs:667-719`

Changes:
1. Show `slug: <value>` next to each PRD and draft entry
2. Add an "Actions" section at the bottom with example commands using actual slugs
3. Only show coverage when non-zero (removes the useless `coverage: —` noise)

### After
```
═══ Published PRDs ═══
  Dry-Run Flag: Workflow Exe...  slug: dry-run-flag-workflow-...

═══ Drafts ═══
  Cursor Composer 2 ACP Backend   slug: cursor-composer-2-acp-backend
  self-developing-workflow         slug: self-developing-workflow
  test-quick                       slug: test-quick

═══ Ideas (5 captured) ═══
  - 2026-05-02 14:47 — Add a --dry-run flag ...

═══ Actions ═══
  /enhance-prd cursor-composer-2-acp-backend  Research & enrich a draft
  /prd-plan cursor-composer-2-acp-backend     Generate implementation plan from draft
  /prd-idea "<text>"                          Capture a new idea
```

Now users can copy slugs directly from the output into subsequent commands.

## Verification

- `cargo check -p roko-cli` — passes
- `cargo clippy -p roko-cli --lib --no-deps -- -D warnings` — clean, no warnings

## Files Modified

| File | Change |
|------|--------|
| `crates/roko-cli/src/prd.rs:667-719` | Show slugs + actionable hints in `cmd_list` |
