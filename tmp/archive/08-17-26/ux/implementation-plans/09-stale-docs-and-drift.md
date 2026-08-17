# 09 — Stale Docs and Spec/Code Drift Sweep

> **Source plans**: `tmp/ux/ux-followup/07-spec-code-drift.md` items 45,
> 46, 47; `tmp/ux/ux-followup/10-stale-docs.md` items 64, 65, 66, 67, 67a.
>
> **Status as of 2026-05-01**: Nine doc-sweeping items remain. None block
> code; all mislead newcomers. Three are P1 sweeps (terminology, paths,
> banners) that take 1-2 hours each. Two are larger (CLAUDE.md smoke
> tests, MORI-PARITY checklist regen).
>
> **Effort**: 1-2 days end-to-end.
>
> **Risk**: Low. Doc-only modifications; no code paths change.

---

## What this plan accomplishes

Bring user-facing documentation back in sync with the code. After this
plan:

- `bardo-backup/tmp/roko-progress/*.md` files have a "stale snapshot"
  banner so newcomers don't read them as live (items 47, 64).
- Old terminology (`grimoire`, `styx`, `clade`, `mortal`, `death`,
  `reincarnation`) does not appear in live docs (items 65, 66). The
  archive in `bardo-backup/` is left alone.
- `MORI-PARITY-CHECKLIST.md` either has a fresh mechanically-derived
  percentage or a banner declaring it a frozen archive (items 46, 67).
- `tmp/implementation-plans/00-INDEX.md` no longer marks PR #13 work as
  "pending" / "in-flight" (item 67a).
- CLAUDE.md "What to work on" items 1-9 carry honest "Done — smoke test
  pending" qualifiers OR have smoke tests added (item 45).
- A small CI guard prevents future re-introduction of the banned
  terminology in live docs.

## Why this matters

Newcomers (and AI agents) take CLAUDE.md and the `tmp/` tree at face
value. False "Done" markers and stale terminology cost real time when
contributors chase ghosts.

---

## Required reading

```
CLAUDE.md
README.md
bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md
bardo-backup/tmp/roko-progress/                  (rest of files)
tmp/implementation-plans/00-INDEX.md
tmp/ux/ux-followup/07-spec-code-drift.md
tmp/ux/ux-followup/10-stale-docs.md
crates/roko-daimon/                              (mortality.rs reference)
```

Memory references (informational): the user's
`~/.claude/projects/.../memory/feedback_naming_conventions.md` records
the canonical renames (`grimoire → neuro`, `styx → Korai`, `clade → fleet`,
"death concepts removed").

---

## Deliverables

### Banner sweep (items 47, 64)

1. Prepend to **every** `bardo-backup/tmp/roko-progress/*.md` file:

   ```markdown
   > ⚠ **Historical snapshot from <YYYY-MM-DD>.** Not kept in sync with
   > the current code. For current state, see `CLAUDE.md` and
   > `tmp/ux/ux-followup/`. The archive directory `bardo-backup/`
   > captures pre-roko (bardo-era) progress and is read-only.
   ```

   Use the file's git-mtime as the date; one date per file.

2. Add a top-level `bardo-backup/README.md` with a banner explaining
   the directory is archive material.

### Terminology sweep (items 65, 66)

1. Run a pre-flight grep:

   ```bash
   rg --no-heading 'grimoire|styx|clade|reincarnation' \
     --glob '!bardo-backup/**' --glob '*.md' --glob '*.rs' --glob '*.toml' \
     CLAUDE.md README.md docs/ tmp/ crates/ apps/

   rg --no-heading -w 'mortal|death' \
     --glob '!bardo-backup/**' --glob '*.md' \
     CLAUDE.md README.md docs/ tmp/

   rg --no-heading 'mortality' \
     --glob '!bardo-backup/**' --glob '*.rs' \
     crates/ apps/
   ```

2. For each hit:

   | Banned | Replace with |
   |--------|-------------|
   | grimoire | neuro |
   | styx | Korai |
   | clade | fleet |
   | reincarnation | retirement and respawn |
   | mortal | tier-bound |
   | death | retirement |

3. **Important exception**: `crates/roko-daimon/src/mortality.rs` is a
   real source file. Either:
   - Rename to `tier_progression.rs` plus equivalent identifier renames
     in the same PR, or
   - Document the legacy name with a `// Note: …` block explaining the
     historical naming.
   Do not silently sed-replace `mortality` in source code.

4. Add a CI guard in `.github/workflows/`:

   ```yaml
   - name: Banned terminology in live docs
     run: |
       set -e
       hits=$(rg --no-heading -w 'grimoire|styx|clade|reincarnation' \
         --glob '!bardo-backup/**' --glob '*.md' || true)
       if [[ -n "$hits" ]]; then
         echo "Banned terminology found:"; echo "$hits"; exit 1
       fi
   ```

### MORI-PARITY freshness (items 46, 67)

Two paths; pick one:

**Path A (regenerate)**:

1. Write `tmp/scripts/regenerate-mori-parity.sh` that walks the
   1 253-item checklist and for each item greps the current codebase
   for the call-site/symbol it claims. Output a Markdown checklist
   with ✓ / ✗ / ? per line.
2. Replace `bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md`
   with the freshly-generated version (copy the original to a sibling
   with `.snapshot.md`).
3. Document the regen process in the file header.

**Path B (banner only)**:

1. Add the banner from item 64 above with extra emphasis on path
   staleness ("references `apps/mori/*` paths that no longer exist").
2. Document in `CLAUDE.md` that the canonical parity tracking lives in
   `tmp/ux/ux-followup/00-INDEX.md`.

Recommendation: **Path B**. The hand-maintained percentage was already
unreliable; replacing it with a freshly-derived one is high effort and
the result is still a snapshot. A banner is a smaller commitment.

### `tmp/implementation-plans/00-INDEX.md` refresh (item 67a)

1. Read the index. List items currently marked pending / in-flight.
2. Cross-reference against `tmp/ux/ux-followup/00-INDEX.md` 2026-04-20
   re-audit and the post-PR-13 delta.
3. Update each row's status. Add a line at the top:

   ```markdown
   > Status refreshed YYYY-MM-DD against PR #13 + 2026-04-20 audit.
   ```

### CLAUDE.md smoke-test qualifier (item 45)

CLAUDE.md "What to work on" items 1-9 are struck-through ("Done"). The
audit notes: "Wired but no integration tests confirm end-to-end
semantics."

Two paths:

**Path A (smoke tests)**: write the 9 missing smoke tests. Half a day
each on average; ~5 days total. **Out of scope for this plan** — folded
into the "Done when" of plan `10` (hygiene / coverage).

**Path B (qualifier)**: change strikethrough text to:

```markdown
~~1. Rust toolchain ready~~ — Done; smoke test pending (see plan 10).
```

Apply Path B in this plan as a stop-gap. Plan `10` upgrades to Path A
when the smoke tests land.

---

## Step-by-step

### Step 1 — Banner sweep (1 hr)

```bash
# For each file under bardo-backup/tmp/roko-progress/:
for f in bardo-backup/tmp/roko-progress/*.md; do
  date=$(git log -1 --format=%ad --date=short -- "$f" 2>/dev/null || echo "unknown")
  banner=$(printf '> ⚠ **Historical snapshot from %s.** Not kept in sync with the current code.\n> For current state, see `CLAUDE.md` and `tmp/ux/ux-followup/`.\n\n' "$date")
  # Skip if banner already present.
  if ! head -3 "$f" | grep -q 'Historical snapshot'; then
    printf '%s%s\n' "$banner" "$(cat "$f")" > "$f"
  fi
done
```

(Run from the workspace root; do not run inside `bardo-backup/`.)

Verify by reading three random files.

### Step 2 — Terminology sweep (2 hrs)

Run the pre-flight grep above. For each hit, decide rename or annotate.
Apply changes per file in separate commits so a regression is easy to
revert.

For `crates/roko-daimon/src/mortality.rs`: open the file. If its public
API is exported, rename the module (and the file) plus update all
imports. If it's internal-only, prefer the comment annotation:

```rust
// Note: this module is named `mortality` for historical reasons; the
// concept is "tier progression" in current terminology. See
// docs/v2/RENAMES.md.
```

Add `docs/v2/RENAMES.md` recording all historical → canonical mappings.

### Step 3 — CI guard (30 min)

Add the workflow step from Deliverable 2.4. Test by intentionally
re-introducing one banned term in a throwaway branch; confirm CI fails.
Then revert.

### Step 4 — MORI-PARITY (15 min, banner path)

Add the banner. Add a note to CLAUDE.md "Source-of-truth" section:

```markdown
The canonical parity tracking lives at `tmp/ux/ux-followup/00-INDEX.md`.
The historical `MORI-PARITY-CHECKLIST.md` (under `bardo-backup/`) is a
frozen snapshot.
```

### Step 5 — Implementation plans index refresh (30 min)

Open `tmp/implementation-plans/00-INDEX.md`. Walk row by row. For each
that maps to a closed item in `tmp/ux/ux-followup/00-INDEX.md`, update
the status. Add the dated note at the top.

If the index is heavily stale and all the work is captured in
`tmp/ux/ux-followup/00-INDEX.md`, retire `tmp/implementation-plans/`
entirely and link forward instead. Decide based on read-time.

### Step 6 — CLAUDE.md qualifier (30 min)

Edit "What to work on". Add the "smoke test pending" qualifier per
Path B above. Plan `10` (next) takes ownership of converting these to
real tests.

### Step 7 — Close the followup items (5 min)

Mark items 45, 46, 47, 64, 65, 66, 67, 67a in the followup catalogue
`00-INDEX.md` totals. Add the date and a one-line explanation per item
in the source files (`07-spec-code-drift.md`, `10-stale-docs.md`).

---

## Anti-patterns to avoid

- **Don't sed-replace identifiers in source code.** A rename in
  `mortality.rs` requires `cargo build` to pass; a doc rename does
  not. Mixing the two in a doc-only PR risks subtle compile breakage.
- **Don't add the banner to `bardo-backup/` files that already have
  one.** Idempotent loops are easy to mis-write; the
  `head -3 | grep 'Historical snapshot'` guard above is the contract.
- **Don't widen the CI guard to grep the entire repo.** Source code
  legitimately uses `mortality` (real type name in `daimon`). Restrict
  to `*.md` outside `bardo-backup/`.
- **Don't try to regenerate `MORI-PARITY-CHECKLIST.md`** unless someone
  takes ongoing ownership. A one-shot regen rots in two weeks.
- **Don't update CLAUDE.md "What to work on" items to fully un-struck**
  unless smoke tests have actually landed. Path B's qualifier is the
  honest status until plan `10`.
- **Don't refactor docs and source code in the same PR.** Doc PRs get
  a different review pace; mixing them invites a long-lived branch.

## Done when

1. Every file under `bardo-backup/tmp/roko-progress/` has a stale
   banner.
2. `rg -w 'grimoire|styx|clade|reincarnation' --glob '!bardo-backup/**'
   --glob '*.md'` returns 0 hits.
3. `rg -w 'mortal|death' --glob '!bardo-backup/**' --glob '*.md'`
   returns 0 hits (reincarnation in `--*-test-fixtures*` is fine if
   present; document case by case).
4. The CI guard fails when a banned term is re-introduced in a `.md`
   file.
5. CLAUDE.md "What to work on" items 1-9 either pass smoke tests or
   carry the explicit "smoke test pending" qualifier.
6. `tmp/implementation-plans/00-INDEX.md` reflects PR #13 closure with
   the dated note.
7. `MORI-PARITY-CHECKLIST.md` carries the stale-snapshot banner.
8. `docs/v2/RENAMES.md` exists.
9. `tmp/ux/ux-followup/07-spec-code-drift.md` items 45, 46, 47 and
   `10-stale-docs.md` items 64-67, 67a are marked DONE in their files
   and the followup `00-INDEX.md`.
