# M168 — Create Machine-Readable Academic Reference Index

## Objective
Create a machine-readable academic reference index at `.roko/references.json` by parsing all 27 reference documents in `docs/21-references/`. Each document contains multiple citations organized by topic. Parse these into structured JSON so that the `roko explain` command can cite academic foundations when explaining concepts, and agents can look up relevant papers when planning implementations.

## Scope
- Crates: `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/references.rs` (new — parser + index builder)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (wire module)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/explain.rs` (wire citation lookup)
  - `/Users/will/dev/nunchi/roko/roko/docs/21-references/` (source documents, read-only)
- Depth doc: `tmp/unified-depth/21-roadmap/07-academic-foundations-by-protocol.md`

## Steps
1. Survey the reference documents to understand their structure:
   ```bash
   ls /Users/will/dev/nunchi/roko/roko/docs/21-references/
   head -50 /Users/will/dev/nunchi/roko/roko/docs/21-references/00-lifecycle-and-finite-agency.md
   head -50 /Users/will/dev/nunchi/roko/roko/docs/21-references/09-hdc-vsa.md
   ```

2. Check existing explain command:
   ```bash
   grep -rn 'explain\|pub.*explain' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -10
   ls /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/ 2>/dev/null | head -10
   ```

3. Define the reference index schema:
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct ReferenceIndex {
       pub generated_at: String,
       pub source_dir: String,
       pub total_references: usize,
       pub entries: Vec<ReferenceEntry>,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct ReferenceEntry {
       pub paper_id: String,           // e.g., "lifecycle-001"
       pub title: String,              // Paper title
       pub authors: Vec<String>,       // Author list
       pub year: Option<u16>,          // Publication year
       pub topic: String,              // Source file topic (e.g., "lifecycle-and-finite-agency")
       pub unified_primitive: Option<String>, // Which Cell/trait it maps to
       pub depth_doc_ref: Option<String>,     // Related depth doc path
       pub citation_text: String,      // Original citation text
   }
   ```

4. Implement the parser for reference markdown files:
   ```rust
   /// Parse a single reference document into entries.
   ///
   /// Expected format: markdown with citation blocks, headers grouping by subtopic.
   /// Handles variations: numbered lists, blockquotes, inline citations.
   pub fn parse_reference_doc(path: &Path, topic: &str) -> Vec<ReferenceEntry> { ... }
   ```
   Parse patterns:
   - Lines starting with `- ` or `* ` followed by author/title/year
   - Blockquote citations (`> ...`)
   - Numbered references (`1. ...`, `[1] ...`)
   - Extract year from patterns like `(2023)`, `2023`, `(2023a)`
   - Extract authors from patterns like `Author et al.`, `Author, Author, and Author`

5. Implement the index builder:
   ```rust
   /// Build the full reference index from all docs in the reference directory.
   pub fn build_reference_index(docs_dir: &Path) -> Result<ReferenceIndex, ReferenceError> {
       let mut entries = Vec::new();
       for entry in fs::read_dir(docs_dir)? {
           let path = entry?.path();
           if path.extension() == Some("md".as_ref()) {
               let topic = path.file_stem().unwrap().to_string_lossy()
                   .trim_start_matches(|c: char| c.is_numeric() || c == '-')
                   .to_string();
               entries.extend(parse_reference_doc(&path, &topic));
           }
       }
       Ok(ReferenceIndex { entries, total_references: entries.len(), .. })
   }
   ```

6. Add CLI subcommand integration — `roko references build` to generate the index:
   ```rust
   /// Generate .roko/references.json from docs/21-references/
   pub fn cmd_references_build(workspace: &Path) -> Result<()> {
       let docs_dir = workspace.join("docs/21-references");
       let index = build_reference_index(&docs_dir)?;
       let output = workspace.join(".roko/references.json");
       fs::write(&output, serde_json::to_string_pretty(&index)?)?;
       println!("Wrote {} references to {}", index.total_references, output.display());
       Ok(())
   }
   ```

7. Wire into `roko explain` — query the index for relevant citations:
   ```rust
   /// Look up references relevant to a topic.
   pub fn query_references(index: &ReferenceIndex, topic: &str) -> Vec<&ReferenceEntry> {
       index.entries.iter()
           .filter(|e| e.topic.contains(topic) || e.title.to_lowercase().contains(&topic.to_lowercase()))
           .collect()
   }
   ```

8. Write unit tests:
   - Parse a sample reference doc with known citations
   - Year extraction from various formats
   - Author extraction from various formats
   - Query by topic returns matching entries
   - Index serializes to valid JSON

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- references
```

## What NOT to do
- Do NOT modify the reference markdown files — they are read-only source material
- Do NOT require network access — this is purely local file parsing
- Do NOT use an LLM to parse citations — use regex/heuristic parsing
- Do NOT block on perfect parsing — best-effort extraction with graceful fallbacks
- Do NOT add a database — the JSON index is sufficient for the query patterns needed
