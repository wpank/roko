# 14: Image Support for Roko Agents

## Status: Implemented for vision-capable HTTP providers (2026-08-16)

ACP image prompts now retain the actual image bytes and ordered text/image
semantics through both direct dispatch and the current workflow engine. The
implementation no longer advertises image support solely because a model is
vision-capable: the selected provider transport must also support inline image
payloads.

## Supported paths

| Provider transport | Wire representation | Status |
|---|---|---|
| `anthropic_api` | Anthropic base64 `image/source` blocks | Supported |
| `openai_compat` | OpenAI `image_url` data-URI parts | Supported |
| `gemini_api` | Gemini-native `inlineData` parts (or OpenAI-compatible mode) | Supported |

The same ordered canonical request is used by plain calls and provider-native
tool loops. Structured system messages are lifted correctly for Anthropic and
Gemini instead of being silently discarded.

## Validation and safety

Image input fails before provider I/O when any of these conditions is true:

- the model does not declare `supports_vision`;
- the configured provider transport cannot carry inline images;
- the MIME type is not PNG, JPEG, GIF, or WebP;
- base64 is malformed or empty;
- an image exceeds 5 MiB decoded, the request exceeds 20 images, or aggregate
  decoded image data exceeds 20 MiB;
- an image appears on a non-user message.

Cache identity includes block ordering, MIME type, and image bytes. Custom
`Debug` output redacts base64 payloads, and user-facing placeholders/logging
contain only the MIME type.

## Deliberate boundaries

CLI/subprocess transports (`claude_cli`, Gemini CLI, Cursor CLI/ACP), Hermes,
OpenClaw, Perplexity, Cerebras, and unconfigured exec fallbacks do not currently
have a truthful inline-image contract. They reject image input even if a model
slug is known to support vision. Consequently, the default `claude_cli` ACP
configuration advertises `image: false`.

The legacy ACP workflow selected with `ROKO_ACP_LEGACY` also rejects images.
The current workflow engine supports them and prefixes each phase's text
instructions without changing the original text/image block order.

Audio remains unsupported and is advertised as `false`.

## Evidence

Focused tests cover:

- strict MIME/base64/count/per-image/aggregate limits and payload redaction;
- exact Anthropic, OpenAI, and Gemini bytes and block order;
- plain and tool-enabled provider serialization;
- unsupported/invalid input failing before an HTTP poster is called;
- cache-key changes for byte, MIME, and ordering changes;
- ACP text/image/diff conversion and wire round trips;
- workflow phase-prefix preservation with the original multimodal order.
