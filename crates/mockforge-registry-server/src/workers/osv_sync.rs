//! Background worker that refreshes the `osv_vulnerabilities` cache.
//!
//! The sync runs on a schedule (every 6 hours by default) and reads
//! advisory data from one of three pluggable sources, chosen by env var:
//!
//! | Env var                        | Meaning                                              |
//! |--------------------------------|------------------------------------------------------|
//! | `MOCKFORGE_OSV_SEED_PATH`      | Path to a local file or directory of OSV JSON blobs. |
//! | `MOCKFORGE_OSV_FEED_URL`       | Single HTTP(S) URL returning a JSON array of OSV records. |
//! | (neither)                      | Worker runs but does nothing — logs a hint and sleeps.    |
//!
//! The file path mode is the supported production channel: in air-gapped
//! or cost-constrained deploys, ops downloads the OSV bulk dumps out of
//! band (see <https://osv.dev/docs/#section/Data-Dumps>), extracts them,
//! and points the worker at the directory. The URL mode is a convenience
//! for dev setups that can reach the public OSV service directly — it
//! expects a pre-aggregated JSON array, not OSV.dev's per-advisory API
//! endpoints, so a small proxy or CDN step is still required.
//!
//! On purpose:
//!
//! * **Idempotent imports.** The store's upsert is keyed on
//!   `(advisory_id, ecosystem, package_name)`, so re-running the worker
//!   just refreshes rows. No deduplication logic here.
//! * **No per-scan HTTP.** The scanner reads from the cache synchronously.
//!   If the cache is empty the scanner falls back to the seed list; it
//!   never makes an outbound request during a scan.
//! * **Parse errors are non-fatal.** A single malformed advisory doesn't
//!   halt the import; it's logged and skipped so one bad file can't stall
//!   the whole refresh.

use std::time::Duration;

use mockforge_registry_core::models::osv::OsvImportRecord;
use tracing::{error, info, warn};

use crate::AppState;

const SYNC_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60); // 6h

pub fn start_osv_sync_worker(state: AppState) {
    // Resolve the source eagerly so a misconfigured deploy logs the choice
    // at boot rather than silently every 6 hours.
    let source = resolve_source();
    match &source {
        OsvSource::LocalPath(p) => {
            info!("OSV sync worker will read from local path: {}", p)
        }
        OsvSource::HttpUrl(u) => info!("OSV sync worker will fetch from: {}", u),
        OsvSource::Disabled => info!(
            "OSV sync worker started but disabled — set MOCKFORGE_OSV_SEED_PATH or \
             MOCKFORGE_OSV_FEED_URL to enable"
        ),
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SYNC_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(e) = run_once(&state, &source).await {
                error!("OSV sync tick failed: {}", e);
            }
        }
    });
}

#[derive(Debug, Clone)]
enum OsvSource {
    LocalPath(String),
    HttpUrl(String),
    Disabled,
}

fn resolve_source() -> OsvSource {
    if let Ok(path) = std::env::var("MOCKFORGE_OSV_SEED_PATH") {
        if !path.trim().is_empty() {
            return OsvSource::LocalPath(path);
        }
    }
    if let Ok(url) = std::env::var("MOCKFORGE_OSV_FEED_URL") {
        if !url.trim().is_empty() {
            return OsvSource::HttpUrl(url);
        }
    }
    OsvSource::Disabled
}

async fn run_once(state: &AppState, source: &OsvSource) -> anyhow::Result<()> {
    let records = match source {
        OsvSource::Disabled => return Ok(()),
        OsvSource::LocalPath(p) => load_from_local_path(p).await?,
        OsvSource::HttpUrl(u) => load_from_http(u).await?,
    };

    if records.is_empty() {
        info!("OSV sync: source returned no records");
        return Ok(());
    }

    let mut inserted = 0usize;
    let mut skipped = 0usize;
    for rec in records {
        match state.store.upsert_osv_advisory(&rec).await {
            Ok(n) => inserted += n,
            Err(e) => {
                skipped += 1;
                warn!("OSV sync: skipping {}: {}", rec.id, e);
            }
        }
    }
    info!(
        "OSV sync: imported {} (advisory, package) pair(s), skipped {}",
        inserted, skipped
    );
    Ok(())
}

/// Read advisory records from a local file or directory.
///
/// * If the path is a file ending in `.json`, it's parsed as either a
///   single OSV record or a JSON array of records.
/// * If it's a directory, every `*.json` file directly inside it is
///   parsed. Subdirectories are walked one level deep (OSV's bulk dumps
///   group advisories into per-ecosystem subdirs).
///
/// Non-JSON files and files that fail to parse are logged and skipped, not
/// treated as fatal — a corrupt advisory shouldn't kill the refresh.
async fn load_from_local_path(path: &str) -> anyhow::Result<Vec<OsvImportRecord>> {
    let path_owned = path.to_string();
    tokio::task::spawn_blocking(move || load_local_sync(&path_owned)).await?
}

fn load_local_sync(path: &str) -> anyhow::Result<Vec<OsvImportRecord>> {
    let meta = std::fs::metadata(path)?;
    if meta.is_file() {
        return parse_file(std::path::Path::new(path));
    }
    if !meta.is_dir() {
        anyhow::bail!("OSV seed path is neither file nor directory: {}", path);
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let etype = entry.file_type()?;
        if etype.is_file() {
            match parse_file(&entry.path()) {
                Ok(mut records) => out.append(&mut records),
                Err(e) => warn!("OSV sync: skipping {}: {}", entry.path().display(), e),
            }
        } else if etype.is_dir() {
            // One level deep: walk per-ecosystem subdirs like npm/, PyPI/.
            for inner in std::fs::read_dir(entry.path())? {
                let inner = inner?;
                if inner.file_type()?.is_file() {
                    match parse_file(&inner.path()) {
                        Ok(mut records) => out.append(&mut records),
                        Err(e) => warn!("OSV sync: skipping {}: {}", inner.path().display(), e),
                    }
                }
            }
        }
    }
    Ok(out)
}

fn parse_file(path: &std::path::Path) -> anyhow::Result<Vec<OsvImportRecord>> {
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)?;
    // OSV bulk dumps publish one advisory per file; local aggregated
    // files might be an array. Try array first, fall back to single.
    if let Ok(arr) = serde_json::from_slice::<Vec<OsvImportRecord>>(&bytes) {
        return Ok(arr);
    }
    let single: OsvImportRecord = serde_json::from_slice(&bytes)?;
    Ok(vec![single])
}

async fn load_from_http(url: &str) -> anyhow::Result<Vec<OsvImportRecord>> {
    // Short timeout — we run every 6 hours, we can fail fast and retry.
    // The download timeout stays large (10 minutes) because OSV's
    // ecosystem zips are hundreds of MB.
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(600))
        .build()?;
    let resp = client.get(url).send().await?.error_for_status()?;

    // OSV's official bulk-dump URLs (https://osv-vulnerabilities.storage.googleapis.com/<eco>/all.zip)
    // serve a zip archive with one JSON advisory per entry. Single-record
    // and array JSON payloads are still supported for custom feeds.
    //
    // Detection order (each step is a fallback for the previous failing):
    //   1. URL suffix — most deterministic signal when present.
    //   2. Content-Type header — covers the happy path for osv.dev.
    //   3. Zip magic bytes (`PK\x03\x04`) sniffed from the payload
    //      itself. Robust even if upstream ever serves the dump with a
    //      generic `application/octet-stream` or a stale `text/plain`
    //      content-type.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let url_hint = url.to_ascii_lowercase().ends_with(".zip");
    let ct_hint = content_type.contains("zip") || content_type.contains("octet-stream");

    let bytes = resp.bytes().await?;

    if url_hint || ct_hint || has_zip_magic(&bytes) {
        return parse_bulk_zip(bytes.to_vec()).await;
    }

    // JSON path (single or array).
    let body: serde_json::Value = serde_json::from_slice(&bytes)?;
    let records: Vec<OsvImportRecord> = match body {
        serde_json::Value::Array(_) => serde_json::from_value(body)?,
        serde_json::Value::Object(_) => {
            vec![serde_json::from_value(body)?]
        }
        _ => anyhow::bail!("OSV feed returned unexpected JSON root type"),
    };
    Ok(records)
}

/// Zip files — including the OSV.dev ecosystem dumps — start with the
/// local file header magic `0x50 0x4b 0x03 0x04` ("PK\x03\x04"). We sniff
/// this as a last-resort detection when URL and Content-Type don't
/// identify the payload, so a transport-layer hiccup (wrong
/// content-type, missing suffix) doesn't silently corrupt our import.
fn has_zip_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[..4] == b"PK\x03\x04"
}

/// Unzip an OSV bulk dump in memory and parse every `*.json` entry.
/// Malformed entries are logged and skipped — a single corrupt advisory
/// shouldn't halt the import of the rest.
///
/// Runs on a blocking pool because `zip` reads synchronously. OSV dumps
/// are CPU-bound to decompress and we don't want to tie up the async
/// runtime thread for the duration.
async fn parse_bulk_zip(bytes: Vec<u8>) -> anyhow::Result<Vec<OsvImportRecord>> {
    tokio::task::spawn_blocking(move || parse_bulk_zip_sync(&bytes)).await?
}

fn parse_bulk_zip_sync(bytes: &[u8]) -> anyhow::Result<Vec<OsvImportRecord>> {
    use std::io::{Cursor, Read};

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut out = Vec::with_capacity(archive.len());
    let mut skipped = 0usize;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                warn!("OSV sync: zip entry #{} unreadable: {}", i, e);
                skipped += 1;
                continue;
            }
        };
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_string();
        if !name.to_ascii_lowercase().ends_with(".json") {
            continue;
        }

        let mut buf = Vec::with_capacity(entry.size() as usize);
        if let Err(e) = entry.read_to_end(&mut buf) {
            warn!("OSV sync: could not read {}: {}", name, e);
            skipped += 1;
            continue;
        }

        // Individual advisories are one-record-per-file in OSV dumps,
        // but some mirrors pack arrays. Try both shapes.
        match serde_json::from_slice::<OsvImportRecord>(&buf) {
            Ok(rec) => out.push(rec),
            Err(_) => match serde_json::from_slice::<Vec<OsvImportRecord>>(&buf) {
                Ok(mut recs) => out.append(&mut recs),
                Err(e) => {
                    warn!("OSV sync: skipping {}: {}", name, e);
                    skipped += 1;
                }
            },
        }
    }

    if skipped > 0 {
        info!("OSV sync: parsed {} advisories from zip, skipped {}", out.len(), skipped);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Writes a fixture OSV JSON into a tempdir and confirms we parse it
    /// into the expected record shape. The parser has to tolerate both
    /// `summary`/`details`-only advisories (common in GHSA) and the array
    /// form produced by some OSV aggregators.
    #[test]
    fn parse_file_handles_single_and_array() {
        let tmp = tempfile::tempdir().unwrap();

        let single = r#"{
            "id": "GHSA-test-0000-0000",
            "summary": "dummy advisory",
            "affected": [
                {
                    "package": {"ecosystem": "npm", "name": "evil"},
                    "ranges": [],
                    "versions": ["1.0.0"]
                }
            ]
        }"#;
        let p1 = tmp.path().join("single.json");
        std::fs::File::create(&p1).unwrap().write_all(single.as_bytes()).unwrap();
        let recs = parse_file(&p1).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].id, "GHSA-test-0000-0000");
        assert_eq!(recs[0].affected[0].package.name, "evil");

        let array = r#"[
            {
                "id": "OSV-a",
                "details": "detail body\nsecond line",
                "affected": [
                    {"package": {"ecosystem": "PyPI", "name": "foo"},
                     "ranges": [{"events": [{"introduced": "0"}]}],
                     "versions": []}
                ]
            },
            {
                "id": "OSV-b",
                "summary": "another",
                "affected": []
            }
        ]"#;
        let p2 = tmp.path().join("array.json");
        std::fs::File::create(&p2).unwrap().write_all(array.as_bytes()).unwrap();
        let recs = parse_file(&p2).unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].id, "OSV-a");
        // `details` falls back to summary when `summary` is missing; we
        // take the first line only so multi-paragraph details stay tidy.
        assert_eq!(recs[0].human_summary(), "detail body");
    }

    #[test]
    fn parse_file_skips_non_json() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("README.txt");
        std::fs::File::create(&p).unwrap().write_all(b"not json").unwrap();
        let recs = parse_file(&p).unwrap();
        assert!(recs.is_empty());
    }

    #[test]
    fn zip_magic_sniffer_matches_pk_prefix() {
        assert!(has_zip_magic(b"PK\x03\x04extra"));
        assert!(!has_zip_magic(b""));
        assert!(!has_zip_magic(b"PK"));
        assert!(!has_zip_magic(b"{\"not\":\"a zip\"}"));
        // Gzip prefix is close to zip visually but not the same magic.
        assert!(!has_zip_magic(b"\x1f\x8b\x08\x00"));
    }

    #[test]
    fn bulk_zip_parser_handles_mixed_entries() {
        use std::io::Write;
        use zip::write::FileOptions;

        // Build an in-memory zip with three entries: one valid advisory,
        // one invalid JSON, one non-JSON README. The parser should return
        // the one valid record and skip the rest without error.
        let mut zip_bytes = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut writer = zip::ZipWriter::new(cursor);
            let opts: FileOptions<'_, ()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            writer.start_file("GHSA-good.json", opts).unwrap();
            writer
                .write_all(
                    br#"{"id":"GHSA-test","affected":[{"package":{"ecosystem":"npm","name":"foo"},"ranges":[],"versions":["1.0.0"]}]}"#,
                )
                .unwrap();

            writer.start_file("broken.json", opts).unwrap();
            writer.write_all(b"{ not valid json").unwrap();

            writer.start_file("README", opts).unwrap();
            writer.write_all(b"this is not an advisory").unwrap();

            writer.finish().unwrap();
        }

        let out = parse_bulk_zip_sync(&zip_bytes).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "GHSA-test");
    }

    #[test]
    fn severity_bucket_maps_cvss_score() {
        let rec: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "X",
            "summary": "",
            "affected": [],
            "severity": [{"type": "CVSS_V3", "score": "9.5"}]
        }))
        .unwrap();
        assert_eq!(rec.severity_bucket(), "critical");

        let med: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "Y",
            "summary": "",
            "affected": [],
            "severity": [{"type": "CVSS_V3", "score": "5.1"}]
        }))
        .unwrap();
        assert_eq!(med.severity_bucket(), "medium");

        let no_sev: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "Z",
            "summary": "",
            "affected": []
        }))
        .unwrap();
        assert_eq!(no_sev.severity_bucket(), "medium");
    }
}
