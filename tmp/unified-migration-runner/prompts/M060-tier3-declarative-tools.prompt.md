# M060 — SPI Tier 3: Declarative Tool Loader

## Objective
Implement Tier 3 of the 5-tier SPI: declarative tool definitions in TOML. Tools that wrap subprocesses, HTTP endpoints, or MCP servers can be defined without writing Rust code. A TOML manifest declares the tool's name, description, input/output schema, and execution method (shell command, HTTP call, or MCP server reference). The tool loader discovers and registers these tools alongside builtin tools.

## Scope
- Crates: `roko-std`
- Files: `crates/roko-std/src/tools/declarative.rs` (new), `crates/roko-std/src/tools/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.10
- Spec ref: `tmp/unified/14-CONFIG-AND-AUTHORING.md` SS3 (5-Tier SPI)

## Steps
1. Read the existing tool infrastructure:
   ```bash
   grep -rn 'pub trait Tool\|ToolDef\|ToolSpec\|register_tool' crates/roko-std/src/ --include='*.rs' | head -15
   grep -rn 'builtin.*tool\|BUILTIN' crates/roko-std/src/ --include='*.rs' | head -10
   ls crates/roko-std/src/tools/ 2>/dev/null
   ```

2. Define the declarative tool TOML format:
   ```toml
   # .roko/tools/github-pr-review.toml
   [tool]
   name = "github-pr-review"
   description = "Fetch and review a GitHub PR"
   version = "1.0.0"

   [input]
   pr_url = { type = "string", description = "URL of the PR to review" }

   [output]
   review = { type = "string", description = "The review text" }

   [execution]
   kind = "shell"
   command = "gh pr view {{pr_url}} --json body,title,files"
   timeout_ms = 30000
   ```

3. Define execution kinds:
   ```rust
   pub enum ExecutionKind {
       Shell { command: String, timeout: Duration },
       Http { method: String, url: String, headers: HashMap<String, String>, body_template: Option<String> },
       Mcp { server: String, tool_name: String },
   }
   ```

4. Implement the declarative tool loader:
   ```rust
   pub struct DeclarativeToolLoader {
       search_paths: Vec<PathBuf>,
   }

   impl DeclarativeToolLoader {
       pub fn discover(&self) -> Result<Vec<DeclarativeTool>>;
       pub fn load(&self, path: &Path) -> Result<DeclarativeTool>;
   }
   ```

5. Implement `DeclarativeTool` that wraps execution and adapts to the existing tool interface:
   - Shell: substitute `{{input_name}}` patterns in command, execute via `tokio::process::Command`
   - HTTP: substitute patterns in URL/body, execute via reqwest
   - MCP: delegate to the MCP tool dispatch

6. Register discovered declarative tools alongside builtin tools at startup.

7. Write tests:
   - Shell-based declarative tool executes a command and returns output
   - Input variable substitution works correctly
   - Timeout is respected
   - Discovery finds `.toml` files in `.roko/tools/`

## Verification
```bash
cargo check -p roko-std
cargo clippy -p roko-std --no-deps -- -D warnings
cargo test -p roko-std -- declarative
```

## What NOT to do
- Do NOT replace existing builtin tools -- declarative tools supplement them
- Do NOT execute shell commands without timeout enforcement
- Do NOT allow declarative tools to bypass capability checks -- they need the same grants as any tool
- Do NOT add complex scripting (conditionals, loops) -- that is Tier 4 (WASM)
