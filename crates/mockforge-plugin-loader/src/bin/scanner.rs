//! `mockforge-plugin-scanner` — out-of-process WASM artifact scanner.
//!
//! # Why a separate process
//!
//! The registry-server worker that drains `plugin_security_scans` rows used
//! to run all checks in-process. That bounded one class of failure (panics
//! inside `spawn_blocking`) but nothing stops a malicious WASM artifact from
//! triggering a wasmtime bug that reaches the server's address space,
//! consuming runaway memory, or tripping the global allocator. A dedicated
//! subprocess keeps the request-serving binary lean (no wasmtime
//! dependency) and gives us OS-level isolation: the kernel is the security
//! boundary, not Rust's type system.
//!
//! # Contract with the worker
//!
//! The binary is invoked like:
//!
//! ```text
//! mockforge-plugin-scanner \
//!     --wasm-path /path/to/plugin.wasm \
//!     --checksum deadbeef... \
//!     --declared-size 12345
//! ```
//!
//! It writes a single JSON document of type [`ScanReport`] to stdout, then
//! exits 0. A non-zero exit, malformed stdout, or a process killed by a
//! signal is treated by the worker as a scan failure (the row is marked
//! `"fail"` with an operator-visible finding).
//!
//! # Checks performed
//!
//! - **Integrity.** File size vs. declared size; SHA-256 of bytes vs.
//!   declared checksum.
//! - **Static.** Magic bytes + version, import/export inventory, high-risk
//!   WASI capabilities, suspicious byte patterns in the artifact body.
//! - **Dynamic.** The module is loaded into a wasmtime engine with an
//!   empty linker (no host imports resolved). If the module links without
//!   errors under that zero-capability environment it's reported as
//!   `dynamic_instantiable: true`. Modules that need imports — which every
//!   real plugin does — fail linking and are reported as
//!   `dynamic_instantiable: false` with the offending symbol. This is a
//!   smoke test for "does wasmtime think the module is well-formed enough
//!   to even try," not a correctness check.
//!
//! The scanner never calls a function in the guest module. It never gives
//! the guest access to a filesystem, network, clock, or environment
//! variables. A malicious module cannot do anything beyond triggering a
//! wasmtime parse/validate bug, and if that happens the subprocess crashes
//! without affecting the server.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser as ClapParser;
use serde::Serialize;
use sha2::{Digest, Sha256};
use wasmparser::{Parser, Payload};
use wasmtime::{Engine, Linker, Module, Store};

#[derive(ClapParser, Debug)]
#[command(
    name = "mockforge-plugin-scanner",
    about = "Scan a WebAssembly plugin artifact and print a JSON verdict.",
    version
)]
struct Args {
    /// Path to the WASM artifact on disk. The scanner reads it with
    /// ordinary file-system APIs; the registry worker is expected to drop
    /// the downloaded bytes into a tempfile before invoking.
    #[arg(long)]
    wasm_path: PathBuf,

    /// SHA-256 (hex) the publish request declared. Scanner recomputes and
    /// flags a mismatch as a critical finding.
    #[arg(long)]
    checksum: String,

    /// File size the publish request declared. Flagged as high-severity if
    /// it disagrees with what's on disk.
    #[arg(long)]
    declared_size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize)]
struct Finding {
    severity: Severity,
    category: String,
    title: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ScanReport {
    status: String,
    score: i16,
    findings: Vec<Finding>,
    dynamic_instantiable: bool,
    duration_ms: u128,
}

fn main() {
    let args = Args::parse();
    let started = Instant::now();

    let report = match scan(&args) {
        Ok(mut report) => {
            report.duration_ms = started.elapsed().as_millis();
            report
        }
        Err(e) => ScanReport {
            status: "fail".to_string(),
            score: 0,
            findings: vec![Finding {
                severity: Severity::High,
                category: "other".to_string(),
                title: "Scanner could not read artifact".to_string(),
                description: format!("{}", e),
            }],
            dynamic_instantiable: false,
            duration_ms: started.elapsed().as_millis(),
        },
    };

    match serde_json::to_string(&report) {
        Ok(s) => println!("{}", s),
        Err(e) => {
            // We can't format our own output — exit 2 so the worker knows
            // something's wrong with the scanner itself, not the artifact.
            eprintln!("failed to encode scan report: {}", e);
            std::process::exit(2);
        }
    }
}

fn scan(args: &Args) -> anyhow::Result<ScanReport> {
    let bytes = std::fs::read(&args.wasm_path)?;
    let mut findings: Vec<Finding> = Vec::new();
    let mut score: i16 = 100;

    // --- Integrity -----------------------------------------------------

    if bytes.len() as i64 != args.declared_size {
        findings.push(Finding {
            severity: Severity::High,
            category: "other".to_string(),
            title: "Artifact size mismatch".to_string(),
            description: format!(
                "On-disk artifact is {} bytes, publish declared {}.",
                bytes.len(),
                args.declared_size
            ),
        });
        score -= 40;
    }

    let computed = hex_encode(&Sha256::digest(&bytes));
    if !computed.eq_ignore_ascii_case(&args.checksum) {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "supply_chain".to_string(),
            title: "Checksum mismatch".to_string(),
            description: format!(
                "Recomputed SHA-256 ({}) does not match the declared checksum ({}).",
                computed, args.checksum
            ),
        });
        score = score.saturating_sub(60);
    }

    // --- Magic bytes ---------------------------------------------------

    if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
        findings.push(Finding {
            severity: Severity::Critical,
            category: "other".to_string(),
            title: "Not a valid WebAssembly module".to_string(),
            description: "Artifact does not begin with the WASM magic bytes (\\0asm).".to_string(),
        });
        return Ok(ScanReport {
            status: "fail".to_string(),
            score: 0,
            findings,
            dynamic_instantiable: false,
            duration_ms: 0,
        });
    }

    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 1 {
        findings.push(Finding {
            severity: Severity::Medium,
            category: "other".to_string(),
            title: "Unexpected WASM binary version".to_string(),
            description: format!(
                "Module declares WASM binary version {} (only 1 is stable).",
                version
            ),
        });
        score = score.saturating_sub(10);
    }

    // --- Static inventory (imports/exports) ----------------------------

    let mut import_count = 0u32;
    let mut export_count = 0u32;
    let mut unknown_namespaces = BTreeSet::<String>::new();
    let mut required_imports: Vec<(String, String)> = Vec::new();
    let mut parse_error: Option<String> = None;

    let parser = Parser::new(0);
    for payload in parser.parse_all(&bytes) {
        match payload {
            Ok(Payload::ImportSection(reader)) => {
                for import in reader {
                    match import {
                        Ok(imp) => {
                            import_count += 1;
                            required_imports.push((imp.module.to_string(), imp.name.to_string()));
                            if !ALLOWED_IMPORT_NAMESPACES.contains(&imp.module) {
                                unknown_namespaces.insert(imp.module.to_string());
                            }
                            if imp.module.starts_with("wasi") {
                                if let Some((_, severity, human)) =
                                    HIGH_RISK_WASI_IMPORTS.iter().find(|(n, _, _)| *n == imp.name)
                                {
                                    let (sev, penalty) = match *severity {
                                        "critical" => (Severity::Critical, 40),
                                        "high" => (Severity::High, 20),
                                        "medium" => (Severity::Medium, 8),
                                        _ => (Severity::Low, 3),
                                    };
                                    score = score.saturating_sub(penalty);
                                    findings.push(Finding {
                                        severity: sev,
                                        category: "insecure_coding".to_string(),
                                        title: format!(
                                            "High-risk WASI import: {}::{}",
                                            imp.module, imp.name
                                        ),
                                        description: format!(
                                            "Plugin imports a capability that {}. Review carefully.",
                                            human
                                        ),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            parse_error = Some(format!("malformed import: {}", e));
                            break;
                        }
                    }
                }
            }
            Ok(Payload::ExportSection(reader)) => {
                for export in reader {
                    if export.is_ok() {
                        export_count += 1;
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                parse_error = Some(e.to_string());
                break;
            }
        }
    }

    if let Some(err) = parse_error {
        findings.push(Finding {
            severity: Severity::High,
            category: "other".to_string(),
            title: "WASM module failed to parse".to_string(),
            description: format!("wasmparser rejected the module: {}", err),
        });
        score = score.saturating_sub(40);
    }

    if !unknown_namespaces.is_empty() {
        score = score.saturating_sub(15);
        for ns in &unknown_namespaces {
            findings.push(Finding {
                severity: Severity::Medium,
                category: "supply_chain".to_string(),
                title: "Unknown host import namespace".to_string(),
                description: format!(
                    "Plugin imports from '{}', not provided by any MockForge runtime.",
                    ns
                ),
            });
        }
    }

    findings.push(Finding {
        severity: Severity::Info,
        category: "other".to_string(),
        title: "Module inventory".to_string(),
        description: format!("{} import(s), {} export(s).", import_count, export_count),
    });

    // --- Byte-pattern scan --------------------------------------------

    let lowered = bytes.to_ascii_lowercase();
    for (pattern, severity, description) in SUSPICIOUS_BYTE_PATTERNS {
        let needle = pattern.to_ascii_lowercase();
        if contains_subslice(&lowered, &needle) {
            let (sev, penalty) = match *severity {
                "critical" => (Severity::Critical, 50),
                "high" => (Severity::High, 25),
                "medium" => (Severity::Medium, 10),
                _ => (Severity::Low, 5),
            };
            score = score.saturating_sub(penalty);
            findings.push(Finding {
                severity: sev,
                category: "malware".to_string(),
                title: format!("Suspicious byte pattern: {}", description),
                description: format!(
                    "Artifact contains the byte pattern '{}'.",
                    String::from_utf8_lossy(pattern)
                ),
            });
        }
    }

    // --- Dynamic: zero-capability instantiate --------------------------
    //
    // Compile + link-check the module against an empty linker. If the
    // module only references imports we deliberately don't provide the
    // linker will reject it — that's expected for every real plugin.
    // We record the outcome but don't punish it in the score, because
    // "needs imports" is just a fact of life. What we *do* punish is a
    // wasmtime compile error: the module is structurally broken.

    let dynamic_instantiable = match check_instantiable(&bytes) {
        Ok(()) => true,
        Err(DynError::Compile(msg)) => {
            findings.push(Finding {
                severity: Severity::High,
                category: "other".to_string(),
                title: "wasmtime failed to compile module".to_string(),
                description: format!(
                    "wasmtime could not compile this artifact: {}. It cannot be loaded at runtime.",
                    msg
                ),
            });
            score = score.saturating_sub(40);
            false
        }
        Err(DynError::Link(msg)) => {
            findings.push(Finding {
                severity: Severity::Info,
                category: "other".to_string(),
                title: "Module requires host imports".to_string(),
                description: format!(
                    "Module did not link against the zero-capability scanner linker: {}. This is expected — plugins rely on MockForge host functions.",
                    msg
                ),
            });
            false
        }
    };

    // --- Verdict -------------------------------------------------------

    let clamped = score.clamp(0, 100);
    let status = if clamped >= 70 {
        "pass"
    } else if clamped >= 40 {
        "warning"
    } else {
        "fail"
    }
    .to_string();

    Ok(ScanReport {
        status,
        score: clamped,
        findings,
        dynamic_instantiable,
        duration_ms: 0, // filled in by caller
    })
}

enum DynError {
    /// wasmtime rejected the module — malformed, unsupported feature, etc.
    Compile(String),
    /// The module compiled fine but needed host imports the scanner won't
    /// provide. This is expected for real plugins and is not a defect.
    Link(String),
}

fn check_instantiable(bytes: &[u8]) -> Result<(), DynError> {
    // Configure wasmtime for the smallest possible attack surface: no fuel
    // metering (because we never .call() anything), no epoch interruption,
    // no custom host state. The engine is cheap to create and dropped when
    // this fn returns.
    let mut config = wasmtime::Config::new();
    config.consume_fuel(false);
    config.wasm_reference_types(true);
    config.wasm_bulk_memory(true);
    let engine = Engine::new(&config).map_err(|e| DynError::Compile(e.to_string()))?;

    let module = Module::new(&engine, bytes).map_err(|e| DynError::Compile(e.to_string()))?;

    // An empty linker. Any import the guest declares will fail to resolve
    // here — that's what tells us whether the module needs host functions.
    let linker = Linker::<()>::new(&engine);
    let mut store = Store::new(&engine, ());
    linker
        .instantiate(&mut store, &module)
        .map(|_| ())
        .map_err(|e| DynError::Link(e.to_string()))
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// Kept in sync with the in-process lists in
// `mockforge-registry-server/src/workers/plugin_scanner.rs`. If you change one
// you almost certainly want to change both.
const ALLOWED_IMPORT_NAMESPACES: &[&str] = &[
    "wasi_snapshot_preview1",
    "wasi_unstable",
    "env",
    "mockforge",
    "mockforge_host",
];

const HIGH_RISK_WASI_IMPORTS: &[(&str, &str, &str)] = &[
    ("sock_open", "high", "opens outbound network sockets"),
    ("sock_connect", "high", "initiates outbound network connections"),
    ("sock_bind", "high", "binds to listening sockets"),
    ("sock_accept", "high", "accepts inbound connections"),
    ("path_open", "medium", "opens filesystem paths"),
    ("path_create_directory", "medium", "creates directories"),
    ("path_unlink_file", "medium", "deletes files"),
    ("path_remove_directory", "medium", "removes directories"),
    ("path_rename", "medium", "renames files"),
    ("proc_exec", "critical", "executes external processes"),
    ("proc_exit", "low", "exits the host process"),
];

const SUSPICIOUS_BYTE_PATTERNS: &[(&[u8], &str, &str)] = &[
    (b"/bin/sh -c", "critical", "shell command invocation"),
    (b"/bin/bash -c", "critical", "shell command invocation"),
    (b"curl http", "high", "hardcoded outbound curl URL"),
    (b"wget http", "high", "hardcoded outbound wget URL"),
    (b"nc -e", "critical", "reverse shell marker (netcat -e)"),
    (b"/etc/passwd", "high", "attempts to read system credentials file"),
    (b"/etc/shadow", "critical", "attempts to read system shadow file"),
    (b"aws_access_key_id=", "critical", "hardcoded AWS access key"),
    (b"AKIA", "medium", "possible AWS access key id"),
    (b"-----BEGIN PRIVATE KEY-----", "critical", "embedded private key"),
    (b"-----BEGIN RSA PRIVATE KEY-----", "critical", "embedded RSA private key"),
    (b"-----BEGIN OPENSSH PRIVATE KEY-----", "critical", "embedded SSH private key"),
    (b"xmr.pool", "critical", "cryptominer pool URL"),
    (b"stratum+tcp", "critical", "cryptominer stratum URL"),
];
