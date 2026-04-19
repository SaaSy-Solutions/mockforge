//! Background worker that refreshes the `osv_vulnerabilities` cache.
//!
//! The sync runs on a schedule (every 6 hours by default) and reads
//! advisory data from one of three pluggable sources, chosen by env var:
//!
//! | Env var                        | Meaning                                              |
//! |--------------------------------|------------------------------------------------------|
//! | `MOCKFORGE_OSV_SEED_PATH`      | Path to a local file or directory of OSV JSON blobs. |
//! | `MOCKFORGE_OSV_ECOSYSTEMS`     | Comma-separated OSV ecosystem names (`PyPI,npm,Go,crates.io,RubyGems`). Expands to the canonical bulk-dump URLs. |
//! | `MOCKFORGE_OSV_FEED_URL`       | HTTP(S) URL, or comma-separated list, of pre-built OSV feeds. Used when you already have a custom proxy/CDN or need an ecosystem the canonical list doesn't cover. |
//! | (none)                         | Worker runs but does nothing — logs a hint and sleeps.    |
//!
//! `MOCKFORGE_OSV_ECOSYSTEMS` is the recommended production knob: it's
//! ergonomic (you name the ecosystems, we construct the URLs), keeps the
//! config surface small, and lets the operator add npm/Go coverage by
//! editing one secret. `MOCKFORGE_OSV_FEED_URL` stays as an escape hatch
//! for custom feeds.
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
        OsvSource::HttpUrls(urls) => {
            info!("OSV sync worker will fetch from {} URL(s):", urls.len());
            for u in urls {
                info!("  - {}", u);
            }
        }
        OsvSource::Disabled => info!(
            "OSV sync worker started but disabled — set MOCKFORGE_OSV_SEED_PATH, \
             MOCKFORGE_OSV_ECOSYSTEMS, or MOCKFORGE_OSV_FEED_URL to enable"
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
    HttpUrls(Vec<String>),
    Disabled,
}

/// Canonical base URL for OSV.dev's per-ecosystem bulk dumps. Exposed as a
/// `const` rather than hard-coded into `resolve_source` so the conformance
/// test can format expected URLs the same way production does.
const OSV_BULK_BASE: &str = "https://osv-vulnerabilities.storage.googleapis.com";

fn resolve_source() -> OsvSource {
    if let Ok(path) = std::env::var("MOCKFORGE_OSV_SEED_PATH") {
        if !path.trim().is_empty() {
            return OsvSource::LocalPath(path);
        }
    }

    // Ecosystem list beats raw feed URL when both are set — it's the
    // friendlier knob and lets operators add coverage without thinking
    // about the URL template.
    if let Ok(raw) = std::env::var("MOCKFORGE_OSV_ECOSYSTEMS") {
        let urls = ecosystems_to_urls(&raw);
        if !urls.is_empty() {
            return OsvSource::HttpUrls(urls);
        }
    }

    if let Ok(raw) = std::env::var("MOCKFORGE_OSV_FEED_URL") {
        let urls: Vec<String> =
            raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !urls.is_empty() {
            return OsvSource::HttpUrls(urls);
        }
    }
    OsvSource::Disabled
}

/// Expand a comma-separated ecosystem list (e.g. `"PyPI, npm, Go"`) into
/// the canonical OSV.dev bulk-dump URLs. Whitespace is tolerated so the
/// operator can format the secret for readability; we preserve case
/// because OSV's bucket is case-sensitive (`PyPI/` works, `pypi/` 404s).
fn ecosystems_to_urls(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|eco| format!("{}/{}/all.zip", OSV_BULK_BASE, eco))
        .collect()
}

async fn run_once(state: &AppState, source: &OsvSource) -> anyhow::Result<()> {
    match source {
        OsvSource::Disabled => Ok(()),
        OsvSource::LocalPath(p) => {
            let records = load_from_local_path(p).await?;
            import_records(state, records, "local path").await;
            Ok(())
        }
        OsvSource::HttpUrls(urls) => {
            // Fetch each URL sequentially. Going parallel would cut wall
            // time but each dump can be 100+ MB and we run on a small
            // machine; sequential + streaming (see import_from_http_streaming)
            // keeps peak memory bounded to ~a dozen MB regardless of
            // ecosystem size. One URL failing doesn't stop the others.
            for url in urls {
                if let Err(e) = import_from_http_streaming(state, url).await {
                    error!("OSV sync [{}]: {}", url, e);
                }
            }
            Ok(())
        }
    }
}

/// How many parsed records to batch before flushing to the store.
/// Tuned to keep peak memory in the low tens of MB. Each record is small
/// (most are <2 KB serialized), so 500 ≈ ~1 MB plus the zip entry buffer.
const UPSERT_BATCH: usize = 500;

/// Depth of the mpsc channel between the blocking zip reader and the
/// async upsert loop. Four in-flight batches is enough to overlap
/// parsing with DB round-trips without letting the reader run far ahead
/// of the writer.
const BATCH_CHANNEL_DEPTH: usize = 4;

/// Stream-import one OSV HTTP source into the store.
///
/// The old `Vec<OsvImportRecord>`-returning `load_from_http` spiked peak
/// memory to several hundred MB for ecosystems the size of PyPI or npm:
/// the response body was buffered into `bytes::Bytes`, then the entire
/// decompressed + parsed record set was materialized at once, and only
/// after that did the upsert loop get a chance to free memory. On a 512
/// MB machine that landed us in OOM-killer territory every sync cycle.
///
/// This function does it incrementally:
///
/// 1. Spool the HTTP response body to a `tempfile::NamedTempFile` via
///    `bytes_stream()`, so the zip bytes never all live in RAM.
/// 2. Open the tempfile with `zip::ZipArchive::new(File)`. The zip
///    crate uses Read+Seek and only decompresses the current entry, so
///    the archive's peak resident footprint is roughly one entry's
///    decompressed JSON (tens of KB for an OSV advisory).
/// 3. A blocking task iterates entries, parses records, and sends
///    batches of `UPSERT_BATCH` down an mpsc channel.
/// 4. The async side receives batches and upserts them one record at a
///    time. Channel backpressure naturally paces the reader: four
///    in-flight batches ≈ 2 MB of records, enough to keep parsing and
///    DB I/O overlapping without running the reader off ahead.
async fn import_from_http_streaming(state: &AppState, url: &str) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    use std::io::Write;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(600))
        .build()?;
    let resp = client.get(url).send().await?.error_for_status()?;

    // Detection order for zip vs JSON is the same as before but we have
    // to commit to it before seeing the body (we're streaming to disk,
    // not buffering), so URL + Content-Type have to carry it. The zip
    // magic sniff happens after we've written the first chunk.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let url_hint = url.to_ascii_lowercase().ends_with(".zip");
    let ct_hint = content_type.contains("zip") || content_type.contains("octet-stream");

    // Stream the body to disk.
    let tmp = tokio::task::spawn_blocking(tempfile::NamedTempFile::new).await??;
    let tmp_path = tmp.path().to_path_buf();
    let mut file = tokio::fs::File::from_std(tmp.reopen()?);
    let mut stream = resp.bytes_stream();
    let mut first_four: [u8; 4] = [0; 4];
    let mut first_seen = 0usize;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if first_seen < 4 {
            let take = (4 - first_seen).min(chunk.len());
            first_four[first_seen..first_seen + take].copy_from_slice(&chunk[..take]);
            first_seen += take;
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file); // Close the handle so the blocking reader can open it exclusively.

    let is_zip = url_hint || ct_hint || (first_seen == 4 && has_zip_magic(&first_four));

    if is_zip {
        stream_zip_file_to_store(state, &tmp_path, url).await?;
    } else {
        // JSON path: small feeds only (single record or array). Re-read
        // the tempfile — it's bounded by the feed size and in practice
        // tiny compared to the zip dumps.
        let bytes = tokio::task::spawn_blocking({
            let p = tmp_path.clone();
            move || std::fs::read(p)
        })
        .await??;
        let body: serde_json::Value = serde_json::from_slice(&bytes)?;
        let records: Vec<OsvImportRecord> = match body {
            serde_json::Value::Array(_) => serde_json::from_value(body)?,
            serde_json::Value::Object(_) => vec![serde_json::from_value(body)?],
            _ => anyhow::bail!("OSV feed returned unexpected JSON root type"),
        };
        import_records(state, records, url).await;
    }

    // Tempfile drops here, removing it from disk. Writing above went
    // through `reopen()` so the `NamedTempFile`'s own handle is still
    // the owner of the underlying delete-on-drop.
    drop(tmp);
    // Explicit shadowed-variable hint so `_` isn't necessary; suppress
    // the unused-let warning that can fire when this is the last use.
    std::io::sink().write_all(&[]).ok();
    Ok(())
}

/// Run the zip reader on a blocking thread, send parsed records over
/// an mpsc channel in batches, upsert batches on the async side.
async fn stream_zip_file_to_store(
    state: &AppState,
    zip_path: &std::path::Path,
    source_label: &str,
) -> anyhow::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<OsvImportRecord>>(BATCH_CHANNEL_DEPTH);

    let path_for_task = zip_path.to_path_buf();
    let reader_task = tokio::task::spawn_blocking(move || -> anyhow::Result<usize> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path_for_task)?)?;
        let mut batch: Vec<OsvImportRecord> = Vec::with_capacity(UPSERT_BATCH);
        let mut entry_skipped = 0usize;

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(_) => {
                    entry_skipped += 1;
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
            if std::io::Read::read_to_end(&mut entry, &mut buf).is_err() {
                entry_skipped += 1;
                continue;
            }
            // One-record-per-file is the OSV dump convention, but some
            // mirrors pack arrays. Try both shapes.
            match serde_json::from_slice::<OsvImportRecord>(&buf) {
                Ok(rec) => batch.push(rec),
                Err(_) => match serde_json::from_slice::<Vec<OsvImportRecord>>(&buf) {
                    Ok(recs) => batch.extend(recs),
                    Err(_) => {
                        entry_skipped += 1;
                        continue;
                    }
                },
            }

            if batch.len() >= UPSERT_BATCH {
                let out = std::mem::replace(&mut batch, Vec::with_capacity(UPSERT_BATCH));
                // Consumer gone means the whole import was cancelled —
                // stop gracefully instead of blocking forever.
                if tx.blocking_send(out).is_err() {
                    return Ok(entry_skipped);
                }
            }
        }
        if !batch.is_empty() {
            let _ = tx.blocking_send(batch);
        }
        Ok(entry_skipped)
    });

    let mut inserted = 0usize;
    let mut upsert_skipped = 0usize;
    while let Some(batch) = rx.recv().await {
        for rec in batch {
            match state.store.upsert_osv_advisory(&rec).await {
                Ok(n) => inserted += n,
                Err(e) => {
                    upsert_skipped += 1;
                    warn!("OSV sync [{}]: skipping {}: {}", source_label, rec.id, e);
                }
            }
        }
    }

    let entry_skipped = reader_task.await??;

    info!(
        "OSV sync [{}]: imported {} (advisory, package) pair(s); skipped {} entries, {} upserts",
        source_label, inserted, entry_skipped, upsert_skipped
    );
    Ok(())
}

/// Shared body for the local-path and HTTP code paths. Logs per-source so
/// multi-ecosystem runs show which feed contributed what, otherwise a
/// thousand-record import is indistinguishable from a stuck loop.
async fn import_records(state: &AppState, records: Vec<OsvImportRecord>, source_label: &str) {
    if records.is_empty() {
        info!("OSV sync [{}]: no records", source_label);
        return;
    }

    let mut inserted = 0usize;
    let mut skipped = 0usize;
    for rec in records {
        match state.store.upsert_osv_advisory(&rec).await {
            Ok(n) => inserted += n,
            Err(e) => {
                skipped += 1;
                warn!("OSV sync [{}]: skipping {}: {}", source_label, rec.id, e);
            }
        }
    }
    info!(
        "OSV sync [{}]: imported {} (advisory, package) pair(s), skipped {}",
        source_label, inserted, skipped
    );
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

/// Zip files — including the OSV.dev ecosystem dumps — start with the
/// local file header magic `0x50 0x4b 0x03 0x04` ("PK\x03\x04"). We sniff
/// this on the first downloaded chunk as a last-resort detection when
/// URL and Content-Type don't identify the payload, so a transport-layer
/// hiccup (wrong content-type, missing suffix) doesn't silently corrupt
/// our import.
fn has_zip_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[..4] == b"PK\x03\x04"
}

/// In-memory zip parser retained as a unit-test helper so the record-
/// parsing logic is covered without standing up a tempfile +
/// tokio runtime. Production uses `stream_zip_file_to_store` which
/// reads the same format off disk one entry at a time.
#[cfg(test)]
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
    fn ecosystems_to_urls_expands_comma_list() {
        assert_eq!(
            ecosystems_to_urls("PyPI,npm,Go"),
            vec![
                format!("{}/PyPI/all.zip", OSV_BULK_BASE),
                format!("{}/npm/all.zip", OSV_BULK_BASE),
                format!("{}/Go/all.zip", OSV_BULK_BASE),
            ]
        );
        // Whitespace around names is tolerated.
        assert_eq!(
            ecosystems_to_urls("  PyPI  , npm "),
            vec![
                format!("{}/PyPI/all.zip", OSV_BULK_BASE),
                format!("{}/npm/all.zip", OSV_BULK_BASE),
            ]
        );
        // Case is preserved — `pypi` would 404 on the real bucket.
        let out = ecosystems_to_urls("PyPI");
        assert!(out[0].contains("/PyPI/"), "got {}", out[0]);
        // Empty / whitespace-only input produces no URLs so the caller
        // knows to fall through to the next source.
        assert!(ecosystems_to_urls("").is_empty());
        assert!(ecosystems_to_urls(" , , ").is_empty());
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
