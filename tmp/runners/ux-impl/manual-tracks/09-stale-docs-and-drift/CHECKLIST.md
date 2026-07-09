# DC — Stale docs and drift sweep (manual)

> Source plan: `tmp/ux/implementation-plans/09-stale-docs-and-drift.md`.
> Tracker rows: ISSUE-TRACKER.md Wave DC.

Doc-only. No compile risk. Done by hand because banned-term replacements
benefit from human review.

---

## DC01 — Banner sweep across `bardo-backup/tmp/roko-progress/*.md`

```bash
cd /Users/will/dev/nunchi/roko/roko
for f in bardo-backup/tmp/roko-progress/*.md; do
  date=$(git log -1 --format=%ad --date=short -- "$f" 2>/dev/null || echo "unknown")
  banner=$(printf '> ⚠ **Historical snapshot from %s.** Not kept in sync with the current code.\n> For current state, see `CLAUDE.md` and `tmp/ux/ux-followup/`.\n\n' "$date")
  if ! head -3 "$f" | grep -q 'Historical snapshot'; then
    printf '%s%s\n' "$banner" "$(cat "$f")" > "$f"
  fi
done
```

- [ ] Verify by reading 3 random files.
- [ ] Add `bardo-backup/README.md` with a directory-level archive banner.
- [ ] Tick `[ ] DC01`.

---

## DC02 — Replace `grimoire`/`styx`/`clade` outside `bardo-backup/`

```bash
rg --no-heading 'grimoire|styx|clade' \
  --glob '!bardo-backup/**' --glob '*.md' --glob '*.rs' --glob '*.toml' \
  CLAUDE.md README.md docs/ tmp/ crates/ apps/
```

For each hit:

| Banned | Replace with |
|--------|-------------|
| grimoire | neuro |
| styx | Korai |
| clade | fleet |

- [ ] Apply replacements per file, one commit per file.
- [ ] Verify the grep returns 0 hits afterward.
- [ ] Tick `[ ] DC02`.

---

## DC03 — Replace `mortal`/`death`/`reincarnation` in live docs

```bash
rg --no-heading -w 'mortal|death|reincarnation' \
  --glob '!bardo-backup/**' --glob '*.md' \
  CLAUDE.md README.md docs/ tmp/
```

| Banned | Replace with |
|--------|-------------|
| mortal | tier-bound |
| death | retirement |
| reincarnation | retirement and respawn |

**Exception**: `crates/roko-daimon/src/mortality.rs` is a real source
file. Either:

- [ ] Rename the module to `tier_progression.rs` (and update all
  imports + `mod` declarations in the same commit), OR
- [ ] Add a comment at the top of `mortality.rs` documenting the
  legacy name maps to "tier progression". Add to `docs/v2/RENAMES.md`.

Choose one. Tick `[ ] DC03`.

---

## DC04 — CI guard against banned terms in live `*.md`

Add `.github/workflows/banned-terms.yml`:

```yaml
name: banned-terms
on: [pull_request]
jobs:
  banned-terms:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check for banned terminology in live .md
        run: |
          set -e
          hits=$(rg --no-heading -w 'grimoire|styx|clade|reincarnation' \
            --glob '!bardo-backup/**' --glob '*.md' || true)
          if [[ -n "$hits" ]]; then
            echo "Banned terminology found:"
            echo "$hits"
            exit 1
          fi
```

- [ ] Test by intentionally re-introducing one banned term in a throwaway
  branch; confirm CI fails. Then revert.
- [ ] Tick `[ ] DC04`.

---

## DC05 — Add stale banner to `MORI-PARITY-CHECKLIST.md` (Path B)

The hand-maintained checklist references `apps/mori/*` paths that no
longer exist. Path A (auto-regenerate) is high-effort; Path B (banner)
is the recommended path.

- [ ] Prepend to `bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md`:

  ```markdown
  > ⚠ **Historical snapshot.** This file references `apps/mori/*` paths
  > that no longer exist. Canonical parity tracking lives at
  > `tmp/ux/ux-followup/00-INDEX.md`. Do not use this file as a
  > source of truth for current code state.
  ```

- [ ] Add a CLAUDE.md line under "Source-of-truth" pointing at
  `tmp/ux/ux-followup/00-INDEX.md` as canonical, marking
  `MORI-PARITY-CHECKLIST.md` as a frozen snapshot.

- [ ] Tick `[ ] DC05`.

---

## DC06 — Refresh `tmp/implementation-plans/00-INDEX.md` status

Open `tmp/implementation-plans/00-INDEX.md` (note: this is the *old*
index, not the new one created by this runner under
`tmp/ux/implementation-plans/`).

- [ ] Walk row by row. For each item closed by PR #13 or the 2026-04-20
  audit, change status to "DONE" with the date.
- [ ] Add at the top:

  ```markdown
  > Status refreshed YYYY-MM-DD against PR #13 + 2026-04-20 audit.
  ```

- [ ] If the entire file is now stale (every row done), retire it by
  prepending a `> Retired YYYY-MM-DD; see tmp/ux/implementation-plans/00-INDEX.md`
  banner.

- [ ] Tick `[ ] DC06`.

---

## DC07 — CLAUDE.md "What to work on" qualifiers

- [ ] For each item 1-9 in CLAUDE.md "What to work on" that's marked
  done but lacks a smoke test, replace the strikethrough with:

  ```markdown
  ~~1. Rust toolchain ready~~ — Done; smoke test pending (see Wave HY).
  ```

  (Where Wave HY = `tmp/runners/ux-impl/prompts/HY07.prompt.md` etc.)

- [ ] Once Wave HY closes, return and remove the qualifier (full
  strikethrough).

- [ ] Tick `[ ] DC07`.

---

## DC08 — Create `docs/v2/RENAMES.md`

```markdown
# Historical renames

| Old name | New name | Notes |
|----------|----------|-------|
| grimoire | neuro | runtime knowledge store |
| styx | Korai | inference layer |
| clade | fleet | agent grouping |
| mortal / death / reincarnation | tier progression / retirement / respawn | lifecycle vocabulary |
| bardo-* | roko-* | crate prefix; archive at `bardo-backup/` |
| apps/mori | crates/roko-cli/src/tui | TUI implementation |

`crates/roko-daimon/src/mortality.rs` retains the legacy name
internally for historical reasons; conceptually it's
`tier_progression.rs`.
```

- [ ] Create the file.
- [ ] Tick `[ ] DC08`.

---

## Close the followup items

- [ ] Mark items 45, 46, 47 in `tmp/ux/ux-followup/07-spec-code-drift.md`
  as DONE with this checklist as the closure reference.
- [ ] Mark items 64, 65, 66, 67, 67a in
  `tmp/ux/ux-followup/10-stale-docs.md` as DONE.
- [ ] Bump `tmp/ux/ux-followup/00-INDEX.md` totals.

---

## Done when

- [ ] `rg -w 'grimoire|styx|clade|reincarnation' --glob '!bardo-backup/**' --glob '*.md'` returns 0 hits.
- [ ] CI banned-terms guard exists and was demonstrated to fail on a
  re-introduction.
- [ ] CLAUDE.md qualifiers in place.
- [ ] `tmp/implementation-plans/00-INDEX.md` reflects PR #13 closure
  with the dated note.
- [ ] `MORI-PARITY-CHECKLIST.md` banner present.
- [ ] `docs/v2/RENAMES.md` exists.
- [ ] All ISSUE-TRACKER.md Wave DC rows ticked.
