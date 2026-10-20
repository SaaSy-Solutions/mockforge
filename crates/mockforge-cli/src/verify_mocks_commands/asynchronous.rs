//! AsyncAPI definition, captured-message and complete fixture-set fidelity.
use super::{load_fidelity_document, DriftFinding, Value, VerifyMocksArgs};
use mockforge_contracts::fidelity::compare_mock_spec;
use mockforge_core::{Error, Result};
mod capture;
mod fixtures;
mod report;

pub(super) async fn handle(args: VerifyMocksArgs, source: Value) -> Result<()> {
    // Also validates a capture/fixture-only source before opening any database.
    compare_mock_spec(&source, &source).map_err(Error::config)?;
    let definition = args
        .mock_spec
        .as_ref()
        .map(|path| {
            compare_mock_spec(&load_fidelity_document(path)?, &source).map_err(Error::config)
        })
        .transpose()?;
    let sample = if let Some(path) = &args.capture_db {
        let records = capture::capture(path, args.limit).await?;
        Some(capture::check(&source, &records)?)
    } else {
        None
    };
    let fixtures = args
        .fixtures
        .as_deref()
        .map(|path| fixtures::verify(path, &source))
        .transpose()?;
    report::Report::new(args.spec.display().to_string(), definition, sample, fixtures)
        .emit(args.output.as_deref(), args.fail_on_drift)
}
