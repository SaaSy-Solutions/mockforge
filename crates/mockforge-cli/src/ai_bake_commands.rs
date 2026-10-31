//! Author static response configurations once; serve them without an AI provider.
use clap::Subcommand;
use mockforge_core::intelligent_behavior::{llm_client::LlmClient, types::LlmGenerationRequest};
use std::time::Duration;

mod args;
mod document;
mod providers;
mod route_path;
mod storage;
mod validation;
use args::BakeArgs;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(test)]
mod tests;

#[derive(Subcommand)]
pub(crate) enum AiCommands {
    /// Bake a JSON response into a static serve configuration; existing verified files are reused
    Bake(BakeArgs),
}

pub(crate) async fn handle(command: AiCommands) -> Result<()> {
    match command {
        AiCommands::Bake(args) => bake(args).await,
    }
}

async fn bake(args: BakeArgs) -> Result<()> {
    validation::validate(&args)?;
    let prompt = storage::read_prompt(&args.prompt_file)?;
    let identity = document::identity(&args, &prompt);
    if storage::load(&args.output, &identity)?.is_some() {
        println!("Reusing verified baked fixture: {}", args.output.display());
        return Ok(());
    }
    let model = providers::config(&args)?;
    let client = LlmClient::new(model);
    let mut request = LlmGenerationRequest::new(
        "Generate the JSON HTTP response body requested by the user. Return exactly one valid JSON value, without Markdown fences or commentary. Do not include credentials or configuration for contacting an AI provider.",
        prompt,
    ).with_temperature(args.temperature).with_max_tokens(args.max_tokens as usize);
    request.seed = args.seed;
    let body = tokio::time::timeout(Duration::from_secs(args.timeout), client.generate_json(&request))
        .await.map_err(|_| "AI bake request timed out; no fixture was written")?
        .map_err(|_| "AI bake failed: the provider must return a successful response containing valid JSON; no fixture was written")?;
    let baked = document::build(&args, body, &identity)?;
    storage::publish(&args.output, &baked, &identity)?;
    println!("Baked static fixture: {}", args.output.display());
    println!("Serve offline with: mockforge serve --config <fixture-path>");
    Ok(())
}
