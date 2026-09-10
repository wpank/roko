//! Content block conversion, image handling, file context resolution,
//! and prompt text extraction.

use std::path::{Path, PathBuf};

use roko_core::agent::ProviderKind;
use roko_core::foundation::{
    MessageRole, ModelInputBlock, ModelInputMessage, validate_model_input_messages,
};
use tracing::warn;

use crate::types::ContentBlock;

pub(crate) fn extract_prompt_text(prompt: &[ContentBlock]) -> String {
    prompt
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.clone(),
            ContentBlock::Resource { .. } => String::new(),
            ContentBlock::Image { mime_type, .. } => format!("[image: {mime_type}]"),
            ContentBlock::Diff { path, diff, .. } => {
                format!("diff {path}:\n{}", diff.as_deref().unwrap_or(""))
            }
            ContentBlock::Unknown => {
                tracing::debug!("skipping unknown content block type in prompt");
                String::new()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn model_input_blocks_from_prompt(prompt: &[ContentBlock]) -> Vec<ModelInputBlock> {
    prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } if !text.is_empty() => {
                Some(ModelInputBlock::text(text.clone()))
            }
            ContentBlock::Image { data, mime_type } => {
                Some(ModelInputBlock::image(mime_type.clone(), data.clone()))
            }
            ContentBlock::Diff { path, diff, .. } => Some(ModelInputBlock::text(format!(
                "diff {path}:\n{}",
                diff.as_deref().unwrap_or("")
            ))),
            ContentBlock::Text { .. } | ContentBlock::Resource { .. } | ContentBlock::Unknown => {
                None
            }
        })
        .collect()
}

/// Builds an OpenAI-compatible content array from prompt blocks, converting
/// `Image` blocks into `image_url` content parts with inline data URIs.
/// Returns `None` when no images are present (caller can use a plain string).
pub(crate) fn build_openai_content_parts(prompt: &[ContentBlock]) -> Option<Vec<serde_json::Value>> {
    let has_image = prompt
        .iter()
        .any(|b| matches!(b, ContentBlock::Image { .. }));
    if !has_image {
        return None;
    }
    let mut parts = Vec::new();
    for block in model_input_blocks_from_prompt(prompt) {
        match block {
            ModelInputBlock::Text { text } => {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            ModelInputBlock::Image { data, media_type } => {
                parts.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": { "url": format!("data:{media_type};base64,{data}") }
                }));
            }
        }
    }
    Some(parts)
}

/// Converts prompt blocks into Anthropic multi-part content (text + base64 image).
/// Returns `None` when there are no image blocks, so the caller can skip replacement.
pub(crate) fn build_anthropic_content_parts(prompt: &[ContentBlock]) -> Option<Vec<serde_json::Value>> {
    let has_image = prompt
        .iter()
        .any(|b| matches!(b, ContentBlock::Image { .. }));
    if !has_image {
        return None;
    }
    let mut parts = Vec::new();
    for block in model_input_blocks_from_prompt(prompt) {
        match block {
            ModelInputBlock::Text { text } => {
                parts.push(serde_json::json!({"type": "text", "text": text}));
            }
            ModelInputBlock::Image { data, media_type } => {
                parts.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": data,
                    }
                }));
            }
        }
    }
    Some(parts)
}

/// Replaces the last user message's `content` field in `msgs` with a multi-part
/// content array when `prompt` contains `Image` blocks.  No-ops when there are
/// no image blocks so text-only prompts are unaffected.
pub(crate) fn inject_image_parts(
    msgs: &mut [serde_json::Value],
    prompt: &[ContentBlock],
    provider_kind: ProviderKind,
) {
    let mut image_parts = if provider_kind == ProviderKind::AnthropicApi {
        build_anthropic_content_parts(prompt)
    } else {
        build_openai_content_parts(prompt)
    };
    if let Some(last) = msgs.last_mut()
        && last.get("role").and_then(|v| v.as_str()) == Some("user")
        && let Some(parts) = image_parts.as_mut()
    {
        let prompt_text = extract_prompt_text(prompt);
        if let Some(existing) = last.get("content").and_then(serde_json::Value::as_str)
            && let Some(suffix) = existing.strip_prefix(&prompt_text)
            && !suffix.is_empty()
        {
            parts.push(serde_json::json!({"type": "text", "text": suffix}));
        }
        last["content"] = serde_json::Value::Array(std::mem::take(parts));
    }
}

pub(crate) fn model_input_messages_from_wire(
    messages: &[serde_json::Value],
) -> std::result::Result<Vec<ModelInputMessage>, String> {
    let mut structured = Vec::with_capacity(messages.len());
    for (message_index, message) in messages.iter().enumerate() {
        let role = match message.get("role").and_then(serde_json::Value::as_str) {
            Some("system") => MessageRole::System,
            Some("user") => MessageRole::User,
            Some("assistant") => MessageRole::Assistant,
            Some(other) => {
                return Err(format!(
                    "message {} has unsupported role {other:?}",
                    message_index + 1
                ));
            }
            None => return Err(format!("message {} has no role", message_index + 1)),
        };
        let content = message
            .get("content")
            .ok_or_else(|| format!("message {} has no content", message_index + 1))?;
        let blocks = if let Some(text) = content.as_str() {
            vec![ModelInputBlock::text(text)]
        } else if let Some(parts) = content.as_array() {
            parts
                .iter()
                .enumerate()
                .map(|(part_index, part)| {
                    match part.get("type").and_then(serde_json::Value::as_str) {
                        Some("text") => part
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .map(|text| ModelInputBlock::text(text.to_string()))
                            .ok_or_else(|| {
                                format!(
                                    "message {} part {} has no text",
                                    message_index + 1,
                                    part_index + 1
                                )
                            }),
                        Some("image") => {
                            let source = part.get("source").ok_or_else(|| {
                                format!(
                                    "message {} image part {} has no source",
                                    message_index + 1,
                                    part_index + 1
                                )
                            })?;
                            let media_type = source
                                .get("media_type")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "Anthropic image has no media_type".to_string())?;
                            let data = source
                                .get("data")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "Anthropic image has no data".to_string())?;
                            Ok(ModelInputBlock::image(media_type, data))
                        }
                        Some("image_url") => {
                            let uri = part
                                .pointer("/image_url/url")
                                .and_then(serde_json::Value::as_str)
                                .ok_or_else(|| "OpenAI image has no data URI".to_string())?;
                            let encoded = uri.strip_prefix("data:").ok_or_else(|| {
                                "OpenAI image URL is not an inline data URI".to_string()
                            })?;
                            let (media_type, data) =
                                encoded.split_once(";base64,").ok_or_else(|| {
                                    "OpenAI image data URI is not base64 encoded".to_string()
                                })?;
                            Ok(ModelInputBlock::image(media_type, data))
                        }
                        Some(other) => Err(format!(
                            "message {} part {} has unsupported type {other:?}",
                            message_index + 1,
                            part_index + 1
                        )),
                        None => Err(format!(
                            "message {} part {} has no type",
                            message_index + 1,
                            part_index + 1
                        )),
                    }
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            return Err(format!(
                "message {} content is neither text nor an array",
                message_index + 1
            ));
        };
        structured.push(ModelInputMessage::new(role, blocks));
    }
    validate_model_input_messages(&structured)?;
    Ok(structured)
}

/// Extracts `file://` URIs from Resource blocks in the prompt.
pub(crate) fn extract_resource_uris(prompt: &[ContentBlock]) -> Vec<String> {
    use crate::types::ResourceRef;
    prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Resource {
                resource: ResourceRef::File { uri },
            } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

/// Reads file contents for the given URIs, returning XML-tagged file context.
/// Validates that paths stay within the workdir for security.
pub(crate) fn read_file_context(uris: &[String], workdir: &Path) -> String {
    let mut context = String::new();
    let workdir_canonical = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());

    for uri in uris {
        let path_str = uri.strip_prefix("file://").unwrap_or(uri);
        let path = PathBuf::from(path_str);

        // Security: ensure path is within workdir.
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !canonical.starts_with(&workdir_canonical) {
            warn!(path = %path.display(), "skipping file outside workdir");
            continue;
        }

        match std::fs::read_to_string(&canonical) {
            Ok(contents) => {
                // Cap individual file at 32KB to avoid blowing up context.
                let truncated = truncate_with_limit(&contents, 32_768, "... [truncated at 32KB]");
                let rel_path = canonical
                    .strip_prefix(&workdir_canonical)
                    .unwrap_or(&canonical);
                context.push_str(&format!(
                    "<file path=\"{}\">\n{}\n</file>\n",
                    rel_path.display(),
                    truncated
                ));
            }
            Err(e) => {
                warn!(path = %canonical.display(), error = %e, "failed to read file for context");
            }
        }
    }

    context
}

/// Resolves context annotations in prompt blocks into a single context string.
///
/// Explicit file attachments are resolved as XML-tagged file content. Text
/// blocks are scanned for `@` mentions and each supported mention is resolved
/// to either git context or file content.
pub(crate) async fn resolve_context_items(prompt: &[ContentBlock], workdir: &Path) -> String {
    use crate::types::ResourceRef;

    let mut parts = Vec::new();

    for block in prompt {
        match block {
            ContentBlock::Resource {
                resource: ResourceRef::File { uri },
            } => match resolve_file_uri(uri, workdir).await {
                Ok(content) => parts.push(content),
                Err(error) => {
                    warn!(uri = %uri, error = %error, "failed to resolve file resource URI");
                }
            },
            ContentBlock::Text { text } => {
                for label in extract_at_mentions(text) {
                    match resolve_at_mention(&label, workdir).await {
                        Ok(content) => parts.push(content),
                        Err(error) => {
                            warn!(label = %label, error = %error, "failed to resolve @-mention");
                        }
                    }
                }
            }
            ContentBlock::Image { .. } | ContentBlock::Diff { .. } => {}
            ContentBlock::Unknown => {
                tracing::debug!("skipping unknown content block in context resolution");
            }
        }
    }

    parts.join("\n\n")
}

pub(crate) async fn resolve_file_uri(uri: &str, workdir: &Path) -> anyhow::Result<String> {
    let path_str = uri.strip_prefix("file://").unwrap_or(uri);
    let (rel_path, contents) = resolve_local_file_contents(Path::new(path_str), workdir).await?;
    Ok(format!(
        "<file path=\"{}\">\n{}\n</file>",
        rel_path.display(),
        contents
    ))
}

pub(crate) async fn resolve_at_mention(label: &str, workdir: &Path) -> anyhow::Result<String> {
    match label {
        "branch-diff" | "diff" => {
            let output = tokio::process::Command::new("git")
                .args(["diff"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git diff")?;
            let diff = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&diff, 10_240, "...\n[truncated]");
            Ok(format!("--- branch diff ---\n{truncated}"))
        }
        "recent-commits" | "git-log" | "log" => {
            let output = tokio::process::Command::new("git")
                .args(["log", "--oneline", "-20"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git log")?;
            let log = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&log, 10_240, "...\n[truncated]");
            Ok(format!("--- recent commits ---\n{truncated}"))
        }
        "status" | "git-status" => {
            let output = tokio::process::Command::new("git")
                .args(["status", "--short"])
                .current_dir(workdir)
                .output()
                .await?;
            ensure_git_output_success(&output, "git status")?;
            let status = String::from_utf8_lossy(&output.stdout);
            let truncated = truncate_with_limit(&status, 10_240, "...\n[truncated]");
            Ok(format!("--- git status ---\n{truncated}"))
        }
        _ => {
            let (rel_path, contents) =
                resolve_local_file_contents(Path::new(label), workdir).await?;
            Ok(format!("--- {} ---\n{contents}", rel_path.display()))
        }
    }
}

pub(crate) async fn resolve_local_file_contents(
    path: &Path,
    workdir: &Path,
) -> anyhow::Result<(PathBuf, String)> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workdir.join(path)
    };

    let workdir_canonical = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let canonical = full_path
        .canonicalize()
        .map_err(|error| anyhow::anyhow!("cannot canonicalize {}: {error}", full_path.display()))?;
    if !canonical.starts_with(&workdir_canonical) {
        return Err(anyhow::anyhow!(
            "path {} is outside workdir",
            canonical.display()
        ));
    }

    let contents = tokio::fs::read_to_string(&canonical).await?;
    let rel_path = canonical
        .strip_prefix(&workdir_canonical)
        .unwrap_or(&canonical)
        .to_path_buf();
    let truncated = truncate_with_limit(&contents, 32_768, "... [truncated at 32KB]");

    Ok((rel_path, truncated))
}

pub(crate) fn extract_at_mentions(text: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut search_start = 0;

    while let Some(relative_at) = text[search_start..].find('@') {
        let at_index = search_start + relative_at;
        let prev = text[..at_index].chars().next_back();
        if matches!(prev, Some(c) if c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
            search_start = at_index + 1;
            continue;
        }

        let mut end = at_index + 1;
        while end < text.len() {
            let Some(ch) = text[end..].chars().next() else { break; };
            if ch.is_whitespace()
                || ch == '@'
                || matches!(
                    ch,
                    ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '<' | '>'
                )
            {
                break;
            }
            end += ch.len_utf8();
        }

        let label = text[at_index + 1..end].trim_matches(|ch: char| {
            matches!(
                ch,
                ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '<' | '>' | '\'' | '"'
            )
        });
        if !label.is_empty() && !label.starts_with('@') {
            mentions.push(label.to_owned());
        }

        search_start = end;
    }

    mentions
}

pub(crate) fn truncate_with_limit(text: &str, limit: usize, suffix: &str) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }

    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = String::with_capacity(end + suffix.len());
    truncated.push_str(&text[..end]);
    truncated.push_str(suffix);
    truncated
}

pub(crate) fn ensure_git_output_success(output: &std::process::Output, command: &str) -> anyhow::Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        Err(anyhow::anyhow!("{command} failed"))
    } else {
        Err(anyhow::anyhow!("{command} failed: {stderr}"))
    }
}

