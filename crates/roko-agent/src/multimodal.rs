//! Provider-wire conversion for the provider-neutral multimodal request contract.

use roko_core::{MessageRole, ModelInputBlock, ModelInputMessage};
use serde_json::{Value, json};

fn wire_role(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    }
}

/// Return whether a structured history contains at least one image.
pub(crate) fn contains_images(messages: &[ModelInputMessage]) -> bool {
    messages.iter().any(|message| {
        message
            .content
            .iter()
            .any(|block| matches!(block, ModelInputBlock::Image { .. }))
    })
}

/// Detect inline image blocks in any provider wire representation used by the
/// shared tool loop.
pub(crate) fn wire_messages_contain_images(messages: &[Value]) -> bool {
    messages.iter().any(|message| {
        message
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|parts| {
                parts.iter().any(|part| {
                    matches!(
                        part.get("type").and_then(Value::as_str),
                        Some("image" | "image_url")
                    ) || part.get("inlineData").is_some()
                })
            })
            || message
                .get("parts")
                .and_then(Value::as_array)
                .is_some_and(|parts| parts.iter().any(|part| part.get("inlineData").is_some()))
    })
}

/// Convert ordered canonical messages to Anthropic Messages API blocks.
pub(crate) fn anthropic_messages(messages: &[ModelInputMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            let content = message
                .content
                .iter()
                .map(|block| match block {
                    ModelInputBlock::Text { text } => json!({
                        "type": "text",
                        "text": text,
                    }),
                    ModelInputBlock::Image { media_type, data } => json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": data,
                        }
                    }),
                })
                .collect::<Vec<_>>();
            json!({
                "role": wire_role(message.role),
                "content": content,
            })
        })
        .collect()
}

/// Convert ordered canonical messages to OpenAI-compatible content parts.
pub(crate) fn openai_messages(messages: &[ModelInputMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            let content = message
                .content
                .iter()
                .map(|block| match block {
                    ModelInputBlock::Text { text } => json!({
                        "type": "text",
                        "text": text,
                    }),
                    ModelInputBlock::Image { media_type, data } => json!({
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{media_type};base64,{data}"),
                        }
                    }),
                })
                .collect::<Vec<_>>();
            json!({
                "role": wire_role(message.role),
                "content": content,
            })
        })
        .collect()
}

/// Convert ordered canonical messages to Gemini-native `parts` messages.
pub(crate) fn gemini_messages(messages: &[ModelInputMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            if message.role == MessageRole::System {
                let text = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ModelInputBlock::Text { text } => Some(text.as_str()),
                        ModelInputBlock::Image { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                return json!({"role": "system", "content": text});
            }
            let parts = message
                .content
                .iter()
                .map(|block| match block {
                    ModelInputBlock::Text { text } => json!({"text": text}),
                    ModelInputBlock::Image { media_type, data } => json!({
                        "inlineData": {
                            "mimeType": media_type,
                            "data": data,
                        }
                    }),
                })
                .collect::<Vec<_>>();
            json!({
                "role": if message.role == MessageRole::Assistant { "model" } else { "user" },
                "parts": parts,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordered_input() -> Vec<ModelInputMessage> {
        vec![ModelInputMessage::new(
            MessageRole::User,
            vec![
                ModelInputBlock::text("before"),
                ModelInputBlock::image("image/png", "aGVsbG8="),
                ModelInputBlock::text("after"),
            ],
        )]
    }

    #[test]
    fn provider_conversions_preserve_block_order_and_bytes() {
        let anthropic = anthropic_messages(&ordered_input());
        assert_eq!(anthropic[0]["content"][0]["text"], "before");
        assert_eq!(anthropic[0]["content"][1]["source"]["data"], "aGVsbG8=");
        assert_eq!(anthropic[0]["content"][2]["text"], "after");

        let openai = openai_messages(&ordered_input());
        assert_eq!(openai[0]["content"][0]["text"], "before");
        assert_eq!(
            openai[0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,aGVsbG8="
        );
        assert_eq!(openai[0]["content"][2]["text"], "after");

        let gemini = gemini_messages(&ordered_input());
        assert_eq!(gemini[0]["parts"][0]["text"], "before");
        assert_eq!(gemini[0]["parts"][1]["inlineData"]["data"], "aGVsbG8=");
        assert_eq!(gemini[0]["parts"][2]["text"], "after");

        assert!(wire_messages_contain_images(&anthropic));
        assert!(wire_messages_contain_images(&openai));
        assert!(wire_messages_contain_images(&gemini));
    }
}
