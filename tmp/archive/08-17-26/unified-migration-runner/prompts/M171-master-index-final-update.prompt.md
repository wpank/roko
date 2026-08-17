# M171 — Final Reconciliation of Master Index

## Objective
Final reconciliation of `tmp/unified-depth/INDEX.md`. Verify that all source documents referenced in the unified migration are marked as either Absorbed or explicitly Deferred. Update the Coverage Summary table with final counts. Verify all depth doc filenames referenced in runner prompts (M001–M171) match actual files on disk. Generate a machine-readable coverage report as JSON. This is a documentation-only meta-task with no code changes.

## Scope
- Crates: none (documentation only)
- Files:
  - `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md` (primary — update)
  - `/Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/prompts/` (all M*.prompt.md — read for depth_doc refs)
  - `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/` (all subdirectories — verify files exist)
  - `/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/COVERAGE.json` (new — machine-readable report)
- Depth doc: N/A (meta-task)

## Steps
1. Read the current INDEX.md to understand its structure:
   ```bash
   head -80 /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md
   ```

2. Collect all depth doc references from runner prompts:
   ```bash
   grep -h 'Depth doc:' /Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/prompts/M*.prompt.md | sort | uniq
   ```

3. Verify each referenced depth doc exists on disk:
   ```bash
   grep -oh 'tmp/unified-depth/[^ ]*\.md' /Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/prompts/M*.prompt.md | sort -u | while read f; do
       [ -f "/Users/will/dev/nunchi/roko/roko/$f" ] && echo "OK: $f" || echo "MISSING: $f"
   done
   ```

4. Count all depth docs actually on disk:
   ```bash
   find /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/ -name '*.md' -not -name 'INDEX.md' | wc -l
   find /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/ -name '*.md' -not -name 'INDEX.md' | sort
   ```

5. Count all runner prompts:
   ```bash
   ls /Users/will/dev/nunchi/roko/roko/tmp/unified-migration-runner/prompts/M*.prompt.md | wc -l
   ```

6. Check INDEX.md for Absorbed/Deferred markers:
   ```bash
   grep -c 'Absorbed\|absorbed\|ABSORBED' /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md
   grep -c 'Deferred\|deferred\|DEFERRED' /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md
   grep -c '^\|' /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md  # Table rows
   ```

7. Update INDEX.md Coverage Summary table:
   - Total source documents tracked
   - Number marked Absorbed
   - Number marked Deferred
   - Number of depth docs produced
   - Number of runner prompts produced (M001–M171)
   - Any source docs with no status (these need resolution)

8. Generate `COVERAGE.json`:
   ```json
   {
       "generated_at": "2026-04-26T...",
       "total_source_docs": N,
       "absorbed": N,
       "deferred": N,
       "unresolved": N,
       "depth_docs_on_disk": N,
       "runner_prompts": 171,
       "depth_doc_refs_valid": N,
       "depth_doc_refs_missing": N,
       "missing_refs": ["path/to/missing.md", ...]
   }
   ```

9. For any MISSING depth doc references found in step 3:
   - If the depth doc topic is covered by another file (renamed), update the INDEX mapping
   - If the depth doc was never written, mark it as Deferred with a note
   - Do NOT create placeholder depth docs — just document the gap

10. Final sanity check — ensure no duplicate entries in INDEX.md:
    ```bash
    grep '|' /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md | awk -F'|' '{print $2}' | sort | uniq -d
    ```

## Verification
```bash
# No cargo commands — documentation only
# Verify INDEX.md is valid markdown
head -5 /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md
# Verify COVERAGE.json is valid JSON
python3 -c "import json; json.load(open('/Users/will/dev/nunchi/roko/roko/tmp/unified-depth/COVERAGE.json'))"
# Verify no broken internal links
grep -oh 'tmp/unified-depth/[^ )]*\.md' /Users/will/dev/nunchi/roko/roko/tmp/unified-depth/INDEX.md | while read f; do
    [ -f "/Users/will/dev/nunchi/roko/roko/$f" ] || echo "BROKEN: $f"
done
```

## What NOT to do
- Do NOT create placeholder depth docs to fill gaps — document them as Deferred
- Do NOT modify any runner prompt files — this is read-only analysis of them
- Do NOT modify any Rust code — this is purely documentation
- Do NOT change the INDEX.md format — only update content within existing structure
- Do NOT remove Deferred items — they are intentionally deferred to future phases
