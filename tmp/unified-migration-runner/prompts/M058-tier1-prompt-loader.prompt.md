# M058 — SPI Tier 1: Prompt Loader

## Objective
Implement the Tier 1 prompt loader for the 5-tier SPI (Scriptable Plugin Interface). Load Markdown files with TOML front-matter as prompt templates. Resolution order: workspace (`.roko/prompts/`) > user (`~/.roko/prompts/`) > builtin (shipped with roko). This enables prompt customization without writing Rust code -- the lowest-effort extension point.

## Scope
- Crates: `roko-compose`
- Files: `crates/roko-compose/src/prompts/loader.rs` (new), `crates/roko-compose/src/prompts/mod.rs` (new or update), `crates/roko-compose/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.10
- Spec ref: `tmp/unified/14-CONFIG-AND-AUTHORING.md` SS3 (5-Tier SPI)

## Steps
1. Check for existing prompt loading code:
   ```bash
   grep -rn 'prompt.*load\|load.*prompt\|template\|front.?matter\|PromptTemplate' crates/roko-compose/src/ --include='*.rs' | head -15
   ls crates/roko-compose/src/prompts/ 2>/dev/null || echo "no prompts dir"
   ```

2. Read the current prompt/template directory structure:
   ```bash
   ls crates/roko-compose/src/templates/ 2>/dev/null
   ```

3. Define the prompt template format:
   ```markdown
   ---
   name = "code-review-system"
   version = "1.0.0"
   description = "System prompt for code review tasks"
   tags = ["coding", "review"]
   variables = ["language", "project_name"]
   ---

   You are a code reviewer for {{project_name}}.
   Focus on {{language}} best practices.
   ```

4. Implement the prompt loader in `crates/roko-compose/src/prompts/loader.rs`:
   ```rust
   pub struct PromptLoader {
       search_paths: Vec<PathBuf>,  // workspace, user, builtin (in priority order)
   }

   pub struct PromptTemplate {
       pub name: String,
       pub version: String,
       pub description: String,
       pub tags: Vec<String>,
       pub variables: Vec<String>,
       pub body: String,
   }

   impl PromptLoader {
       pub fn new(workspace: Option<PathBuf>, user: Option<PathBuf>, builtin: PathBuf) -> Self;
       pub fn load(&self, name: &str) -> Result<PromptTemplate>;
       pub fn list(&self) -> Vec<PromptTemplate>;
       pub fn resolve(&self, name: &str) -> Option<PathBuf>;  // which file will be used
   }
   ```

5. Implement resolution order: workspace shadows user shadows builtin. If `.roko/prompts/code-review-system.md` exists, it takes precedence over `~/.roko/prompts/code-review-system.md`.

6. Implement variable substitution: `render(template: &PromptTemplate, vars: &HashMap<String, String>) -> String` replaces `{{variable}}` patterns.

7. Parse TOML front-matter (between `---` delimiters) separately from the Markdown body.

8. Write tests:
   - `.roko/prompts/custom.md` file is discovered and loadable
   - Workspace prompt shadows builtin prompt with same name
   - Variable substitution replaces all `{{var}}` patterns
   - Missing required variable produces error
   - TOML front-matter parses correctly

## Verification
```bash
cargo check -p roko-compose
cargo clippy -p roko-compose --no-deps -- -D warnings
cargo test -p roko-compose -- prompts
```

## What NOT to do
- Do NOT use a complex template engine (Tera, Handlebars) -- simple `{{var}}` substitution is sufficient
- Do NOT add conditional logic to templates -- that is a Tier 3+ concern
- Do NOT modify existing templates in `crates/roko-compose/src/templates/`
- Do NOT add prompt caching or hot-reload yet -- load on demand
