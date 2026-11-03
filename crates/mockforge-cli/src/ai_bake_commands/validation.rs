use super::{args::BakeArgs, Result};

pub(super) fn validate(args: &BakeArgs) -> Result<()> {
    if args.model.trim().is_empty()
        || !args.temperature.is_finite()
        || !(0.0..=2.0).contains(&args.temperature)
    {
        return Err("A model and finite temperature between 0 and 2 are required".into());
    }
    super::providers::validate(args)?;
    if !matches!(
        args.method.to_ascii_uppercase().as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    ) || matches!(args.status, 204 | 205 | 304)
    {
        return Err(
            "AI bake requires a method and status that can carry a JSON response body".into()
        );
    }
    super::route_path::validate(&args.path)?;
    super::storage::format(&args.output)?;
    Ok(())
}
