//! OpenAI-compatible provider adapter.
//!
//! Kimi-K2.5 is an OpenAI-compatible backend with a few non-standard
//! constraints that the request builder and response history plumbing must
//! respect:
//!
//! - Images must be sent as base64 `data:image/...;base64,...` payloads.
//!   Plain image URLs are not accepted.
//! - When thinking is enabled, Kimi fixes `temperature = 1.0`, `top_p = 0.95`,
//!   and `n = 1`.
//! - With thinking enabled, `tool_choice` is limited to `"auto"` or `"none"`.
//! - The built-in `$web_search` tool is incompatible with thinking mode.
//! - Tool-call IDs are returned in `functions.<name>:<idx>` form and must be
//!   preserved exactly when dispatching tool results.
//! - A single request may include at most 128 tools.
//! - Requests have a 2-hour timeout budget.
//! - `reasoning_content` must be carried forward in conversation history when
//!   building the next turn.

use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::sync::Arc;

use crate::Agent;
use crate::codex_agent::CodexAgent;
use crate::dispatcher::HandlerResolver;
use crate::http::ReqwestPoster;
use crate::mcp::{DynamicToolRegistry as McpDynamicToolRegistry, McpConfig, discover_mcp_runtime};
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, build_tool_dispatcher,
    tool_limit_for_temperament, tool_loop_max_iterations_for_profile,
};
use crate::tool_loop::backends::create_openai_compat_backend;
use crate::tool_loop::{MultimodalInputFormat, ToolLoop, ToolLoopAgent};
use crate::translate::capability::cap_tools_for_profile;
use crate::translate::{OpenAiTranslator, Translator};
use roko_core::agent::ProviderKind;
#[cfg(test)]
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::DEFAULT_MAX_OUTPUT_TOKENS;
use roko_core::tool::aliases::canonical_of_claude;
use roko_core::tool::{ToolDef, ToolRegistry, ToolSource, VecToolRegistry};
use roko_std::{DynamicToolRegistry as LocalToolRegistry, StaticToolRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// Adapter for OpenAI-compatible HTTP providers.
pub struct OpenAiCompatAdapter;

fn is_zai_provider(provider: &ProviderConfig, model: &ModelProfile) -> bool {
    model.provider.eq_ignore_ascii_case("zai")
        || provider
            .base_url
            .as_deref()
            .is_some_and(|base_url| base_url.contains("z.ai") || base_url.contains("bigmodel.cn"))
}

fn is_openrouter(base_url: &str) -> bool {
    base_url.contains("openrouter.ai")
}

fn inject_glm_params(
    body: &mut Map<String, Value>,
    provider: &ProviderConfig,
    model: &ModelProfile,
) {
    if !model.supports_thinking || !is_zai_provider(provider, model) {
        return;
    }

    body.insert(
        "thinking".to_string(),
        json!({
            "type": "enabled",
            "clear_thinking": true,
        }),
    );
    body.insert("tool_stream".to_string(), Value::Bool(true));
}

fn inject_provider_routing(
    body: &mut Map<String, Value>,
    provider: &ProviderConfig,
    model: &ModelProfile,
) {
    let Some(base_url) = provider.base_url.as_deref() else {
        return;
    };
    if !is_openrouter(base_url) {
        return;
    }

    let Some(routing) = model.provider_routing.as_ref() else {
        return;
    };

    match serde_json::to_value(routing) {
        Ok(value) => {
            body.insert("provider".to_string(), value);
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to serialize provider routing config for OpenRouter");
        }
    }
}

fn is_kimi_model(model: &ModelProfile) -> bool {
    model.slug.starts_with("kimi-")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub(crate) struct ChatMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[allow(dead_code)]
impl ChatMessage {
    fn content_blocks(&self) -> Option<&[ContentBlock]> {
        (!self.content.is_empty()).then_some(self.content.as_slice())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum ContentBlock {
    Text { text: String },
    ImageUrl { image_url: ImageUrlBlock },
}

#[allow(dead_code)]
impl ContentBlock {
    fn is_image_url(&self) -> bool {
        matches!(self, Self::ImageUrl { .. })
    }

    fn is_base64(&self) -> bool {
        matches!(
            self,
            Self::ImageUrl { image_url }
                if image_url.url.starts_with("data:image/")
                    && image_url.url.contains(";base64,")
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub(crate) struct ImageUrlBlock {
    url: String,
}

fn inject_kimi_params(body: &mut Map<String, Value>, model: &ModelProfile) {
    if !model.supports_thinking || !is_kimi_model(model) {
        return;
    }

    body.insert(
        "thinking".to_string(),
        json!({
            "type": "enabled",
        }),
    );
}

pub(crate) fn resolve_api_key(provider: &ProviderConfig) -> Result<String, AgentCreationError> {
    provider
        .resolve_api_key()
        .or_else(|| {
            if provider.base_url.as_deref().is_some_and(|url| {
                url.starts_with("http://localhost:") || url.starts_with("http://127.0.0.1:")
            }) {
                Some(String::new())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            AgentCreationError::MissingApiKey(provider.api_key_env.clone().unwrap_or_default())
        })
}

fn base_url_for_codex(provider: &ProviderConfig) -> String {
    let base_url = provider
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    base_url
        .strip_suffix("/v1")
        .unwrap_or(base_url.as_str())
        .to_string()
}

pub(crate) fn base_url_for_tool_loop(provider: &ProviderConfig) -> String {
    provider
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
}

pub(crate) fn build_extra_body_params(
    provider: &ProviderConfig,
    model: &ModelProfile,
) -> Map<String, Value> {
    let mut extra_body_params = Map::new();
    inject_glm_params(&mut extra_body_params, provider, model);
    inject_kimi_params(&mut extra_body_params, model);
    inject_provider_routing(&mut extra_body_params, provider, model);
    extra_body_params
}

/// Returns `true` for model slugs that require `max_completion_tokens`
/// instead of `max_tokens` in the request body (modern OpenAI models).
pub(crate) fn should_use_max_completion_tokens(slug: &str) -> bool {
    const PREFIXES: &[&str] = &["gpt-4o", "gpt-5", "o1", "o3", "o4", "codex"];
    PREFIXES.iter().any(|p| slug.starts_with(p))
}

pub(crate) fn max_tokens_for_model(model: &ModelProfile) -> u32 {
    model
        .max_output
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS)
}

fn parse_allowed_tools_csv(csv: Option<&str>) -> Option<HashSet<String>> {
    let allowed: HashSet<String> = csv
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            // Resolve Claude CLI aliases ("Read" -> "read_file") to canonical names.
            // If already canonical or unknown (MCP/plugin tool), pass through as-is.
            canonical_of_claude(name).unwrap_or(name).to_string()
        })
        .collect();
    (!allowed.is_empty()).then_some(allowed)
}

struct McpRuntimeOwner(std::sync::Mutex<Option<tokio::runtime::Runtime>>);

impl Drop for McpRuntimeOwner {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.lock().expect("MCP runtime owner lock").take() {
            runtime.shutdown_background();
        }
    }
}

fn block_on<F>(future: F) -> Result<(F::Output, Arc<dyn Send + Sync>), AgentCreationError>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let build_rt = || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| {
                AgentCreationError::MissingConfig(format!(
                    "failed to create MCP discovery runtime: {e}"
                ))
            })
    };

    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(move || {
            let rt = build_rt()?;
            let output = rt.block_on(future);
            let owner: Arc<dyn Send + Sync> =
                Arc::new(McpRuntimeOwner(std::sync::Mutex::new(Some(rt))));
            Ok((output, owner))
        })
        .join()
        .unwrap_or_else(|_| {
            Err(AgentCreationError::MissingConfig(
                "MCP discovery thread panicked".to_string(),
            ))
        })
    } else {
        let rt = build_rt()?;
        let output = rt.block_on(future);
        let owner: Arc<dyn Send + Sync> =
            Arc::new(McpRuntimeOwner(std::sync::Mutex::new(Some(rt))));
        Ok((output, owner))
    }
}

fn add_mcp_tools_to_registry(registry: &mut McpDynamicToolRegistry, mcp_tools: Vec<ToolDef>) {
    let mut by_server: BTreeMap<String, Vec<ToolDef>> = BTreeMap::new();

    for tool in mcp_tools {
        let server = match &tool.source {
            roko_core::tool::ToolSource::Mcp { server } => server.clone(),
            _ => tool
                .name
                .split_once('.')
                .map_or("unknown", |(server, _)| server)
                .to_string(),
        };
        by_server.entry(server).or_default().push(tool);
    }

    for (server, tools) in by_server {
        registry.add_mcp_tools(&server, tools);
    }
}

/// Build the tool registry for a non-Claude provider.
///
/// Alias resolution audit (P09): all non-Claude provider backends
/// (Anthropic API, Gemini, Cerebras, Perplexity) call this function,
/// so the Claude alias fix in `parse_allowed_tools_csv` covers all of them.
/// The Claude CLI backend (`claude_cli.rs`) passes the CSV directly to the CLI
/// process and does not call this function — Claude CLI natively understands
/// its own PascalCase names, so no alias translation is needed there.
pub(crate) fn tool_registry_for_options(
    model: &ModelProfile,
    options: &AgentOptions,
) -> Result<
    (
        Arc<dyn ToolRegistry>,
        Vec<ToolDef>,
        Arc<dyn HandlerResolver>,
    ),
    AgentCreationError,
> {
    let static_base = StaticToolRegistry::new();
    // The shared static catalog also contains definition-only GitHub MCP
    // schemas. Seed the executable provider catalog only with locally-backed
    // definitions; live MCP definitions are layered in below with their
    // retained clients.
    let local_builtin_tools = static_base
        .all()
        .iter()
        .filter(|tool| !matches!(&tool.source, ToolSource::Mcp { .. }))
        .cloned()
        .collect();
    let mut local_registry = LocalToolRegistry::from_tools(local_builtin_tools);
    let builtin_resolver: Arc<dyn HandlerResolver> =
        Arc::new(|name: &str| roko_std::tool::handlers::handler_for(name));
    let static_resolver = if let Some(local_runtime) = &options.pre_discovered_local_tools {
        let incorrectly_sourced = local_runtime
            .tools()
            .iter()
            .filter(|tool| matches!(&tool.source, ToolSource::Mcp { .. }))
            .map(|tool| tool.name.clone())
            .collect::<Vec<_>>();
        if !incorrectly_sourced.is_empty() {
            return Err(AgentCreationError::MissingConfig(format!(
                "local tool runtime contains MCP definitions without retained MCP clients: {}",
                incorrectly_sourced.join(", ")
            )));
        }
        let missing = local_runtime.missing_handlers();
        if !missing.is_empty() {
            return Err(AgentCreationError::MissingConfig(format!(
                "local tool catalog contains definitions without executable handlers: {}",
                missing.join(", ")
            )));
        }
        local_registry.register_all(local_runtime.tools().iter().cloned());
        local_runtime.resolver(builtin_resolver)
    } else {
        builtin_resolver
    };
    let mut registry = McpDynamicToolRegistry::new(&local_registry);
    let mut mcp_runtime = options.pre_discovered_mcp_runtime.clone();

    // A runtime bundle is the preferred path: it retains both definitions and
    // initialized clients. Definition-only tools remain supported for schema
    // rendering callers, but cannot create executable MCP handlers.
    if let Some(runtime) = &mcp_runtime {
        let unexecutable = runtime.unexecutable_tools();
        if !unexecutable.is_empty() {
            return Err(AgentCreationError::MissingConfig(format!(
                "MCP runtime contains definitions without initialized clients: {}",
                unexecutable.join(", ")
            )));
        }
        add_mcp_tools_to_registry(&mut registry, runtime.tools().as_ref().clone());
    } else if let Some(pre_discovered) = &options.pre_discovered_mcp_tools {
        if pre_discovered
            .iter()
            .any(|tool| matches!(&tool.source, roko_core::tool::ToolSource::Mcp { .. }))
        {
            return Err(AgentCreationError::MissingConfig(
                "pre-discovered MCP definitions require pre_discovered_mcp_runtime so advertised tools retain executable clients"
                    .to_string(),
            ));
        }
        add_mcp_tools_to_registry(&mut registry, pre_discovered.as_ref().clone());
    } else if let Some(mcp_config_path) = &options.mcp_config {
        let mcp_config = McpConfig::load(mcp_config_path).map_err(|err| {
            AgentCreationError::MissingConfig(format!(
                "mcp config {}: {err}",
                mcp_config_path.display()
            ))
        })?;
        let (runtime, owner) = block_on(async move { discover_mcp_runtime(&mcp_config).await })?;
        let runtime = runtime
            .map_err(|err| {
                AgentCreationError::MissingConfig(format!(
                    "mcp tool discovery from {} failed: {err}",
                    mcp_config_path.display()
                ))
            })?
            .with_owner(owner);
        add_mcp_tools_to_registry(&mut registry, runtime.tools().as_ref().clone());
        mcp_runtime = Some(Arc::new(runtime));
    }

    // Structured contracts take precedence over the legacy CSV. In
    // particular, `Some([])` must remain an empty set rather than collapsing
    // to `None` (which means unrestricted in the legacy parser).
    let allowed = options
        .agent_contract
        .as_ref()
        .and_then(|contract| contract.allowed_tools.as_ref())
        .map(|tools| tools.iter().cloned().collect::<HashSet<_>>())
        .or_else(|| parse_allowed_tools_csv(options.tools.as_deref()));
    let denied = options
        .agent_contract
        .as_ref()
        .map(|contract| {
            contract
                .forbidden_tool_names()
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let tools: Vec<ToolDef> = registry
        .all()
        .iter()
        .filter(|tool| {
            !denied.contains(tool.name.as_str())
                && allowed
                    .as_ref()
                    .is_none_or(|allowed| allowed.contains(tool.name.as_str()))
        })
        .cloned()
        .collect();
    // User-configured plugin/MCP tools must survive model tool-count caps;
    // otherwise a complete builtin catalog can silently truncate every
    // extension appended after it. Preserve deterministic order within each
    // class while placing runtime-contributed definitions first.
    let (mut plugin_tools, non_plugin_tools): (Vec<_>, Vec<_>) = tools
        .into_iter()
        .partition(|tool| matches!(&tool.source, roko_core::tool::ToolSource::Plugin { .. }));
    let (runtime_tools, builtin_tools): (Vec<_>, Vec<_>) = non_plugin_tools
        .into_iter()
        .partition(|tool| !matches!(&tool.source, roko_core::tool::ToolSource::Builtin));
    plugin_tools.extend(runtime_tools);
    plugin_tools.extend(builtin_tools);
    let mut tools = cap_tools_for_profile(model, plugin_tools);
    tools.truncate(tool_limit_for_temperament(tools.len()));

    let resolver = mcp_runtime.map_or(static_resolver.clone(), |runtime| {
        runtime.resolver(static_resolver)
    });

    Ok((
        Arc::new(VecToolRegistry::from_tools(tools.clone())),
        tools,
        resolver,
    ))
}

fn default_agent_name(model: &ModelProfile, options: &AgentOptions) -> String {
    if options.name.is_empty() {
        format!("codex:{}", model.slug)
    } else {
        options.name.clone()
    }
}

impl ProviderAdapter for OpenAiCompatAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAiCompat
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        let api_key = resolve_api_key(provider)?;

        let timeout = options.effective_timeout_ms(provider.timeout_ms);
        let max_tokens = max_tokens_for_model(model);
        let extra_headers = provider.extra_headers.clone().unwrap_or_default();
        let extra_body_params = build_extra_body_params(provider, model);
        let agent_name = default_agent_name(model, options);

        if model.supports_tools {
            let (registry, tools, resolver) = tool_registry_for_options(model, options)?;
            let dispatcher = build_tool_dispatcher(registry, resolver);
            let translator: Arc<dyn Translator> = Arc::new(OpenAiTranslator);
            let mut tool_loop_provider = provider.clone();
            tool_loop_provider.timeout_ms = Some(timeout);
            let poster = Arc::new(ReqwestPoster::new());
            let backend = create_openai_compat_backend(&tool_loop_provider, model, poster)?;

            let tool_loop = ToolLoop::new(translator, dispatcher, backend)
                .with_max_iterations(tool_loop_max_iterations_for_profile(Some(model)))
                .with_context_token_limit(
                    usize::try_from(model.context_window).unwrap_or(usize::MAX),
                )
                .with_model_profile(model.clone());

            let mut agent = ToolLoopAgent::new(tool_loop)
                .with_tools(tools)
                .with_name(agent_name)
                .with_input_messages(options.input_messages.clone())
                .with_multimodal_input_format(MultimodalInputFormat::OpenAi);
            if let Some(prompt) = &options.system_prompt {
                agent = agent.with_system_prompt(prompt.clone());
            }
            if let Some(ref dir) = options.working_dir {
                agent = agent.with_worktree_path(dir.clone());
            }
            if let Some(root) = options.effective_immune_root() {
                agent = agent.with_immune_root(root);
            }

            return Ok(Box::new(agent));
        }

        let mut agent = CodexAgent::new(api_key, model.slug.clone())
            .with_base_url(base_url_for_codex(provider))
            .with_timeout_ms(timeout)
            .with_max_tokens(max_tokens)
            .with_use_max_completion_tokens(
                model.use_max_completion_tokens || should_use_max_completion_tokens(&model.slug),
            )
            .with_extra_headers(extra_headers)
            .with_extra_body_params(extra_body_params)
            .with_input_messages(options.input_messages.clone())
            .with_name(agent_name);

        if let Some(prompt) = &options.system_prompt {
            agent = agent.with_system_prompt(prompt.clone());
        }

        if let Some(provider_semaphores) = options.provider_semaphores.clone() {
            agent = agent.with_provider_semaphores(model.provider.clone(), provider_semaphores);
        }

        Ok(Box::new(agent))
    }

    fn supports_local_tool_runtime(&self) -> bool {
        true
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        // Z.AI-specific numeric error codes take priority.
        if let Some(code) = body.pointer("/error/code").and_then(Value::as_str) {
            match code {
                "1302" => {
                    return ProviderError::RateLimit {
                        retry_after_ms: Some(5_000),
                    };
                }
                "1303" | "1304" | "1305" => {
                    return ProviderError::RateLimit {
                        retry_after_ms: Some(60_000),
                    };
                }
                "1301" => return ProviderError::ContentPolicy,
                "1000" | "1001" | "1002" | "1003" | "1004" => return ProviderError::AuthFailure,
                "1211" => return ProviderError::ModelNotFound,
                "1261" => return ProviderError::ContextOverflow,
                // Non-Z.AI codes (e.g. OpenAI "context_length_exceeded") fall
                // through to the generic classifier below.
                _ => {}
            }
        }

        super::error_classify::classify_http_status(
            status,
            body,
            super::error_classify::RetryAfterSource::BodyRetryAfterCompat,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{HttpPostError, HttpPoster};
    use crate::mcp::client::McpError;
    use crate::mcp::{
        McpClient, McpRequest, McpResponse, McpRuntime, McpRuntimeTransport, Transport,
    };
    use crate::provider::{LocalToolRuntime, with_safety_layer};
    use roko_core::tool::{ToolCategory, ToolPermission};
    use roko_core::{Body, Context, Kind, Signal};
    use std::collections::HashMap;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    fn spawn_chat_server(
        response: String,
    ) -> (String, Arc<Mutex<Option<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let captured = Arc::new(Mutex::new(None));
        let captured_request = Arc::clone(&captured);

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");

            let mut buf = Vec::new();
            let mut header_end = None;
            let mut content_length = None;

            loop {
                let mut chunk = [0_u8; 1024];
                let n = stream.read(&mut chunk).expect("read request");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);

                if header_end.is_none()
                    && let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    header_end = Some(pos + 4);
                    let headers = String::from_utf8_lossy(&buf[..pos + 4]);
                    content_length = headers.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    });
                }

                if let (Some(header_end), Some(content_length)) = (header_end, content_length)
                    && buf.len() >= header_end + content_length
                {
                    break;
                }
            }

            let header_end = header_end.expect("request headers");
            let content_length = content_length.expect("content length");
            let request = String::from_utf8_lossy(&buf[..header_end + content_length]).to_string();
            *captured_request.lock().expect("capture lock") = Some(request);

            let response_bytes = response.as_bytes();
            let wire = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_bytes.len(),
                response
            );
            stream.write_all(wire.as_bytes()).expect("write response");
            stream.flush().expect("flush response");
        });

        (format!("http://{}", addr), captured, handle)
    }

    fn write_script(path: &std::path::Path, body: &str) {
        fs::write(path, body).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).expect("script metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).expect("chmod script");
        }
    }

    fn spawn_chat_server_sequence(
        responses: Vec<String>,
    ) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = Arc::clone(&captured);

        let handle = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("set read timeout");

                let mut buf = Vec::new();
                let mut header_end = None;
                let mut content_length = None;

                loop {
                    let mut chunk = [0_u8; 1024];
                    let n = stream.read(&mut chunk).expect("read request");
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);

                    if header_end.is_none()
                        && let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        header_end = Some(pos + 4);
                        let headers = String::from_utf8_lossy(&buf[..pos + 4]);
                        content_length = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        });
                    }

                    if let (Some(header_end), Some(content_length)) = (header_end, content_length)
                        && buf.len() >= header_end + content_length
                    {
                        break;
                    }
                }

                let header_end = header_end.expect("request headers");
                let content_length = content_length.expect("content length");
                let request =
                    String::from_utf8_lossy(&buf[..header_end + content_length]).to_string();
                captured_requests
                    .lock()
                    .expect("capture lock")
                    .push(request);

                let response_bytes = response.as_bytes();
                let wire = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_bytes.len(),
                    response
                );
                stream.write_all(wire.as_bytes()).expect("write response");
                stream.flush().expect("flush response");
            }
        });

        (format!("http://{}", addr), captured, handle)
    }

    #[derive(Debug, Clone)]
    struct RecordedRequest {
        url: String,
        headers: Vec<(String, String)>,
        body: String,
        timeout_ms: u64,
    }

    #[derive(Debug)]
    struct MockPoster {
        response: String,
        captured: Arc<Mutex<Option<RecordedRequest>>>,
    }

    impl MockPoster {
        fn new(response: impl Into<String>) -> (Arc<Self>, Arc<Mutex<Option<RecordedRequest>>>) {
            let captured = Arc::new(Mutex::new(None));
            let poster = Arc::new(Self {
                response: response.into(),
                captured: Arc::clone(&captured),
            });
            (poster, captured)
        }
    }

    #[async_trait::async_trait]
    impl HttpPoster for MockPoster {
        async fn post_json(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: &[u8],
            timeout_ms: u64,
        ) -> Result<String, HttpPostError> {
            let request = RecordedRequest {
                url: url.to_string(),
                headers: headers.to_vec(),
                body: String::from_utf8(body.to_vec()).expect("request body is utf8"),
                timeout_ms,
            };
            *self.captured.lock().expect("capture lock") = Some(request);
            Ok(self.response.clone())
        }
    }

    #[derive(Debug)]
    struct UnusedMcpTransport;

    #[async_trait::async_trait]
    impl Transport for UnusedMcpTransport {
        async fn roundtrip(&self, _request: &McpRequest) -> Result<McpResponse, McpError> {
            Err(McpError::Transport(
                "test transport must not be called".to_string(),
            ))
        }
    }

    fn live_mcp_runtime(tools: Vec<ToolDef>, server: &str) -> Arc<McpRuntime> {
        let transport: McpRuntimeTransport = Arc::new(UnusedMcpTransport);
        let client = Arc::new(McpClient::new(transport));
        Arc::new(McpRuntime::from_clients(
            tools,
            HashMap::from([(server.to_string(), client)]),
        ))
    }

    fn plugin_tool(name: &str, plugin: &str) -> ToolDef {
        let mut tool = ToolDef::new(
            name,
            "test plugin tool",
            ToolCategory::Exec,
            ToolPermission::executes(),
        );
        tool.source = ToolSource::Plugin {
            name: plugin.to_string(),
        };
        tool
    }

    #[tokio::test]
    async fn glm_thinking_injection() {
        let response = serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "zai-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        })
        .to_string();
        let (base_url, captured, handle) = spawn_chat_server(response);

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some(format!("{base_url}/v4")),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            context_window: 200_000,
            max_output: Some(1_024),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        };
        let options = AgentOptions {
            timeout_ms: Some(2_500),
            name: "zai-agent".to_string(),
            ..Default::default()
        };

        let adapter = OpenAiCompatAdapter;
        assert_eq!(adapter.kind(), ProviderKind::OpenAiCompat);

        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("create agent");
        assert_eq!(agent.name(), "zai-agent");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "zai-ok");

        let request = captured
            .lock()
            .expect("capture lock")
            .take()
            .expect("captured request");
        assert!(
            request.starts_with("POST /v4/chat/completions HTTP/1.1"),
            "unexpected request line: {}",
            request.lines().next().unwrap_or("")
        );

        let body = request.split("\r\n\r\n").nth(1).expect("request body");
        let parsed: Value = serde_json::from_str(body).expect("json request body");
        assert_eq!(parsed["model"], "glm-5.1");
        assert_eq!(parsed["max_tokens"], 1024);
        assert_eq!(parsed["messages"][1]["content"], "hello");
        assert!(!parsed["tools"].as_array().expect("tools array").is_empty());
        assert_eq!(
            parsed["thinking"],
            serde_json::json!({
                "type": "enabled",
                "clear_thinking": true
            })
        );
        assert_eq!(parsed["tool_stream"], Value::Bool(true));

        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn kimi_thinking_injection() {
        let response = serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "kimi-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 13,
                "completion_tokens": 8,
                "total_tokens": 21
            }
        })
        .to_string();
        let (base_url, captured, handle) = spawn_chat_server(response);

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some(format!("{base_url}/v1")),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "moonshot".to_string(),
            slug: "kimi-k2.5".to_string(),
            context_window: 256_000,
            max_output: Some(65_535),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: true,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: true,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: Some(128),
            tokenizer_ratio: None,
            ..Default::default()
        };
        let options = AgentOptions {
            timeout_ms: Some(2_500),
            name: "kimi-agent".to_string(),
            ..Default::default()
        };

        let adapter = OpenAiCompatAdapter;
        assert_eq!(adapter.kind(), ProviderKind::OpenAiCompat);

        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("create agent");
        assert_eq!(agent.name(), "kimi-agent");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "kimi-ok");

        let request = captured
            .lock()
            .expect("capture lock")
            .take()
            .expect("captured request");
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));

        let body = request.split("\r\n\r\n").nth(1).expect("request body");
        let parsed: Value = serde_json::from_str(body).expect("json request body");
        assert_eq!(parsed["model"], "kimi-k2.5");
        assert_eq!(parsed["max_tokens"], 65535);
        assert_eq!(parsed["messages"][1]["content"], "hello");
        assert!(!parsed["tools"].as_array().expect("tools array").is_empty());
        assert_eq!(
            parsed["thinking"],
            serde_json::json!({
                "type": "enabled"
            })
        );

        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn adapter_uses_codex_fallback_for_non_tool_models() {
        let response = serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "codex-fallback-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        })
        .to_string();
        let (base_url, captured, handle) = spawn_chat_server(response);

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some(format!("{base_url}/v1")),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "openai".to_string(),
            slug: "gpt-4.1-mini".to_string(),
            context_window: 128_000,
            max_output: Some(2_048),
            supports_tools: false,
            supports_thinking: false,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        };
        let options = AgentOptions {
            timeout_ms: Some(2_500),
            name: "codex-fallback-agent".to_string(),
            ..Default::default()
        };

        let agent = OpenAiCompatAdapter
            .create_agent(&provider, &model, &options)
            .expect("create codex fallback agent");
        assert_eq!(agent.name(), "codex-fallback-agent");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(
            result.output.body.as_text().unwrap_or(""),
            "codex-fallback-ok"
        );

        let request = captured
            .lock()
            .expect("capture lock")
            .take()
            .expect("captured request");
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));

        let body = request.split("\r\n\r\n").nth(1).expect("request body");
        let parsed: Value = serde_json::from_str(body).expect("json request body");
        assert_eq!(parsed["model"], "gpt-4.1-mini");
        assert_eq!(parsed["max_tokens"], 2048);
        assert_eq!(parsed["messages"][0]["content"], "hello");
        assert!(parsed.get("tools").is_none());

        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn adapter_uses_tool_loop() {
        let first_response = serde_json::json!({
            "id": "chatcmpl-tool-1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "call-ls-1",
                        "type": "function",
                        "function": {
                            "name": "ls",
                            "arguments": "{\"path\":\".\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 17,
                "completion_tokens": 4,
                "total_tokens": 21
            }
        })
        .to_string();
        let second_response = serde_json::json!({
            "id": "chatcmpl-tool-2",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "tool-loop-ok"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 19,
                "completion_tokens": 3,
                "total_tokens": 22
            }
        })
        .to_string();
        let (base_url, captured, handle) =
            spawn_chat_server_sequence(vec![first_response, second_response]);

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some(format!("{base_url}/v1")),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            context_window: 200_000,
            max_output: Some(1_024),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        };

        let agent = OpenAiCompatAdapter
            .create_agent(&provider, &model, &AgentOptions::default())
            .expect("create tool-loop agent");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "tool-loop-ok");

        let requests = captured.lock().expect("capture lock").clone();
        assert_eq!(requests.len(), 2, "tool loop should make two backend turns");

        let first_body = requests[0].split("\r\n\r\n").nth(1).expect("first body");
        let first_json: Value = serde_json::from_str(first_body).expect("first request json");
        assert_eq!(first_json["model"], "glm-5.1");
        assert_eq!(first_json["messages"][1]["content"], "hello");
        assert!(
            !first_json["tools"]
                .as_array()
                .expect("tools array")
                .is_empty()
        );

        let second_body = requests[1].split("\r\n\r\n").nth(1).expect("second body");
        let second_json: Value = serde_json::from_str(second_body).expect("second request json");
        let messages = second_json["messages"]
            .as_array()
            .expect("second request messages");
        assert!(messages.iter().any(|message| {
            message.get("role").and_then(Value::as_str) == Some("assistant")
                && message.get("tool_calls").is_some()
        }));
        assert!(messages.iter().any(|message| {
            message.get("role").and_then(Value::as_str) == Some("tool")
                && message.get("tool_call_id").and_then(Value::as_str) == Some("call-ls-1")
        }));

        handle.join().expect("server thread");
    }

    #[tokio::test]
    async fn mcp_bridge_http_executes_discovered_tool_through_live_client() {
        let first_response = serde_json::json!({
            "id": "chatcmpl-mcp-1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "call-mcp-1",
                        "type": "function",
                        "function": {
                            "name": "local__DOT__echo",
                            "arguments": "{\"text\":\"hello\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 3,
                "total_tokens": 15
            }
        })
        .to_string();
        let second_response = serde_json::json!({
            "id": "chatcmpl-mcp-2",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "mcp-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 18,
                "completion_tokens": 3,
                "total_tokens": 21
            }
        })
        .to_string();
        let (base_url, captured, handle) =
            spawn_chat_server_sequence(vec![first_response, second_response]);

        let tmp = tempfile::tempdir().expect("tempdir");
        let server_script = tmp.path().join("mcp-server.sh");
        write_script(
            &server_script,
            r#"#!/bin/sh
set -eu
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
      ;;
    *'"method":"tools/list"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo from MCP","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]},"annotations":{"readOnlyHint":true}}]}}'
      ;;
    *'"method":"tools/call"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"echoed-by-mcp"}],"isError":false}}'
      ;;
    *)
      printf '%s\n' '{"jsonrpc":"2.0","id":999,"result":{}}'
      ;;
  esac
done
"#,
        );
        let mcp_config = tmp.path().join("mcp.json");
        fs::write(
            &mcp_config,
            serde_json::json!({
                "servers": [{
                    "name": "local",
                    "command": server_script,
                    "args": [],
                    "env": {}
                }]
            })
            .to_string(),
        )
        .expect("write mcp config");

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some(format!("{base_url}/v1")),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            context_window: 200_000,
            max_output: Some(1_024),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: true,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            max_tools: Some(128),
            tokenizer_ratio: None,
            ..Default::default()
        };
        let options = AgentOptions {
            mcp_config: Some(mcp_config),
            ..Default::default()
        };

        let agent = with_safety_layer(Some(crate::safety::SafetyLayer::permissive()), || {
            OpenAiCompatAdapter
                .create_agent(&provider, &model, &options)
                .expect("create agent")
        });

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "mcp-ok");

        let requests = captured.lock().expect("capture lock").clone();
        assert_eq!(requests.len(), 2, "MCP tool loop needs two model turns");
        let first_body = requests[0]
            .split("\r\n\r\n")
            .nth(1)
            .expect("first request body");
        let first: Value = serde_json::from_str(first_body).expect("first request json");
        let tools = first["tools"].as_array().expect("tools array");
        let tool_names = tools
            .iter()
            .filter_map(|tool| tool["function"]["name"].as_str())
            .collect::<Vec<_>>();

        assert!(
            tools.iter().any(|tool| {
                tool["function"]["name"].as_str() == Some("local__DOT__echo")
                    && tool["function"]["description"].as_str() == Some("Echo from MCP")
            }),
            "advertised tools: {tool_names:?}"
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["function"]["name"].as_str() == Some("ls"))
        );

        let second_body = requests[1]
            .split("\r\n\r\n")
            .nth(1)
            .expect("second request body");
        let second: Value = serde_json::from_str(second_body).expect("second request json");
        let tool_message = second["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .find(|message| message["role"] == "tool")
            .expect("MCP tool result message");
        assert_eq!(tool_message["tool_call_id"], "call-mcp-1");
        assert_eq!(tool_message["content"], "echoed-by-mcp");

        handle.join().expect("server thread");
    }

    #[test]
    fn definition_only_mcp_tools_are_rejected_instead_of_advertised_without_handlers() {
        let definition = crate::mcp::mcp_to_tool_def(
            &crate::mcp::McpToolDef {
                name: "echo".to_string(),
                description: Some("Echo".to_string()),
                input_schema: None,
                annotations: None,
            },
            "local",
        );
        let options = AgentOptions {
            pre_discovered_mcp_tools: Some(Arc::new(vec![definition])),
            ..Default::default()
        };

        let error = match tool_registry_for_options(&ModelProfile::default(), &options) {
            Ok(_) => panic!("definition-only MCP tool must not be advertised"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("pre_discovered_mcp_runtime"));
    }

    #[test]
    fn no_mcp_runtime_omits_definition_only_static_mcp_tools() {
        let (_, tools, resolver) =
            tool_registry_for_options(&ModelProfile::default(), &AgentOptions::default())
                .expect("local executable catalog");

        assert!(
            tools
                .iter()
                .all(|tool| !matches!(&tool.source, ToolSource::Mcp { .. })),
            "definition-only MCP tools were advertised: {:?}",
            tools
                .iter()
                .filter(|tool| matches!(&tool.source, ToolSource::Mcp { .. }))
                .map(|tool| &tool.name)
                .collect::<Vec<_>>()
        );
        assert!(
            tools
                .iter()
                .all(|tool| resolver.resolve(&tool.name).is_some())
        );
        assert!(!tools.iter().any(|tool| tool.name == "github.list_prs"));
    }

    #[test]
    fn mcp_runtime_without_client_is_rejected() {
        let definition = crate::mcp::mcp_to_tool_def(
            &crate::mcp::McpToolDef {
                name: "echo".to_string(),
                description: Some("Echo".to_string()),
                input_schema: None,
                annotations: None,
            },
            "missing",
        );
        let options = AgentOptions {
            pre_discovered_mcp_runtime: Some(Arc::new(McpRuntime::from_clients(
                vec![definition],
                HashMap::new(),
            ))),
            ..Default::default()
        };

        let error = match tool_registry_for_options(&ModelProfile::default(), &options) {
            Ok(_) => panic!("clientless MCP runtime must not be advertised"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("missing.echo"));
        assert!(error.to_string().contains("initialized clients"));
    }

    #[test]
    fn plugin_definitions_and_handlers_win_builtin_and_mcp_collisions() {
        let plugin_tools = vec![
            plugin_tool("read_file", "override"),
            plugin_tool("live.remote", "override"),
        ];
        let local_resolver: Arc<dyn HandlerResolver> = Arc::new(|name: &str| match name {
            "read_file" => roko_std::tool::handlers::handler_for("grep"),
            "live.remote" => roko_std::tool::handlers::handler_for("ls"),
            _ => None,
        });
        let mcp_tool = crate::mcp::mcp_to_tool_def(
            &crate::mcp::McpToolDef {
                name: "remote".to_string(),
                description: Some("colliding MCP tool".to_string()),
                input_schema: None,
                annotations: None,
            },
            "live",
        );
        let options = AgentOptions {
            pre_discovered_local_tools: Some(Arc::new(LocalToolRuntime::new(
                plugin_tools,
                local_resolver,
            ))),
            pre_discovered_mcp_runtime: Some(live_mcp_runtime(vec![mcp_tool], "live")),
            ..Default::default()
        };
        let model = ModelProfile {
            slug: "glm-5.1".to_string(),
            max_tools: Some(128),
            ..Default::default()
        };

        let (_, tools, resolver) =
            tool_registry_for_options(&model, &options).expect("collision-safe catalog");

        for name in ["read_file", "live.remote"] {
            let matches = tools
                .iter()
                .filter(|tool| tool.name == name)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "{name} should be deduplicated");
            assert!(matches!(
                &matches[0].source,
                ToolSource::Plugin { name } if name == "override"
            ));
        }
        assert_eq!(
            resolver
                .resolve("read_file")
                .expect("plugin handler")
                .name(),
            "grep"
        );
        assert_eq!(
            resolver
                .resolve("live.remote")
                .expect("plugin handler")
                .name(),
            "ls"
        );
    }

    #[test]
    fn model_cap_prioritizes_plugins_then_live_mcp_then_builtins() {
        let plugin_tools = vec![plugin_tool("one.run", "one"), plugin_tool("two.run", "two")];
        let local_resolver: Arc<dyn HandlerResolver> = Arc::new(|name: &str| {
            matches!(name, "one.run" | "two.run")
                .then(|| roko_std::tool::handlers::handler_for("ls"))
                .flatten()
        });
        let mcp_tools = ["first", "second"]
            .into_iter()
            .map(|name| {
                crate::mcp::mcp_to_tool_def(
                    &crate::mcp::McpToolDef {
                        name: name.to_string(),
                        description: None,
                        input_schema: None,
                        annotations: None,
                    },
                    "live",
                )
            })
            .collect::<Vec<_>>();
        let options = AgentOptions {
            pre_discovered_local_tools: Some(Arc::new(LocalToolRuntime::new(
                plugin_tools,
                local_resolver,
            ))),
            pre_discovered_mcp_runtime: Some(live_mcp_runtime(mcp_tools, "live")),
            ..Default::default()
        };
        let model = ModelProfile {
            slug: "glm-5.1".to_string(),
            max_tools: Some(3),
            ..Default::default()
        };

        let (_, tools, _) = tool_registry_for_options(&model, &options).expect("capped catalog");
        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["one.run", "two.run", "live.first"]);
        assert!(matches!(&tools[0].source, ToolSource::Plugin { .. }));
        assert!(matches!(&tools[1].source, ToolSource::Plugin { .. }));
        assert!(matches!(&tools[2].source, ToolSource::Mcp { .. }));
    }

    #[test]
    fn local_tool_runtime_preserves_definition_and_executable_resolver() {
        let mut tool = ToolDef::new(
            "example.echo",
            "local plugin echo",
            ToolCategory::Exec,
            ToolPermission::executes(),
        );
        tool.source = ToolSource::Plugin {
            name: "example".to_string(),
        };
        let local_resolver: Arc<dyn HandlerResolver> = Arc::new(|name: &str| {
            (name == "example.echo")
                .then(|| roko_std::tool::handlers::handler_for("read_file"))
                .flatten()
        });
        let options = AgentOptions {
            pre_discovered_local_tools: Some(Arc::new(LocalToolRuntime::new(
                vec![tool],
                local_resolver,
            ))),
            ..Default::default()
        };

        let (registry, tools, resolver) =
            tool_registry_for_options(&ModelProfile::default(), &options)
                .expect("local runtime should compose with provider registry");

        assert!(
            registry.get("example.echo").is_some(),
            "advertised tools: {:?}",
            tools.iter().map(|tool| &tool.name).collect::<Vec<_>>()
        );
        assert!(tools.iter().any(|tool| tool.name == "example.echo"));
        assert!(resolver.resolve("example.echo").is_some());
    }

    #[test]
    fn local_tool_runtime_fails_closed_when_handler_is_missing() {
        let mut tool = ToolDef::new(
            "orphan.run",
            "must not be advertised",
            ToolCategory::Exec,
            ToolPermission::executes(),
        );
        tool.source = ToolSource::Plugin {
            name: "orphan".to_string(),
        };
        let options = AgentOptions {
            pre_discovered_local_tools: Some(Arc::new(LocalToolRuntime::new(
                vec![tool],
                Arc::new(|_: &str| None),
            ))),
            ..Default::default()
        };

        let error = match tool_registry_for_options(&ModelProfile::default(), &options) {
            Ok(_) => panic!("handlerless local tool must not be advertised"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("orphan.run"));
    }

    #[test]
    fn structured_contract_filters_bridge_tool_registry_with_deny_precedence() {
        use crate::safety::contract::{AgentContract, GovernanceRule};

        let options = AgentOptions {
            agent_contract: Some(AgentContract {
                role: "known-role".into(),
                allowed_tools: Some(vec!["read_file".into(), "grep".into(), "bash".into()]),
                governance: vec![GovernanceRule::ForbiddenTools(vec!["bash".into()])],
                ..AgentContract::default()
            }),
            ..AgentOptions::default()
        };

        let (_, tools, _) = tool_registry_for_options(&ModelProfile::default(), &options)
            .expect("filtered registry");
        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep"]);
    }

    #[test]
    fn restricted_contract_advertises_no_bridge_tools() {
        use crate::safety::contract::AgentContract;

        let options = AgentOptions {
            agent_contract: Some(AgentContract::restricted("unknown-role")),
            ..AgentOptions::default()
        };

        let (_, tools, _) = tool_registry_for_options(&ModelProfile::default(), &options)
            .expect("deny-all registry");
        assert!(tools.is_empty(), "deny-all must advertise zero tools");
    }

    #[tokio::test]
    async fn openrouter_routing_injection() {
        let response = serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "openrouter-ok"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 4,
                "total_tokens": 13
            }
        })
        .to_string();
        let (poster, captured) = MockPoster::new(response);

        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            api_key_env: Some("PATH".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(1_500),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let model = ModelProfile {
            provider: "openrouter".to_string(),
            slug: "z-ai/glm-5.1".to_string(),
            context_window: 200_000,
            max_output: Some(1_024),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: Some(roko_core::config::schema::ProviderRouting {
                sort: Some("price".to_string()),
                order: Some(vec!["z-ai".to_string(), "moonshotai".to_string()]),
                allow_fallbacks: Some(true),
                max_price: Some(0.0025),
                require_parameters: Some(vec!["temperature".to_string()]),
            }),
            tool_format: "openai_json".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        };
        let options = AgentOptions {
            timeout_ms: Some(2_500),
            name: "openrouter-agent".to_string(),
            ..Default::default()
        };

        let mut extra_body_params = Map::new();
        inject_provider_routing(&mut extra_body_params, &provider, &model);

        let agent = CodexAgent::new("test-key", model.slug.clone())
            .with_base_url("https://openrouter.ai/api")
            .with_http_poster(poster)
            .with_extra_body_params(extra_body_params)
            .with_name(options.name.clone());

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap_or(""), "openrouter-ok");

        let request = captured
            .lock()
            .expect("capture lock")
            .clone()
            .expect("captured request");
        assert_eq!(request.url, "https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(request.timeout_ms, 120_000);
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("authorization")
                    && value == "Bearer test-key")
        );
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("content-type")
                    && value == "application/json")
        );
        let parsed: Value = serde_json::from_str(&request.body).expect("json request body");
        assert_eq!(
            parsed["provider"],
            serde_json::json!({
                "sort": "price",
                "order": ["z-ai", "moonshotai"],
                "allow_fallbacks": true,
                "max_price": 0.0025,
                "require_parameters": ["temperature"]
            })
        );
    }

    #[test]
    fn classify_error_maps_retry_after_and_auth() {
        let adapter = OpenAiCompatAdapter;
        let rate_limit = adapter.classify_error(429, &serde_json::json!({ "retry_after": 7 }));
        match rate_limit {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 7_000),
            other => panic!("unexpected rate limit classification: {other:?}"),
        }
        assert!(matches!(
            adapter.classify_error(401, &Value::Null),
            ProviderError::AuthFailure
        ));
    }

    #[test]
    fn zai_error_classify_maps_business_codes() {
        let adapter = OpenAiCompatAdapter;

        match adapter.classify_error(429, &serde_json::json!({ "error": { "code": "1302" } })) {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 5_000),
            other => panic!("unexpected Z.AI classification: {other:?}"),
        }

        assert!(matches!(
            adapter.classify_error(400, &serde_json::json!({ "error": { "code": "1261" } })),
            ProviderError::ContextOverflow
        ));
    }

    #[test]
    fn classify_error_400_context_overflow() {
        let adapter = OpenAiCompatAdapter;

        // OpenAI-style context_length_exceeded
        assert!(matches!(
            adapter.classify_error(
                400,
                &serde_json::json!({
                    "error": {
                        "message": "This model's maximum context length is 128000 tokens. However, your messages resulted in 130000 tokens. Please reduce the length of the messages.",
                        "type": "invalid_request_error",
                        "code": "context_length_exceeded"
                    }
                })
            ),
            ProviderError::ContextOverflow
        ));

        // Generic context + token message
        assert!(matches!(
            adapter.classify_error(
                400,
                &serde_json::json!({
                    "error": { "message": "context token limit exceeded" }
                })
            ),
            ProviderError::ContextOverflow
        ));

        // Non-context 400 should not be ContextOverflow
        assert!(!matches!(
            adapter.classify_error(
                400,
                &serde_json::json!({
                    "error": { "message": "invalid request format" }
                })
            ),
            ProviderError::ContextOverflow
        ));
    }

    // ── parse_allowed_tools_csv ───────────────────────────────────────────────

    #[test]
    fn parse_allowed_tools_csv_resolves_claude_names_read_write_edit() {
        let result = parse_allowed_tools_csv(Some("Read,Write,Edit"));
        let set = result.expect("should be Some");
        assert!(set.contains("read_file"), "Read -> read_file");
        assert!(set.contains("write_file"), "Write -> write_file");
        assert!(set.contains("edit_file"), "Edit -> edit_file");
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn parse_allowed_tools_csv_canonical_names_pass_through() {
        let result = parse_allowed_tools_csv(Some("read_file,bash"));
        let set = result.expect("should be Some");
        assert!(set.contains("read_file"));
        assert!(set.contains("bash"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn parse_allowed_tools_csv_mixed_names_work() {
        let result = parse_allowed_tools_csv(Some("Read,bash,Glob"));
        let set = result.expect("should be Some");
        assert!(set.contains("read_file"), "Read -> read_file");
        assert!(set.contains("bash"), "bash stays bash");
        assert!(set.contains("glob"), "Glob -> glob");
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn parse_allowed_tools_csv_unknown_names_pass_through() {
        let result = parse_allowed_tools_csv(Some("Read,my_mcp_tool"));
        let set = result.expect("should be Some");
        assert!(set.contains("read_file"), "Read -> read_file");
        assert!(set.contains("my_mcp_tool"), "unknown passes through");
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn parse_allowed_tools_csv_none_returns_none() {
        assert!(parse_allowed_tools_csv(None).is_none());
    }

    #[test]
    fn parse_allowed_tools_csv_empty_string_returns_none() {
        assert!(parse_allowed_tools_csv(Some("")).is_none());
    }
}
