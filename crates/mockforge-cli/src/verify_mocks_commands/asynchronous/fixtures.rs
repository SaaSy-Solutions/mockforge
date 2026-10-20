//! Read every recorder-format fixture without applying a capture sample limit.
use super::*;
use crate::verify_mocks_commands::fixtures::files;

#[derive(Serialize)]
pub(super) struct Report {
    files_checked: usize,
    validation_scope: &'static str,
    pub assessment: AsyncSampleReport,
}

pub(super) fn verify(path: &Path, source: &Value) -> Result<Report> {
    let mut paths = Vec::new();
    files::discover(path, 0, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(Error::config("No asynchronous recorder JSON fixtures found"));
    }
    let mut records = Vec::new();
    let mut bytes = 0;
    for path in &paths {
        let data = files::read_bytes(path)?;
        bytes += data.len();
        if bytes > 64 * 1024 * 1024 {
            return Err(Error::config("AsyncAPI fixture set exceeds 64 MiB"));
        }
        let value: Value = serde_json::from_slice(&data).map_err(|e| {
            Error::config(format!("Invalid recorder fixture {}: {e}", path.display()))
        })?;
        let values = match value {
            Value::Array(values) => values,
            object => vec![object],
        };
        for value in values {
            records.push(serde_json::from_value(value).map_err(|e| {
                Error::config(format!("Invalid recorded exchange {}: {e}", path.display()))
            })?);
            if records.len() > 10000 {
                return Err(Error::config("AsyncAPI fixture set exceeds 10000 exchanges"));
            }
        }
    }
    let mut assessment = check(source, &records)?;
    for route in &mut assessment.routes {
        if route.exchanges_checked == 0 {
            route.status = "drift-detected";
            route
                .findings
                .push("Source channel/operation has no message fixture in this set".into());
        }
    }
    Ok(Report { files_checked:paths.len(),validation_scope:"AsyncAPI channel, operation, JSON/text payload and captured header schemas; recorder exchange JSON fixtures",assessment })
}
