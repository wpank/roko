+++
id = "bug-d0b8b8"
kind = "bug"
title = "tool_format hermes_json is ignored for openai_compat providers"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/providers"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs:509", "crates/roko-agent/src/translate/capability.rs::translator_for_profile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Arc::new(OpenAiTranslator)' crates/roko-agent/src/provider/openai_compat.rs && grep -q 'fn hermes_json_profile_uses_hermes_translator' crates/roko-agent/src/provider/openai_compat.rs && cargo test -p roko-agent --lib provider::openai_compat"
+++

The openai_compat provider always builds its tool loop with `Arc::new(OpenAiTranslator)` (`provider/openai_compat.rs:509`), whatever the model profile's `tool_format` says.
Models configured for `hermes_json` (text `<tool_call>` blocks) get OpenAI-native tool requests and their calls are never parsed; per a local audit `build_body` also rejects the system-prompt block the Hermes translator emits.
Fix: pick the translator from the resolved profile's `tool_format`, splice its system block into the system message, omit empty `tools`, and test each format.

Re-checked 2026-09-29: still open. The translator selector already exists as translate/capability.rs::translator_for_profile (and translator_for_capabilities); the fix should call it at openai_compat.rs:509 instead of hard-coding OpenAiTranslator. The existing openai_compat tests all use tool_format = "openai_json", so the current verify command passes while the bug is present.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  - The `openai_compat` adapter takes its translator from `capability::translator_for_openai_compat(model)`:
    - Hermes `<tool_call>` text for a `hermes_json` profile;
    - `OpenAiTranslator` for every native format, since the endpoint speaks the OpenAI wire. `translator_for_profile`
      would map `openai_json` to the Ollama translator, so it is not used here.
  - `OpenAiCompatLlmBackend::build_body` accepts a `SystemPromptBlock`. The block is appended to the system message,
    or becomes the first message when there is none, and the request then carries no `tools`. An empty native `tools`
    array is omitted, and `parallel_tool_calls` is sent only with tools.
  - Tests:
    - `hermes_json_profile_uses_hermes_translator`: a mock endpoint; the `<tool_call>` text runs and a
      `<tool_response>` goes back;
    - `build_body_puts_a_system_prompt_tool_block_in_the_system_message`;
    - `translator_for_openai_compat_follows_hermes_json_only`.
  - Behaviour change: a profile synthesized from its slug takes the format registry's preferred format, so `qwen3*`
    and `qwen2*` slugs (`hermes_json`) on an `openai_compat` provider now use Hermes text instead of native tools.
    Models declared in `roko.toml` default to `openai_json` and are unaffected.
  - Not fixed, reported: `HermesXmlTranslator::render_assistant_message` returns `None`, as `ReActTranslator`'s does,
    so the tool loop never puts the model's own `<tool_call>` turn into the history. The next request carries only
    the `<tool_response>`.
