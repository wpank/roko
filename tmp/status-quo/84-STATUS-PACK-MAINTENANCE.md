# Status Pack Maintenance

This file describes how to keep `tmp/status-quo` useful after code/docs continue moving. The pack is a reconciliation layer, so it must be cheap to validate and regenerate.

## Generated Files

| File | Source | Refresh when |
|---|---|---|
| `80-SOURCE-DOC-MANIFEST.md` | `docs/v1`, `docs/v2`, `docs/v2-depth` markdown files. | Any source docs are added, deleted, moved, or archived. |
| `83-ENV-VAR-MANIFEST.md` | Direct env reads, Clap `env =`, and build-time env declarations under `crates/`, `apps/`, `demo/`. | Any `std::env::var`, `env::var`, `var_os`, or Clap env binding is added. |

## Validation Commands

```sh
find tmp/status-quo -maxdepth 1 -type f -name '*.md' | wc -l
wc -l tmp/status-quo/*.md | tail -n 1
ruby -e 'missing=[]; Dir["tmp/status-quo/*.md"].each do |f|; text=File.read(f); text.scan(/\\[[^\\]]+\\]\\(([^)]+\\.md)(?:#[^)]+)?\\)/).flatten.each do |href|; next if href.start_with?("http"); path=File.expand_path(href, File.dirname(f)); missing << [f, href] unless File.exist?(path); end; end; if missing.empty?; puts "all local markdown links resolve"; else; missing.each { |f,h| puts "#{f}: #{h}" }; exit 1; end'
rg -n 'docs/v1` \\| 585|docs/v2-depth` \\| 217|runner-v2 is default|Runner-v2 is default|defaults to \\*\\*runner-v2\\*\\*|permissive-default|no auto-trigger|stubs only|19 builtin tools|completely unwired|Production Ready|Working for local development' tmp/status-quo
```

The stale-phrase scan should usually return no results. When it returns a historical quote that should remain, the nearby text must explicitly mark it as historical/stale.

## Regeneration Requirements

- Regenerating `80` must preserve the counts in `72`, `00`, and `65`.
- Regenerating `83` must preserve the trust-boundary links in `61`, `75`, and `77`.
- Adding a new status doc must update `00-INDEX.md` and, if it affects work priority, `12`, `24`, and `25`.
- Adding a new generated manifest must add a maintenance row here.
- Adding a new stale-claim category must add it to the validation scan above.

## Review Checklist

- [ ] New docs have a clear owner and are linked from `00-INDEX.md`.
- [ ] Roadmap-impacting findings are copied into `12-ROADMAP.md` or `24-OPEN-ISSUE-LEDGER.md`.
- [ ] Proof-impacting findings are copied into `25-PROOF-GATES.md`.
- [ ] Source-doc changes are reflected in `80-SOURCE-DOC-MANIFEST.md`.
- [ ] Env/config changes are reflected in `83-ENV-VAR-MANIFEST.md`.
- [ ] Local markdown links resolve.
- [ ] Targeted stale phrase scan is clean or every hit is explicitly labeled historical.
