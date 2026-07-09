# M062 — Cell Manifest and Local Registry

## Objective
Define the Cell manifest format (TOML) and implement a local Cell registry that discovers, indexes, and resolves Cells from the workspace. The manifest declares: name, version, author, description, protocols implemented, capabilities required, and input/output schemas. The registry enables `roko marketplace list` to show all discoverable Cells with their protocols.

## Scope
- Crates: `roko-core`, `roko-orchestrator`
- Files: `crates/roko-core/src/manifest.rs` (new), `crates/roko-orchestrator/src/registry.rs` (new), `crates/roko-orchestrator/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.11
- Spec ref: `tmp/unified/15-MARKETPLACE-AND-SHARING.md` SS3-4

## Steps
1. Check for existing manifest or registry code:
   ```bash
   grep -rn 'Manifest\|manifest\|Registry\|registry\|CellRegistry' crates/ --include='*.rs' | grep -v target | head -20
   ```

2. Define the Cell manifest in `crates/roko-core/src/manifest.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CellManifest {
       pub name: String,
       pub version: String,
       pub author: String,
       pub description: String,
       pub protocols: Vec<String>,    // ["Score", "Verify", "Route"]
       pub capabilities: Vec<String>, // ["FsRead", "Shell"]
       pub input_schema: Option<TypeSchema>,
       pub output_schema: Option<TypeSchema>,
       pub tags: Vec<String>,
       pub license: Option<String>,
       pub source: CellSource,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum CellSource {
       Builtin,
       Workspace { path: PathBuf },
       Wasm { path: PathBuf },
       Declarative { path: PathBuf },
   }
   ```

3. Support TOML manifest format:
   ```toml
   [cell]
   name = "markdown-classify"
   version = "1.0.0"
   author = "@wpank"
   description = "Classifies markdown segments by intent"
   protocols = ["Score"]
   capabilities = ["Llm"]
   tags = ["nlp", "classification"]
   ```

4. Implement the local registry in `crates/roko-orchestrator/src/registry.rs`:
   ```rust
   pub struct CellRegistry {
       cells: HashMap<String, CellManifest>,
       search_paths: Vec<PathBuf>,
   }

   impl CellRegistry {
       pub fn new(search_paths: Vec<PathBuf>) -> Self;
       pub fn discover(&mut self) -> Result<usize>;  // scan paths, return count found
       pub fn get(&self, name: &str) -> Option<&CellManifest>;
       pub fn search(&self, query: &str) -> Vec<&CellManifest>;
       pub fn by_protocol(&self, protocol: &str) -> Vec<&CellManifest>;
       pub fn list(&self) -> Vec<&CellManifest>;
   }
   ```

5. Discovery scans: `.roko/cells/`, `~/.roko/cells/`, builtin paths. Looks for `manifest.toml` or `*.toml` files with a `[cell]` section.

6. Export CellManifest from roko-core and CellRegistry from roko-orchestrator.

7. Write tests:
   - Manifest TOML parses correctly
   - Discovery finds Cells in workspace directory
   - Search by name and protocol works
   - Missing manifest produces clear error

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-orchestrator
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
cargo test -p roko-core -- manifest
cargo test -p roko-orchestrator -- registry
```

## What NOT to do
- Do NOT implement remote registry (marketplace HTTP) -- that is M064
- Do NOT implement Cell execution from manifest -- the registry is discovery-only
- Do NOT add version resolution (semver constraints) yet -- exact match is sufficient
- Do NOT duplicate the existing tool registry -- Cells and tools are different concepts
