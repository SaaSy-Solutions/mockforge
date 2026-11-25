//! `mockforge mcp-contract` — pin what an MCP tool server declares, and fail
//! when it changes.
//!
//! `pin` asks a tool server for its instructions and its tools and writes them
//! to a contract file. That file is what was approved. `check` asks again and
//! reports every difference from the contract, so a rewritten description or a
//! new tool is reviewed instead of reaching an agent unnoticed.
//!
//! A contract pins what a server says. It cannot see what the server does
//! behind its own boundary, and it only compares like with like when the
//! client identity and the credentials are the ones it was pinned with.
//!
//! Exit codes: 0 pinned or unchanged, 1 changed since the contract, 2 when
//! there was nothing to compare. A server that cannot be asked is never
//! reported as unchanged. `--format json` prints one document and nothing else
//! on stdout.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Args, Subcommand};
use mockforge_contracts::mcp_contract::client::{fetch_declared, ClientOptions, Declared};
use mockforge_contracts::mcp_contract::{
    compare, Change, CheckContext, CheckReport, ClientIdentity, Contract, EnvHeader, Era, Finding,
    FindingKind, McpContractError, Outcome, ServerContract, Severity, SignalKind, SignalSite,
};
use serde::Serialize;
use serde_json::Value;

/// The version of the JSON document `mcp-contract pin --format json` prints.
const PIN_SCHEMA_VERSION: u32 = 1;

/// The `kind` of every pin document.
const PIN_KIND: &str = "mockforge-mcp-contract-pin";

/// Pinned, or nothing differs from the contract.
const EXIT_OK: i32 = 0;
/// The server was stopped before pinning, or differs from its contract.
const EXIT_CHANGED: i32 = 1;
/// Nothing could be pinned or compared.
const EXIT_NOT_CHECKED: i32 = 2;

/// `mockforge mcp-contract`.
#[derive(Args, Debug)]
pub(crate) struct McpContractArgs {
    #[command(subcommand)]
    pub command: McpContractCommand,
}

#[derive(Subcommand, Debug)]
pub(crate) enum McpContractCommand {
    /// Record what an MCP tool server declares today
    Pin(PinArgs),
    /// Report what an MCP tool server changed since its contract was pinned
    Check(CheckArgs),
}

#[derive(Args, Debug)]
pub(crate) struct PinArgs {
    #[command(flatten)]
    pub ask: AskArgs,

    /// The contract file to write. An existing file is never replaced without --force
    #[arg(long)]
    pub output: PathBuf,

    /// Replace an existing contract. This is how a change is approved
    #[arg(long)]
    pub force: bool,

    /// Pin although the server's text contains characters that do not print
    #[arg(long)]
    pub accept_hidden: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CheckArgs {
    #[command(flatten)]
    pub ask: AskArgs,

    /// The pinned contract
    #[arg(long)]
    pub contract: PathBuf,

    /// Fail when a finding is at this level or above
    #[arg(long, value_enum, default_value = "low")]
    pub fail_on: FailOn,

    /// Also write the JSON report to this file
    #[arg(long)]
    pub output: Option<PathBuf>,
}

/// How to ask the server. For `check`, anything left out comes from the
/// contract.
#[derive(Args, Debug)]
pub(crate) struct AskArgs {
    /// The MCP endpoint, http or https. For `check`, the contract's address unless given
    #[arg(long)]
    pub url: Option<String>,

    /// Send a plain header with every request, "Name: value". A header that carries a credential is refused
    #[arg(long = "header", value_name = "NAME:VALUE")]
    pub headers: Vec<String>,

    /// Send a credential header, "Name: ENV_VAR". Its value is read from the environment and is never written to the contract
    #[arg(long = "auth-env", value_name = "NAME:ENV_VAR")]
    pub auth_env: Vec<String>,

    /// Name yourself to the server as the agent's client does (default: mockforge)
    #[arg(long)]
    pub client_name: Option<String>,

    /// The version that goes with --client-name
    #[arg(long)]
    pub client_version: Option<String>,

    /// Seconds allowed for each request
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,

    /// Output format. `json` prints one document and nothing else on stdout
    #[arg(long, value_enum, default_value = "text")]
    pub format: Format,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum Format {
    Text,
    Json,
}

/// The lowest level at which a finding makes `check` fail.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum FailOn {
    /// Every finding, context included
    Info,
    Low,
    Medium,
    High,
}

impl FailOn {
    fn severity(self) -> Severity {
        match self {
            Self::Info => Severity::Info,
            Self::Low => Severity::Low,
            Self::Medium => Severity::Medium,
            Self::High => Severity::High,
        }
    }
}

/// How the server is asked, and what of that may be recorded.
#[derive(Debug)]
struct Asks {
    options: ClientOptions,
    /// Plain headers, which a contract records with their values.
    plain: BTreeMap<String, String>,
    /// The credential header, which a contract records without its value.
    auth: Option<EnvHeader>,
}

/// The document `pin` prints with `--format json`.
#[derive(Serialize)]
struct PinReport {
    schema_version: u32,
    kind: &'static str,
    mockforge_version: String,
    /// The contract file as it was named on the command line.
    output: String,
    /// `pinned` or `not_pinned`.
    outcome: &'static str,
    /// Why nothing was pinned, when that is the outcome.
    error: Option<String>,
    /// What was pinned; null until it is.
    contract: Option<PinSummary>,
    /// Things to read closely in the pinned text, each with where it is.
    signals: Vec<SignalSite>,
}

/// What a pin recorded.
#[derive(Serialize)]
struct PinSummary {
    digest: String,
    era: Era,
    protocol_version: String,
    implementation: Value,
    tools: usize,
}

/// Run `mockforge mcp-contract`.
pub(crate) async fn handle_mcp_contract_command(
    args: McpContractArgs,
) -> mockforge_core::Result<()> {
    let code = match args.command {
        McpContractCommand::Pin(args) => pin(args).await,
        McpContractCommand::Check(args) => check(args).await,
    };
    match code {
        EXIT_OK => Ok(()),
        code => std::process::exit(code),
    }
}

async fn pin(args: PinArgs) -> i32 {
    let format = args.ask.format;
    let output = args.output.display().to_string();
    let report = PinReport::new(&output);
    let asks = match resolve_asks(&args.ask, None) {
        Ok(asks) => asks,
        Err(error) => return not_pinned(report, error, EXIT_NOT_CHECKED, format),
    };
    if args.output.exists() && !args.force {
        return not_pinned(
            report,
            McpContractError::Contract {
                path: output,
                reason: "it already exists. Move it aside, or pass --force to approve a new contract in its place"
                    .to_string(),
            },
            EXIT_NOT_CHECKED,
            format,
        );
    }
    let declared = match fetch_declared(&asks.options).await {
        Ok(declared) => declared,
        Err(error) => return not_pinned(report, error, EXIT_NOT_CHECKED, format),
    };
    let server = server_contract(&asks, &declared);
    let contract =
        match Contract::pin(server, asks.options.client.clone(), declared.tools, version(), &now())
        {
            Ok(contract) => contract,
            Err(error) => return not_pinned(report, error, EXIT_NOT_CHECKED, format),
        };
    let mut report = report;
    report.signals = contract.signal_sites();
    let hidden = report
        .signals
        .iter()
        .any(|site| site.signals.iter().any(|signal| signal.kind == SignalKind::HiddenUnicode));
    if hidden && !args.accept_hidden {
        return not_pinned(
            report,
            McpContractError::Contract {
                path: args.output.display().to_string(),
                reason: "the server's text contains characters that do not print. Read the signals below, then pass \
                         --accept-hidden to pin it anyway"
                    .to_string(),
            },
            EXIT_CHANGED,
            format,
        );
    }
    if let Err(error) = write_json(&args.output, &contract) {
        return not_pinned(report, error, EXIT_NOT_CHECKED, format);
    }
    report.outcome = "pinned";
    report.contract = Some(PinSummary {
        digest: contract.digest.clone(),
        era: contract.server.era,
        protocol_version: contract.server.protocol_version.clone(),
        implementation: contract.server.implementation.clone(),
        tools: contract.tools.len(),
    });
    emit(&report, pin_text(&report), format);
    EXIT_OK
}

async fn check(args: CheckArgs) -> i32 {
    let format = args.ask.format;
    let context = CheckContext {
        contract: args.contract.display().to_string(),
        checked_at: now(),
        mockforge_version: version().to_string(),
    };
    let text = match std::fs::read_to_string(&args.contract) {
        Ok(text) => text,
        Err(error) => {
            return not_checked(
                context,
                "",
                McpContractError::Contract {
                    path: args.contract.display().to_string(),
                    reason: error.to_string(),
                },
                format,
            );
        }
    };
    let pinned = match Contract::from_json(&text, &context.contract) {
        Ok(pinned) => pinned,
        Err(error) => return not_checked(context, "", error, format),
    };
    let digest_pinned = pinned.digest.clone();
    let asks = match resolve_asks(&args.ask, Some(&pinned)) {
        Ok(asks) => asks,
        Err(error) => return not_checked(context, &digest_pinned, error, format),
    };
    let declared = match fetch_declared(&asks.options).await {
        Ok(declared) => declared,
        Err(error) => return not_checked(context, &digest_pinned, error, format),
    };
    let server = server_contract(&asks, &declared);
    let current = match Contract::pin(
        server,
        asks.options.client.clone(),
        declared.tools,
        &context.mockforge_version,
        &context.checked_at,
    ) {
        Ok(current) => current,
        Err(error) => return not_checked(context, &digest_pinned, error, format),
    };
    let findings = compare(&pinned, &current.server, &current.tools);
    let report = CheckReport::checked(
        context,
        &pinned,
        &current.digest,
        &current.server,
        current.tools.len(),
        findings,
    );
    let code = report.exit_code(args.fail_on.severity());
    if let Some(path) = &args.output {
        if let Err(error) = write_json(path, &report) {
            eprintln!("mockforge mcp-contract: could not write {}: {error}", path.display());
            return EXIT_NOT_CHECKED;
        }
    }
    emit(&report, check_text(&report), format);
    code
}

impl PinReport {
    fn new(output: &str) -> Self {
        Self {
            schema_version: PIN_SCHEMA_VERSION,
            kind: PIN_KIND,
            mockforge_version: version().to_string(),
            output: output.to_string(),
            outcome: "not_pinned",
            error: None,
            contract: None,
            signals: Vec::new(),
        }
    }
}

fn not_pinned(mut report: PinReport, error: McpContractError, code: i32, format: Format) -> i32 {
    report.outcome = "not_pinned";
    report.error = Some(error.to_string());
    emit(&report, pin_text(&report), format);
    code
}

fn not_checked(
    context: CheckContext,
    digest_pinned: &str,
    error: McpContractError,
    format: Format,
) -> i32 {
    let report = CheckReport::not_checked(context, digest_pinned, &error.to_string());
    emit(&report, check_text(&report), format);
    EXIT_NOT_CHECKED
}

fn emit(document: &impl Serialize, text: String, format: Format) {
    match format {
        Format::Text => println!("{text}"),
        Format::Json => match serde_json::to_string_pretty(document) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("mockforge mcp-contract: could not print the document: {error}")
            }
        },
    }
}

fn write_json(path: &Path, document: &impl Serialize) -> Result<(), McpContractError> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut text =
        serde_json::to_string_pretty(document).map_err(|error| McpContractError::Protocol {
            method: "pin".to_string(),
            detail: format!("the document cannot be written as JSON: {error}"),
        })?;
    text.push('\n');
    std::fs::write(path, text)?;
    Ok(())
}

/// Resolve how to ask: the command line first, then the contract.
fn resolve_asks(args: &AskArgs, pinned: Option<&Contract>) -> Result<Asks, McpContractError> {
    let endpoint = args
        .url
        .as_deref()
        .or(pinned.map(|pinned| pinned.server.url.as_str()))
        .ok_or_else(|| {
            McpContractError::Endpoint(
                "pin needs --url, the address of the MCP endpoint".to_string(),
            )
        })?;
    let recorded = pinned
        .map(|pinned| pinned.client.clone())
        .unwrap_or_else(ClientOptions::default_identity);
    let client = ClientIdentity {
        name: args.client_name.clone().unwrap_or(recorded.name),
        version: args.client_version.clone().unwrap_or(recorded.version),
    };
    let mut options = ClientOptions::new(endpoint, client)?;
    options.timeout = Duration::from_secs(args.timeout);
    let mut plain = BTreeMap::new();
    // The contract's plain headers go first, so a header given again replaces it.
    for (name, value) in pinned.map(|pinned| pinned.server.headers.clone()).unwrap_or_default() {
        add_plain(&mut options, &mut plain, &format!("{name}: {value}"))?;
    }
    for header in &args.headers {
        add_plain(&mut options, &mut plain, header)?;
    }
    let mut auth = pinned.and_then(|pinned| pinned.server.auth.clone());
    let lookup = |variable: &str| std::env::var(variable).ok();
    if let [one] = args.auth_env.as_slice() {
        options.env_header(one, lookup)?;
        let (name, env) = split_env(one);
        auth = Some(EnvHeader { name, env });
    } else if args.auth_env.len() > 1 {
        return Err(McpContractError::Header(
            "--auth-env may be given once; send headers that carry no credential with --header"
                .to_string(),
        ));
    } else if let Some(auth) = &auth {
        options.env_header(&format!("{}: {}", auth.name, auth.env), lookup)?;
    }
    Ok(Asks {
        options,
        plain,
        auth,
    })
}

/// A header given as `Name: value`, split at its first colon into the
/// lower-case name and the trimmed value. Either is empty when absent.
fn split_env(header: &str) -> (String, String) {
    let (name, value) = header.split_once(':').unwrap_or_default();
    (name.trim().to_ascii_lowercase(), value.trim().to_string())
}

fn add_plain(
    options: &mut ClientOptions,
    plain: &mut BTreeMap<String, String>,
    header: &str,
) -> Result<(), McpContractError> {
    let name = split_env(header).0;
    options.plain_header(header)?;
    let value = options
        .headers
        .iter()
        .find(|(existing, _)| *existing == name)
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    plain.insert(name, value);
    Ok(())
}

/// The server as it answered, with how it was asked.
fn server_contract(asks: &Asks, declared: &Declared) -> ServerContract {
    ServerContract {
        url: asks.options.endpoint.clone(),
        era: declared.era,
        protocol_version: declared.protocol_version.clone(),
        implementation: declared.implementation.clone(),
        capabilities: declared.capabilities.clone(),
        instructions: declared.instructions.clone(),
        headers: asks.plain.clone(),
        auth: asks.auth.clone(),
    }
}

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn check_text(report: &CheckReport) -> String {
    let outcome = match report.outcome {
        Outcome::Unchanged => "unchanged",
        Outcome::Drifted => "drifted",
        Outcome::NotChecked => "not checked",
    };
    let mut text = format!("{} · {outcome}", report.contract);
    if let Some(server) = &report.server {
        text.push_str(&format!(
            " · {} tools · protocol {}",
            report.summary.tools_after, server.protocol_version
        ));
        if let Some(name) = server.implementation.get("name").and_then(Value::as_str) {
            text.push_str(&format!(
                " · {name}{}",
                server
                    .implementation
                    .get("version")
                    .and_then(Value::as_str)
                    .map(|version| format!(" {version}"))
                    .unwrap_or_default()
            ));
        }
    }
    if report.outcome == Outcome::Drifted {
        let summary = report.summary;
        text.push_str(&format!(
            " (+{} added, {} removed, {} changed) · {} finding(s): {} high, {} medium, {} low, {} info",
            summary.added,
            summary.removed,
            summary.changed,
            report.findings.len(),
            summary.high,
            summary.medium,
            summary.low,
            summary.info
        ));
    }
    if let Some(error) = &report.error {
        text.push_str(&format!("\n  {error}"));
    }
    text.extend(report.findings.iter().map(finding_text));
    text
}

fn finding_text(finding: &Finding) -> String {
    let prefix = finding.tool.as_deref().map(|tool| format!("{tool}: ")).unwrap_or_default();
    let mut text = format!(
        "\n  {:<6} {:<23} {}{}",
        severity(finding.severity),
        kind(finding.kind),
        prefix,
        finding.summary
    );
    if !finding.signals.is_empty() {
        let shown: Vec<String> = finding
            .signals
            .iter()
            .map(|signal| format!("{} ({})", signal_kind(signal.kind), signal.detail))
            .collect();
        text.push_str(&format!("\n         signals: {}", shown.join(", ")));
    }
    for change in &finding.changes {
        if let (Some(Value::String(before)), Some(Value::String(after))) =
            (&change.before, &change.after)
        {
            text.push_str(&format!("\n         - {before}"));
            text.push_str(&format!("\n         + {after}"));
            continue;
        }
        let mark = match change.change {
            Change::Added => '+',
            Change::Removed => '-',
            Change::Changed => '~',
        };
        let value = change
            .after
            .as_ref()
            .or(change.before.as_ref())
            .map(|value| serde_json::to_string(value).unwrap_or_default())
            .unwrap_or_default();
        let value: String = value.chars().take(120).collect();
        text.push_str(&format!("\n         {mark} {} {value}", change.location));
    }
    text
}

fn pin_text(report: &PinReport) -> String {
    let mut text = match &report.contract {
        Some(summary) => format!(
            "{} · pinned · {} tools · protocol {} · {} · {}",
            report.output,
            summary.tools,
            summary.protocol_version,
            label(&summary.implementation),
            summary.digest
        ),
        None => format!("{} · not pinned", report.output),
    };
    if let Some(error) = &report.error {
        text.push_str(&format!("\n  {error}"));
    }
    if !report.signals.is_empty() {
        text.push_str(if report.contract.is_some() {
            "\n  read closely:"
        } else {
            "\n  signals:"
        });
        for site in &report.signals {
            let shown: Vec<String> = site
                .signals
                .iter()
                .map(|signal| format!("{} ({})", signal_kind(signal.kind), signal.detail))
                .collect();
            text.push_str(&format!("\n    {}  {}", site.location, shown.join(", ")));
        }
    }
    text
}

fn label(implementation: &Value) -> String {
    let name = implementation.get("name").and_then(Value::as_str).unwrap_or("unnamed");
    match implementation.get("version").and_then(Value::as_str) {
        Some(version) => format!("{name} {version}"),
        None => name.to_string(),
    }
}

fn severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
    }
}

fn kind(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::ToolAdded => "tool-added",
        FindingKind::ToolRemoved => "tool-removed",
        FindingKind::ToolChanged => "tool-changed",
        FindingKind::InstructionsAdded => "instructions-added",
        FindingKind::InstructionsChanged => "instructions-changed",
        FindingKind::InstructionsRemoved => "instructions-removed",
        FindingKind::CapabilitiesChanged => "capabilities-changed",
        FindingKind::ServerIdentityChanged => "server-identity-changed",
        FindingKind::ProtocolChanged => "protocol-changed",
        FindingKind::EndpointChanged => "endpoint-changed",
    }
}

fn signal_kind(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::HiddenUnicode => "hidden-unicode",
        SignalKind::InstructionLikeText => "instruction-like-text",
        SignalKind::ExternalRef => "external-ref",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_contracts::mcp_contract::{Delta, Signal};

    fn server() -> ServerContract {
        ServerContract {
            url: "https://mcp.example.com/mcp".to_string(),
            era: Era::Modern,
            protocol_version: "2026-07-28".to_string(),
            implementation: serde_json::json!({"name": "Example", "version": "1.2.0"}),
            capabilities: serde_json::json!({"tools": {}}),
            instructions: None,
            headers: BTreeMap::from([("x-tenant".to_string(), "acme".to_string())]),
            auth: None,
        }
    }

    fn contract(headers: BTreeMap<String, String>, auth: Option<EnvHeader>) -> Contract {
        let mut server = server();
        server.headers = headers;
        server.auth = auth;
        Contract::pin(
            server,
            ClientIdentity {
                name: "agent".to_string(),
                version: "2.0.0".to_string(),
            },
            vec![serde_json::json!({"name": "search", "inputSchema": {"type": "object"}})],
            "0.0.0",
            "2026-11-24T00:00:00Z",
        )
        .unwrap()
    }

    fn ask_args(url: Option<&str>, headers: &[&str], auth_env: &[&str]) -> AskArgs {
        AskArgs {
            url: url.map(str::to_string),
            headers: headers.iter().map(|header| (*header).to_string()).collect(),
            auth_env: auth_env.iter().map(|header| (*header).to_string()).collect(),
            client_name: None,
            client_version: None,
            timeout: 30,
            format: Format::Text,
        }
    }

    #[test]
    fn a_check_line_names_the_contract_the_outcome_and_the_server() {
        let context = CheckContext {
            contract: "acme.json".to_string(),
            checked_at: String::new(),
            mockforge_version: "0.0.0".to_string(),
        };
        let pinned = contract(BTreeMap::new(), None);
        let report =
            CheckReport::checked(context, &pinned, &pinned.digest, &pinned.server, 1, Vec::new());
        assert_eq!(
            check_text(&report),
            "acme.json · unchanged · 1 tools · protocol 2026-07-28 · Example 1.2.0"
        );

        let report = CheckReport::not_checked(
            CheckContext {
                contract: "acme.json".to_string(),
                checked_at: String::new(),
                mockforge_version: "0.0.0".to_string(),
            },
            "",
            "could not reach it",
        );
        assert_eq!(check_text(&report), "acme.json · not checked\n  could not reach it");
    }

    #[test]
    fn a_finding_shows_its_signals_and_both_texts() {
        let finding = Finding {
            kind: FindingKind::ToolChanged,
            severity: Severity::High,
            tool: Some("search".to_string()),
            summary: "changed: description".to_string(),
            changes: vec![Delta {
                location: "/description".to_string(),
                change: Change::Changed,
                before: Some(Value::String("Search the web".to_string())),
                after: Some(Value::String("Search the web\u{202E}".to_string())),
            }],
            signals: vec![Signal {
                kind: SignalKind::HiddenUnicode,
                detail: "U+202E RIGHT-TO-LEFT OVERRIDE x1".to_string(),
            }],
        };
        let head =
            format!("\n  {:<6} {:<23} {}", "high", "tool-changed", "search: changed: description");
        assert_eq!(
            finding_text(&finding),
            format!(
                "{head}\n         signals: hidden-unicode (U+202E RIGHT-TO-LEFT OVERRIDE x1)\
                 \n         - Search the web\n         + Search the web\u{202E}"
            )
        );

        let added = Finding {
            kind: FindingKind::ToolAdded,
            severity: Severity::High,
            tool: Some("drop".to_string()),
            summary: "new tool: Drop a table.".to_string(),
            changes: Vec::new(),
            signals: Vec::new(),
        };
        assert_eq!(
            finding_text(&added),
            format!("\n  {:<6} {:<23} {}", "high", "tool-added", "drop: new tool: Drop a table.")
        );
    }

    #[test]
    fn a_pin_line_says_what_was_pinned_and_what_to_read() {
        let mut report = PinReport::new("acme.json");
        assert_eq!(pin_text(&report), "acme.json · not pinned");
        report.error = Some("it already exists".to_string());
        assert_eq!(pin_text(&report), "acme.json · not pinned\n  it already exists");

        report.signals = vec![SignalSite {
            location: "/tools/1/description".to_string(),
            signals: vec![Signal {
                kind: SignalKind::InstructionLikeText,
                detail: "\"Ignore previous instructions\"".to_string(),
            }],
        }];
        assert_eq!(
            pin_text(&report),
            "acme.json · not pinned\n  it already exists\n  signals:\n    /tools/1/description  \
             instruction-like-text (\"Ignore previous instructions\")"
        );
    }

    #[test]
    fn the_command_line_comes_first_and_the_contract_fills_in_the_rest() {
        let error = resolve_asks(&ask_args(None, &[], &[]), None).unwrap_err();
        assert!(error.to_string().contains("pin needs --url"), "{error}");

        let pinned = contract(
            BTreeMap::from([("x-tenant".to_string(), "acme".to_string())]),
            Some(EnvHeader {
                name: "authorization".to_string(),
                env: "ABSENT_VARIABLE".to_string(),
            }),
        );
        // Nothing given: the contract's address, headers and credential are used.
        let error = resolve_asks(&ask_args(None, &[], &[]), Some(&pinned)).unwrap_err();
        assert!(error.to_string().contains("ABSENT_VARIABLE"), "{error}");

        // A header given again replaces the recorded one; a credential is read from its variable.
        let asks = resolve_asks(
            &ask_args(None, &["X-Tenant: other"], &[]),
            Some(&contract(BTreeMap::new(), None)),
        )
        .unwrap();
        assert_eq!(asks.options.endpoint, "https://mcp.example.com/mcp");
        assert_eq!(asks.options.client.name, "agent");
        assert_eq!(asks.options.headers, vec![("x-tenant".to_string(), "other".to_string())]);
        assert_eq!(asks.plain, BTreeMap::from([("x-tenant".to_string(), "other".to_string())]));
        assert!(asks.auth.is_none());
        assert_eq!(asks.options.timeout, Duration::from_secs(30));

        let error =
            resolve_asks(&ask_args(None, &[], &["Authorization: A", "X-Key: B"]), Some(&pinned))
                .unwrap_err();
        assert!(error.to_string().contains("--auth-env may be given once"), "{error}");
        let error = resolve_asks(&ask_args(None, &["Authorization: Bearer x"], &[]), Some(&pinned))
            .unwrap_err();
        assert!(error.to_string().contains("credential"), "{error}");
    }
}
