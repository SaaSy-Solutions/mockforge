//! Provider configuration and support checks; credentials remain in memory.
use super::{
    args::{self, BakeArgs, Provider},
    Result,
};
use mockforge_core::intelligent_behavior::config::BehaviorModelConfig;

pub(super) fn validate(args: &BakeArgs) -> Result<()> {
    if args.provider == Provider::Anthropic && args.seed.is_some() {
        return Err("Anthropic does not support sampling seeds; omit --seed and use the frozen fixture for CI".into());
    }
    if args.provider == Provider::Ollama && args.api_key_env.is_some() {
        return Err("The Ollama adapter does not support API-key headers; use an OpenAI-compatible endpoint for authenticated generation".into());
    }
    if args.provider == Provider::OpenaiCompatible && args.endpoint.is_none() {
        return Err("--endpoint is required for an OpenAI-compatible provider".into());
    }
    if let Some(endpoint) = &args.endpoint {
        let url = url::Url::parse(endpoint).map_err(|_| "Invalid provider endpoint URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err("Use an HTTP(S) endpoint without URL userinfo or a fragment; supply credentials through an environment variable".into());
        }
    }
    Ok(())
}

pub(super) fn config(args: &BakeArgs) -> Result<BehaviorModelConfig> {
    let api_key = if let Some(name) = &args.api_key_env {
        Some(
            std::env::var(name)
                .map_err(|_| "The requested API-key environment variable is not set")?,
        )
    } else if args.provider == args::Provider::OpenaiCompatible {
        std::env::var("OPENAI_API_KEY").ok()
    } else {
        None
    };
    Ok(BehaviorModelConfig {
        llm_provider: args.provider.name().into(),
        model: args.model.clone(),
        api_key,
        api_endpoint: args.endpoint.clone(),
        temperature: args.temperature,
        max_tokens: args.max_tokens as usize,
        seed: args.seed,
        ..Default::default()
    })
}
