use clap::{Args, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(super) enum Provider {
    Openai,
    OpenaiCompatible,
    Ollama,
    Anthropic,
}

impl Provider {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::OpenaiCompatible => "openai-compatible",
            Self::Ollama => "ollama",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Args)]
pub(crate) struct BakeArgs {
    /// UTF-8 prompt file; its text is never written to fixture metadata
    #[arg(long)]
    pub(super) prompt_file: PathBuf,
    /// JSON or YAML serve configuration to create; verified existing files are never replaced
    #[arg(long)]
    pub(super) output: PathBuf,
    #[arg(long, value_enum, default_value = "ollama")]
    pub(super) provider: Provider,
    /// Pin the provider model used at authoring time
    #[arg(long)]
    pub(super) model: String,
    /// Full provider API endpoint, required for openai-compatible servers
    #[arg(long)]
    pub(super) endpoint: Option<String>,
    /// Read credentials from this environment variable, never from a command-line value
    #[arg(long)]
    pub(super) api_key_env: Option<String>,
    /// Best-effort provider sampling seed; frozen fixtures provide the CI guarantee
    #[arg(long, env = "MOCKFORGE_AI_SEED", allow_negative_numbers = true)]
    pub(super) seed: Option<i64>,
    #[arg(long, default_value_t = 0.0)]
    pub(super) temperature: f64,
    #[arg(long,default_value_t=4096,value_parser=clap::value_parser!(u32).range(1..=1_000_000))]
    pub(super) max_tokens: u32,
    #[arg(long,default_value_t=60,value_parser=clap::value_parser!(u64).range(1..=86400))]
    pub(super) timeout: u64,
    #[arg(long, default_value = "GET")]
    pub(super) method: String,
    /// Literal route or whole-segment path parameters such as /users/{id}
    #[arg(long, default_value = "/fixture")]
    pub(super) path: String,
    #[arg(long,default_value_t=200,value_parser=clap::value_parser!(u16).range(200..600))]
    pub(super) status: u16,
}
