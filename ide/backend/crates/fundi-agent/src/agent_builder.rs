//! Shared agent construction functions.
//!
//! These helpers build an [`AgentConfig`], LLM provider, and tool-registry
//! bridge from a [`GatewayConfig`] + model string.  Both the CLI (`app.rs`)
//! and the Dashboard (`lib.rs`) import from here so the logic lives in one
//! place.

use std::sync::Arc;

use futures::StreamExt;
use serde_json::Value;

use fundi_config::GatewayConfig;
use fundi_core::{AgentError, LlmProvider};
use fundi_tools::ToolRegistry;

use crate::agent_loop::{
    CheapModelRouteConfig, RuntimeProviderConfig, SmartModelRoutingConfig,
    ToolRegistry as AgentToolRegistry,
};
use crate::provider::{AnthropicProvider, GenericProvider, OpenAiProvider, OpenRouterProvider};
use crate::providers_extra::{
    CopilotProvider, KimiProvider, MiniMaxProvider, NousProvider, QwenProvider,
};
use crate::AgentConfig;

// ---------------------------------------------------------------------------
// parse_model_string
// ---------------------------------------------------------------------------

/// Split a `"provider:model"` string into `(provider, model)`.
///
/// If no colon is present the provider defaults to `"openai"`.
pub fn parse_model_string(model: &str) -> (&str, &str) {
    model.split_once(':').unwrap_or(("openai", model))
}

// ---------------------------------------------------------------------------
// build_agent_config
// ---------------------------------------------------------------------------

/// Build an [`AgentConfig`] from a [`GatewayConfig`] and a model string.
///
/// `platform` lets callers tag the config with their origin (`"cli"`,
/// `"http"`, `"gateway"`, …).  When `None` the field is left unset.
pub fn build_agent_config(
    config: &GatewayConfig,
    model: &str,
    platform: Option<&str>,
) -> AgentConfig {
    let provider_from_model = model.split_once(':').map(|(p, _)| p.to_string());
    AgentConfig {
        max_turns: config.max_turns,
        budget: config.budget.clone(),
        model: model.to_string(),
        system_prompt: config.system_prompt.clone(),
        personality: config.personality.clone(),
        fundi_home: config.home_dir.clone(),
        provider: provider_from_model,
        stream: config.streaming.enabled,
        platform: platform.map(|s| s.to_string()),
        pass_session_id: true,
        runtime_providers: config
            .llm_providers
            .iter()
            .map(|(name, cfg)| {
                (
                    name.clone(),
                    RuntimeProviderConfig {
                        api_key: cfg.api_key.clone(),
                        base_url: cfg.base_url.clone(),
                        command: cfg.command.clone(),
                        args: cfg.args.clone(),
                        oauth_token_url: cfg.oauth_token_url.clone(),
                        oauth_client_id: cfg.oauth_client_id.clone(),
                    },
                )
            })
            .collect(),
        smart_model_routing: SmartModelRoutingConfig {
            enabled: config.smart_model_routing.enabled,
            max_simple_chars: config.smart_model_routing.max_simple_chars,
            max_simple_words: config.smart_model_routing.max_simple_words,
            cheap_model: config.smart_model_routing.cheap_model.as_ref().map(|m| {
                CheapModelRouteConfig {
                    provider: m.provider.clone(),
                    model: m.model.clone(),
                    base_url: m.base_url.clone(),
                    api_key_env: m.api_key_env.clone(),
                }
            }),
        },
        memory_nudge_interval: config.agent.memory_nudge_interval,
        skill_creation_nudge_interval: config.agent.skill_creation_nudge_interval,
        background_review_enabled: config.agent.background_review_enabled,
        ..AgentConfig::default()
    }
}

// ---------------------------------------------------------------------------
// bridge_tool_registry
// ---------------------------------------------------------------------------

/// Bridge a full [`fundi_tools::ToolRegistry`] into the minimal
/// [`AgentToolRegistry`](crate::agent_loop::ToolRegistry) consumed by
/// [`AgentLoop`](crate::AgentLoop).
pub fn bridge_tool_registry(tools: &ToolRegistry) -> AgentToolRegistry {
    let mut agent_registry = AgentToolRegistry::new();
    for schema in tools.get_definitions() {
        let name = schema.name.clone();
        let tools_clone = tools.clone();
        agent_registry.register(
            name.clone(),
            schema,
            Arc::new(
                move |params: Value| -> Result<String, fundi_core::ToolError> {
                    Ok(tools_clone.dispatch(&name, params))
                },
            ),
        );
    }
    agent_registry
}

// ---------------------------------------------------------------------------
// provider_api_key_from_env
// ---------------------------------------------------------------------------

/// Resolve an API key / token for a named LLM provider from well-known
/// environment variables.
pub fn provider_api_key_from_env(provider: &str) -> Option<String> {
    let var = match provider {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "qwen" => "DASHSCOPE_API_KEY",
        "kimi" | "moonshot" => "MOONSHOT_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "nous" => "NOUS_API_KEY",
        "copilot" => "GITHUB_COPILOT_TOKEN",
        "vllm" => "VLLM_API_KEY", // OPTIONAL — vLLM inaendesha bila key (lacal compute)
        _ => return None,
    };
    std::env::var(var).ok().filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// build_provider
// ---------------------------------------------------------------------------

/// Build an `Arc<dyn LlmProvider>` from a [`GatewayConfig`] and model string.
///
/// Falls back to [`StubProvider`] when no API key can be resolved.
pub fn build_provider(config: &GatewayConfig, model: &str) -> Arc<dyn LlmProvider> {
    let (provider_name, model_name) = parse_model_string(model);

    // AWS Bedrock / Converse + SigV4 is not implemented (Python v0.11.0 parity gap).
    // Do not silently route to [`GenericProvider`] against an OpenAI-compatible URL.
    if provider_name == "bedrock" {
        tracing::warn!(
            model = %model,
            "build_provider: AWS Bedrock is not implemented in fundi-agent-rust; see deploy/PARITY_MODULE_C.md (C1)"
        );
        return Arc::new(BedrockUnsupportedProvider {
            model: model.to_string(),
        });
    }

    let provider_config = config.llm_providers.get(provider_name);

    let api_key = provider_config
        .and_then(|c| c.api_key.clone())
        .or_else(|| provider_api_key_from_env(provider_name));

    let api_key = match api_key {
        Some(k) => k,
        // vLLM OpenAI-compatible server — HAITAJI API key (auth="EMPTY" standard).
        // Hakuna StubProvider hapa: local compute ni gharama sifuri.
        None if provider_name == "vllm" => "EMPTY".to_string(),
        None => {
            tracing::warn!(
                "No API key for provider '{provider_name}'; falling back to StubProvider"
            );
            return Arc::new(StubProvider {
                model: model.to_string(),
            });
        }
    };

    let base_url = provider_config.and_then(|c| c.base_url.clone());

    match provider_name {
        "openai" => {
            let mut p = OpenAiProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "anthropic" => {
            let mut p = AnthropicProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "openrouter" => {
            let mut p = OpenRouterProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            if let Some(cfg) = provider_config {
                if !cfg.provider_order.is_empty() {
                    p = p.with_provider_order(cfg.provider_order.clone());
                }
            }
            Arc::new(p)
        }
        "qwen" => {
            let mut p = QwenProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "kimi" | "moonshot" => {
            let mut p = KimiProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "minimax" => {
            let mut p = MiniMaxProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "nous" => {
            let mut p = NousProvider::new(&api_key).with_model(model_name);
            if let Some(url) = base_url {
                p = p.with_base_url(url);
            }
            Arc::new(p)
        }
        "copilot" => {
            let p = CopilotProvider::new(
                base_url.unwrap_or_else(|| "https://api.github.com/copilot".to_string()),
                &api_key,
            )
            .with_model(model_name);
            Arc::new(p)
        }
        "vllm" => {
            // vLLM (OpenAI-compatible) kwenye local GPU/CPU — 500+ tok/s inawezekana
            // kwa modeli ndogo (Qwen2.5-Coder-1.5B/7B). Default: single-node server.
            let url = base_url.unwrap_or_else(|| "http://127.0.0.1:8000/v1".to_string());
            Arc::new(GenericProvider::new(url, &api_key, model_name))
        }
        "deepseek" => {
            let url = base_url.unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
            Arc::new(GenericProvider::new(url, &api_key, model_name))
        }
        _ => {
            let url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            Arc::new(GenericProvider::new(url, &api_key, model_name))
        }
    }
}

// ---------------------------------------------------------------------------
// BedrockUnsupportedProvider — explicit parity gap (module C1)
// ---------------------------------------------------------------------------

/// Placeholder when the model string uses `bedrock:…` — Converse/SigV4 not wired.
pub struct BedrockUnsupportedProvider {
    pub model: String,
}

#[async_trait::async_trait]
impl LlmProvider for BedrockUnsupportedProvider {
    async fn chat_completion(
        &self,
        _messages: &[fundi_core::Message],
        _tools: &[fundi_core::ToolSchema],
        _max_tokens: Option<u32>,
        _temperature: Option<f64>,
        _model: Option<&str>,
        _extra_body: Option<&Value>,
    ) -> Result<fundi_core::LlmResponse, AgentError> {
        Err(AgentError::LlmApi(format!(
            "AWS Bedrock is not implemented in fundi-agent-rust (Python parity gap). \
             Model '{}'. See deploy/PARITY_MODULE_C.md (C1).",
            self.model
        )))
    }

    fn chat_completion_stream(
        &self,
        _messages: &[fundi_core::Message],
        _tools: &[fundi_core::ToolSchema],
        _max_tokens: Option<u32>,
        _temperature: Option<f64>,
        _model: Option<&str>,
        _extra_body: Option<&Value>,
    ) -> futures::stream::BoxStream<'static, Result<fundi_core::StreamChunk, AgentError>> {
        futures::stream::once(async move {
            Err(AgentError::LlmApi(
                "AWS Bedrock streaming is not implemented in fundi-agent-rust.".to_string(),
            ))
        })
        .boxed()
    }
}

// ---------------------------------------------------------------------------
// StubProvider — fallback when no API key is configured
// ---------------------------------------------------------------------------

/// Fallback LLM provider returned by [`build_provider`] when no API key
/// can be resolved for the requested provider.  Every call returns an error
/// directing the user to configure credentials.
pub struct StubProvider {
    pub model: String,
}

#[async_trait::async_trait]
impl LlmProvider for StubProvider {
    async fn chat_completion(
        &self,
        _messages: &[fundi_core::Message],
        _tools: &[fundi_core::ToolSchema],
        _max_tokens: Option<u32>,
        _temperature: Option<f64>,
        _model: Option<&str>,
        _extra_body: Option<&Value>,
    ) -> Result<fundi_core::LlmResponse, AgentError> {
        Err(AgentError::LlmApi(format!(
            "StubProvider: no LLM backend configured for model '{}'. \
             Configure an API key and provider in the config file.",
            self.model
        )))
    }

    fn chat_completion_stream(
        &self,
        _messages: &[fundi_core::Message],
        _tools: &[fundi_core::ToolSchema],
        _max_tokens: Option<u32>,
        _temperature: Option<f64>,
        _model: Option<&str>,
        _extra_body: Option<&Value>,
    ) -> futures::stream::BoxStream<'static, Result<fundi_core::StreamChunk, AgentError>> {
        futures::stream::once(async move {
            Err(AgentError::LlmApi(
                "StubProvider: no LLM backend configured for streaming.".to_string(),
            ))
        })
        .boxed()
    }
}

#[cfg(test)]
mod build_provider_tests {
    use super::*;

    #[test]
    fn bedrock_provider_is_explicit_gap() {
        let cfg = GatewayConfig::default();
        let p = build_provider(&cfg, "bedrock:us.anthropic.claude-3-5-sonnet-20241022-v2:0");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(async { p.chat_completion(&[], &[], None, None, None, None).await })
            .expect_err("bedrock should error");
        let s = err.to_string();
        assert!(
            s.contains("Bedrock") || s.contains("bedrock"),
            "unexpected: {s}"
        );
    }

    #[test]
    fn vllm_hakuna_key_shika_generic_healthy() {
        // vLLM bila VLLM_API_KEY — NI PASI: GenericProvider (network attempt) si Stub.
        // StubProvider hutoa Ok(kodi ya kujianda); GenericProvider dhidi ya server
        // isiyo na service inatoa Err (connection refused) — tofauti halisi.
        let cfg = GatewayConfig::default();
        let p = build_provider(&cfg, "vllm:Qwen2.5-Coder-1.5B-Instruct");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(async {
            p.chat_completion(&[], &[], None, None, None, None).await
        });
        let msg = match err {
            Err(e) => e.to_string(),
            Ok(reply) => {
                // Ok pia ni HALI YA HALI — StubProvider hutuma "stub" halisi.
                let s = reply.message.content.unwrap_or_default();
                assert!(
                    !s.to_lowercase().contains("stub"),
                    "vLLM bila key lazima isiwe StubProvider — {s}"
                );
                return;
            }
        };
        // Network kosa halisi — hakuna uongo wa "jibu la uongo".
        assert!(
            !msg.to_lowercase().contains("stub"),
            "vLLM bila key lazima isiwe StubProvider — {msg}"
        );
    }

    #[test]
    fn vllm_env_var_inasoma() {
        std::env::set_var("VLLM_API_KEY", "test-key-123");
        let k = provider_api_key_from_env("vllm");
        std::env::remove_var("VLLM_API_KEY");
        assert_eq!(k.as_deref(), Some("test-key-123"));
    }
}
