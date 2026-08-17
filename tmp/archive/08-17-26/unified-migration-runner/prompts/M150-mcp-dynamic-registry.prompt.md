# M150 — Wire MCP Tool Change Notifications

## Objective
Wire MCP tool change notifications in `roko-agent/src/mcp/`. When an MCP server sends `notifications/tools/list_changed`, trigger re-discovery (`tools/list`) and update the `DynamicToolRegistry`. Emit a `tool.registry.changed` Pulse on the Bus so downstream systems know the tool surface has changed. Ensure collision precedence: static > domain > MCP (MCP tools never override built-in or domain tools).

## Scope
- Crates: `roko-agent`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/dynamic_registry.rs` (update logic)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs` (notification handler)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/mod.rs` (re-exports)
- Depth doc: `tmp/unified-depth/13-builtin-catalog/` (MCP tool lifecycle)

## Steps
1. Read the existing DynamicToolRegistry:
   ```bash
   grep -n 'pub fn\|pub async fn\|fn add\|fn remove\|fn update' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/dynamic_registry.rs | head -15
   ```

2. Read the MCP client for notification handling:
   ```bash
   grep -n 'notification\|tools/list\|list_changed\|on_notification' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs | head -15
   grep -n 'pub fn\|pub async fn' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs | head -15
   ```

3. Add notification handler to MCP client:
   ```rust
   /// Handle `notifications/tools/list_changed` from an MCP server.
   ///
   /// Triggers re-discovery and registry update.
   pub async fn handle_tools_list_changed(
       &self,
       server_name: &str,
       registry: &mut DynamicToolRegistry,
   ) -> Result<()> {
       // 1. Call tools/list on the MCP server
       let tools = self.list_tools(server_name).await?;
       // 2. Convert MCP tool responses to ToolDef
       let tool_defs: Vec<ToolDef> = tools.iter().map(|t| mcp_to_tool_def(t, server_name)).collect();
       // 3. Update registry (respecting precedence)
       registry.update_mcp_server(server_name, tool_defs);
       Ok(())
   }
   ```

4. Add `update_mcp_server()` to DynamicToolRegistry:
   ```rust
   impl DynamicToolRegistry {
       /// Replace all tools from a specific MCP server.
       ///
       /// Respects collision precedence: static > domain > MCP.
       /// If an MCP tool name collides with a static/domain tool, the MCP tool is dropped.
       pub fn update_mcp_server(&mut self, server: &str, tools: Vec<ToolDef>) {
           // Remove old tools for this server
           self.mcp_servers.insert(server.to_string(), tools);
           // Rebuild flattened view
           self.rebuild_all_tools();
       }
   }
   ```

5. Add `rebuild_all_tools()` with precedence enforcement:
   ```rust
   fn rebuild_all_tools(&mut self) {
       let mut seen: HashSet<String> = HashSet::new();
       let mut all = Vec::new();
       // Static tools first (highest precedence)
       for tool in &self.base {
           seen.insert(tool.name.clone());
           all.push(tool.clone());
       }
       // Then MCP tools (lowest precedence, skip collisions)
       for tools in self.mcp_servers.values() {
           for tool in tools {
               if !seen.contains(&tool.name) {
                   seen.insert(tool.name.clone());
                   all.push(tool.clone());
               } else {
                   warn!(tool = %tool.name, "MCP tool collision — static/domain takes precedence");
               }
           }
       }
       self.all_tools = all;
   }
   ```

6. Emit Pulse on tool registry change:
   ```bash
   grep -rn 'BusSender\|event_bus\|RokoEvent' /Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/ --include='*.rs' | head -10
   ```
   If event bus is accessible from roko-agent, emit `tool.registry.changed`. Otherwise, return a flag that the caller can use to emit.

7. Write tests:
   - `update_mcp_server` replaces tools for a server
   - Collision: static tool name wins over MCP tool
   - Empty update removes all MCP tools for that server
   - `rebuild_all_tools` produces correct flattened view

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- mcp
cargo test -p roko-agent -- dynamic_registry
```

## What NOT to do
- Do NOT add a polling loop — MCP notifications are push-based, handled on notification arrival
- Do NOT change collision precedence — static > domain > MCP is the correct order
- Do NOT add new dependencies for notification handling — use existing async channels/callbacks
- Do NOT modify the MCP protocol implementation — only add the handler and registry update
- Do NOT make the registry global/static — it lives in the agent session context
