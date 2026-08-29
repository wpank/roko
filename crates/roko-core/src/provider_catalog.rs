//! Built-in provider catalog with known providers and their defaults.
//!
//! This module provides a static catalog of known LLM providers with
//! pre-filled configuration defaults (base URLs, API key env var names,
//! model slugs, context windows, cost data, and capability flags).
//!
//! Operators can use `roko config providers catalog` to browse known providers
//! and `roko config providers discover` to scan the environment for API keys.

use crate::agent::ProviderKind;

/// A known provider in the built-in catalog.
#[derive(Debug, Clone)]
pub struct ProviderCatalogEntry {
    /// Short identifier (e.g. "deepseek").
    pub id: &'static str,
    /// Human-readable display name (e.g. "DeepSeek").
    pub display_name: &'static str,
    /// Provider kind for roko config.
    pub kind: ProviderKind,
    /// Base URL for the API.
    pub base_url: &'static str,
    /// Conventional environment variable name for the API key.
    pub api_key_env: &'static str,
    /// Default models available from this provider.
    pub models: &'static [CatalogModel],
}

/// A model entry in the provider catalog.
#[derive(Debug, Clone)]
pub struct CatalogModel {
    /// Model slug (e.g. "deepseek-chat").
    pub slug: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Context window in tokens.
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_output: u64,
    /// Whether the model supports tool/function calling.
    pub supports_tools: bool,
    /// Whether the model supports extended thinking.
    pub supports_thinking: bool,
    /// Whether the model supports vision/image input.
    pub supports_vision: bool,
    /// Cost per million input tokens (USD).
    pub cost_input_per_m: f64,
    /// Cost per million output tokens (USD).
    pub cost_output_per_m: f64,
}

/// Status of a provider in the user's environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAvailability {
    /// API key found in the environment.
    KeyFound,
    /// API key env var not set.
    KeyMissing,
    /// Local service (no key needed) — availability unchecked.
    Local,
    /// Already configured in roko.toml.
    Configured,
}

impl std::fmt::Display for ProviderAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyFound => write!(f, "key found"),
            Self::KeyMissing => write!(f, "not set"),
            Self::Local => write!(f, "local"),
            Self::Configured => write!(f, "configured"),
        }
    }
}

/// Check whether a provider's API key env var is set.
pub fn check_provider_availability(entry: &ProviderCatalogEntry) -> ProviderAvailability {
    if entry.api_key_env.is_empty() {
        return ProviderAvailability::Local;
    }
    match std::env::var(entry.api_key_env) {
        Ok(val) if !val.is_empty() => ProviderAvailability::KeyFound,
        _ => ProviderAvailability::KeyMissing,
    }
}

/// Return the full built-in provider catalog.
pub fn catalog() -> &'static [ProviderCatalogEntry] {
    CATALOG
}

/// Look up a provider by its catalog ID.
pub fn lookup(id: &str) -> Option<&'static ProviderCatalogEntry> {
    CATALOG.iter().find(|e| e.id == id)
}

/// Scan the environment and return all providers with their availability.
pub fn discover() -> Vec<(&'static ProviderCatalogEntry, ProviderAvailability)> {
    CATALOG
        .iter()
        .map(|entry| (entry, check_provider_availability(entry)))
        .collect()
}

// ---------------------------------------------------------------------------
// Static catalog data
// ---------------------------------------------------------------------------

static CATALOG: &[ProviderCatalogEntry] = &[
    ProviderCatalogEntry {
        id: "anthropic",
        display_name: "Anthropic",
        kind: ProviderKind::AnthropicApi,
        base_url: "https://api.anthropic.com",
        api_key_env: "ANTHROPIC_API_KEY",
        models: &[
            CatalogModel {
                slug: "claude-sonnet-4-6",
                display_name: "Claude Sonnet 4.6",
                context_window: 200_000,
                max_output: 16_000,
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                cost_input_per_m: 3.0,
                cost_output_per_m: 15.0,
            },
            CatalogModel {
                slug: "claude-opus-4-6",
                display_name: "Claude Opus 4.6",
                context_window: 200_000,
                max_output: 32_000,
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                cost_input_per_m: 15.0,
                cost_output_per_m: 75.0,
            },
            CatalogModel {
                slug: "claude-haiku-3-5",
                display_name: "Claude Haiku 3.5",
                context_window: 200_000,
                max_output: 8192,
                supports_tools: true,
                supports_thinking: false,
                supports_vision: true,
                cost_input_per_m: 0.80,
                cost_output_per_m: 4.0,
            },
        ],
    },
    ProviderCatalogEntry {
        id: "openai",
        display_name: "OpenAI",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.openai.com/v1",
        api_key_env: "OPENAI_API_KEY",
        models: &[
            CatalogModel {
                slug: "gpt-4.1",
                display_name: "GPT-4.1",
                context_window: 1_000_000,
                max_output: 32_768,
                supports_tools: true,
                supports_thinking: false,
                supports_vision: true,
                cost_input_per_m: 2.0,
                cost_output_per_m: 8.0,
            },
            CatalogModel {
                slug: "gpt-4.1-mini",
                display_name: "GPT-4.1 Mini",
                context_window: 1_000_000,
                max_output: 16_384,
                supports_tools: true,
                supports_thinking: false,
                supports_vision: true,
                cost_input_per_m: 0.40,
                cost_output_per_m: 1.60,
            },
            CatalogModel {
                slug: "o3-mini",
                display_name: "o3-mini",
                context_window: 200_000,
                max_output: 100_000,
                supports_tools: true,
                supports_thinking: true,
                supports_vision: false,
                cost_input_per_m: 1.10,
                cost_output_per_m: 4.40,
            },
        ],
    },
    ProviderCatalogEntry {
        id: "deepseek",
        display_name: "DeepSeek",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.deepseek.com/v1",
        api_key_env: "DEEPSEEK_API_KEY",
        models: &[CatalogModel {
            slug: "deepseek-chat",
            display_name: "DeepSeek V3",
            context_window: 128_000,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            cost_input_per_m: 0.27,
            cost_output_per_m: 1.10,
        }],
    },
    ProviderCatalogEntry {
        id: "gemini",
        display_name: "Google Gemini",
        kind: ProviderKind::GeminiApi,
        base_url: "https://generativelanguage.googleapis.com/v1beta",
        api_key_env: "GEMINI_API_KEY",
        models: &[
            CatalogModel {
                slug: "gemini-2.5-flash",
                display_name: "Gemini 2.5 Flash",
                context_window: 1_000_000,
                max_output: 65_536,
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                cost_input_per_m: 0.15,
                cost_output_per_m: 0.60,
            },
            CatalogModel {
                slug: "gemini-2.5-pro",
                display_name: "Gemini 2.5 Pro",
                context_window: 1_000_000,
                max_output: 65_536,
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                cost_input_per_m: 1.25,
                cost_output_per_m: 10.0,
            },
        ],
    },
    ProviderCatalogEntry {
        id: "xai",
        display_name: "xAI Grok",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.x.ai/v1",
        api_key_env: "XAI_API_KEY",
        models: &[CatalogModel {
            slug: "grok-3",
            display_name: "Grok 3",
            context_window: 131_072,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: true,
            supports_vision: true,
            cost_input_per_m: 3.0,
            cost_output_per_m: 15.0,
        }],
    },
    ProviderCatalogEntry {
        id: "perplexity",
        display_name: "Perplexity",
        kind: ProviderKind::PerplexityApi,
        base_url: "https://api.perplexity.ai",
        api_key_env: "PERPLEXITY_API_KEY",
        models: &[CatalogModel {
            slug: "sonar-pro",
            display_name: "Sonar Pro",
            context_window: 200_000,
            max_output: 8192,
            supports_tools: false,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 3.0,
            cost_output_per_m: 15.0,
        }],
    },
    ProviderCatalogEntry {
        id: "cerebras",
        display_name: "Cerebras",
        kind: ProviderKind::CerebrasApi,
        base_url: "https://api.cerebras.ai/v1",
        api_key_env: "CEREBRAS_API_KEY",
        models: &[CatalogModel {
            slug: "llama-3.3-70b",
            display_name: "Llama 3.3 70B",
            context_window: 128_000,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.0,
            cost_output_per_m: 0.0,
        }],
    },
    ProviderCatalogEntry {
        id: "openrouter",
        display_name: "OpenRouter",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://openrouter.ai/api/v1",
        api_key_env: "OPENROUTER_API_KEY",
        models: &[CatalogModel {
            slug: "openrouter/auto",
            display_name: "Auto (best available)",
            context_window: 200_000,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: true,
            cost_input_per_m: 0.0,
            cost_output_per_m: 0.0,
        }],
    },
    ProviderCatalogEntry {
        id: "fireworks",
        display_name: "Fireworks AI",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.fireworks.ai/inference/v1",
        api_key_env: "FIREWORKS_API_KEY",
        models: &[CatalogModel {
            slug: "accounts/fireworks/models/llama-v3p3-70b-instruct",
            display_name: "Llama 3.3 70B",
            context_window: 131_072,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.90,
            cost_output_per_m: 0.90,
        }],
    },
    ProviderCatalogEntry {
        id: "together",
        display_name: "Together AI",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.together.xyz/v1",
        api_key_env: "TOGETHER_API_KEY",
        models: &[CatalogModel {
            slug: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            display_name: "Llama 3.3 70B Turbo",
            context_window: 131_072,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.88,
            cost_output_per_m: 0.88,
        }],
    },
    ProviderCatalogEntry {
        id: "moonshot",
        display_name: "Kimi / Moonshot",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.moonshot.cn/v1",
        api_key_env: "MOONSHOT_API_KEY",
        models: &[CatalogModel {
            slug: "moonshot-v1-128k",
            display_name: "Moonshot V1 128K",
            context_window: 128_000,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.84,
            cost_output_per_m: 0.84,
        }],
    },
    ProviderCatalogEntry {
        id: "ollama",
        display_name: "Ollama (local)",
        kind: ProviderKind::OpenAiCompat,
        base_url: "http://localhost:11434/v1",
        api_key_env: "",
        models: &[CatalogModel {
            slug: "llama3.3",
            display_name: "Llama 3.3 (local)",
            context_window: 128_000,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.0,
            cost_output_per_m: 0.0,
        }],
    },
    ProviderCatalogEntry {
        id: "lmstudio",
        display_name: "LM Studio (local)",
        kind: ProviderKind::OpenAiCompat,
        base_url: "http://localhost:1234/v1",
        api_key_env: "",
        models: &[CatalogModel {
            slug: "local-model",
            display_name: "Local Model",
            context_window: 32_000,
            max_output: 4096,
            supports_tools: false,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.0,
            cost_output_per_m: 0.0,
        }],
    },
    ProviderCatalogEntry {
        id: "nvidia",
        display_name: "NVIDIA NIM",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://integrate.api.nvidia.com/v1",
        api_key_env: "NVIDIA_API_KEY",
        models: &[CatalogModel {
            slug: "meta/llama-3.3-70b-instruct",
            display_name: "Llama 3.3 70B",
            context_window: 128_000,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.0,
            cost_output_per_m: 0.0,
        }],
    },
    ProviderCatalogEntry {
        id: "deepinfra",
        display_name: "DeepInfra",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.deepinfra.com/v1/openai",
        api_key_env: "DEEPINFRA_API_KEY",
        models: &[CatalogModel {
            slug: "meta-llama/Llama-3.3-70B-Instruct",
            display_name: "Llama 3.3 70B",
            context_window: 131_072,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.35,
            cost_output_per_m: 0.40,
        }],
    },
    ProviderCatalogEntry {
        id: "zhipu",
        display_name: "Z.AI / Zhipu",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        api_key_env: "ZHIPU_API_KEY",
        models: &[CatalogModel {
            slug: "glm-4-plus",
            display_name: "GLM-4 Plus",
            context_window: 128_000,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 0.70,
            cost_output_per_m: 0.70,
        }],
    },
    ProviderCatalogEntry {
        id: "qwen",
        display_name: "Qwen (Alibaba)",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        api_key_env: "DASHSCOPE_API_KEY",
        models: &[CatalogModel {
            slug: "qwen-plus",
            display_name: "Qwen Plus",
            context_window: 131_072,
            max_output: 8192,
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            cost_input_per_m: 0.80,
            cost_output_per_m: 2.0,
        }],
    },
    ProviderCatalogEntry {
        id: "minimax",
        display_name: "MiniMax",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.minimax.chat/v1",
        api_key_env: "MINIMAX_API_KEY",
        models: &[CatalogModel {
            slug: "MiniMax-Text-01",
            display_name: "MiniMax Text 01",
            context_window: 1_000_000,
            max_output: 16_384,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 1.0,
            cost_output_per_m: 1.0,
        }],
    },
    ProviderCatalogEntry {
        id: "stepfun",
        display_name: "StepFun",
        kind: ProviderKind::OpenAiCompat,
        base_url: "https://api.stepfun.com/v1",
        api_key_env: "STEPFUN_API_KEY",
        models: &[CatalogModel {
            slug: "step-2-16k",
            display_name: "Step 2 16K",
            context_window: 16_384,
            max_output: 4096,
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            cost_input_per_m: 1.38,
            cost_output_per_m: 5.60,
        }],
    },
];
