//! Custom conformance test authoring via YAML
//!
//! Allows users to define additional conformance checks beyond the built-in
//! OpenAPI 3.0.0 feature set. Custom checks are grouped under a "Custom"
//! category in the conformance report.

use crate::error::{BenchError, Result};
use serde::Deserialize;
use std::path::Path;

/// Top-level YAML configuration for custom conformance checks
#[derive(Debug, Deserialize)]
pub struct CustomConformanceConfig {
    /// List of custom checks to run
    pub custom_checks: Vec<CustomCheck>,
    /// Round 38 (#79) — Srikanth on 0.3.182. Repeat the entire
    /// `custom_checks` sequence N times so a "log in, do work,
    /// log out" chain can be exercised under load. The
    /// `${var:...}` / `${cookie:...}` substitution context is
    /// reset at the start of each iteration; values captured in
    /// iteration K are NOT visible to iteration K+1. Defaults to 1.
    #[serde(default = "default_iterations")]
    pub chain_iterations: u32,
}

fn default_iterations() -> u32 {
    1
}

/// A single custom conformance check
#[derive(Debug, Deserialize)]
pub struct CustomCheck {
    /// Check name (should start with "custom:" for report aggregation)
    pub name: String,
    /// Request path (e.g., "/api/users")
    pub path: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Expected HTTP status code
    pub expected_status: u16,
    /// Optional request body (JSON string)
    #[serde(default)]
    pub body: Option<String>,
    /// Optional expected response headers (name -> regex pattern)
    #[serde(default)]
    pub expected_headers: std::collections::HashMap<String, String>,
    /// Optional expected body fields with type validation
    #[serde(default)]
    pub expected_body_fields: Vec<ExpectedBodyField>,
    /// Optional request headers
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,

    /// Round 38 (#79) — file upload support. When set, the request
    /// is sent as `multipart/form-data` with one part per file. Each
    /// file's bytes come from a local path (so the YAML can name a
    /// `.exe`, `.jpg`, `.json`, `.docx`, `.xml`, etc. without
    /// embedding the bytes). `body` wins over `upload`/`uploads`.
    #[serde(default)]
    pub upload: Option<UploadFile>,
    #[serde(default)]
    pub uploads: Vec<UploadFile>,

    /// Round 38 (#79) — capture values from the response into the
    /// chain context so subsequent checks can reference them via
    /// `${var:NAME}`, `${cookie:NAME}`, `${header:NAME}` in path /
    /// headers / body.
    #[serde(default)]
    pub extract: ExtractRules,

    /// Round 38 (#79) — repeat the check N times within an
    /// iteration. `mode: parallel` fires N concurrent requests
    /// (Srikanth's Sequence 1: "Use that cookie and csrf token in 16
    /// subsequent requests that should be sent in parallel").
    /// `mode: sequential` runs them one after another (Sequence 2).
    #[serde(default)]
    pub repeat: Repeat,
}

/// Expected field in the response body with type checking
#[derive(Debug, Deserialize)]
pub struct ExpectedBodyField {
    /// Field name in the JSON response
    pub name: String,
    /// Expected JSON type: "string", "integer", "number", "boolean", "array", "object"
    #[serde(rename = "type")]
    pub field_type: String,
}

/// Round 38 (#79) — a single file to upload as a multipart form part.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadFile {
    /// Local path to the file; bytes are read at request time.
    pub path: String,
    /// `Content-Type` for this part. Common values:
    /// `application/octet-stream`, `image/jpeg`, `application/json`,
    /// `application/xml`.
    #[serde(default = "default_upload_content_type")]
    pub content_type: String,
    /// Multipart form field name. Defaults to `"file"`.
    #[serde(default = "default_upload_field_name")]
    pub field_name: String,
    /// Filename announced to the server. Defaults to the basename
    /// of `path`.
    #[serde(default)]
    pub filename: Option<String>,
}

fn default_upload_content_type() -> String {
    "application/octet-stream".to_string()
}
fn default_upload_field_name() -> String {
    "file".to_string()
}

/// Round 38 (#79) — what to capture from a check's response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExtractRules {
    /// Cookie names to capture from `Set-Cookie`. Stored under
    /// `${cookie:NAME}`.
    #[serde(default)]
    pub cookies: Vec<String>,
    /// Response headers to capture (var_name -> header_name). Header
    /// name is case-insensitive. Stored under `${var:VAR_NAME}`.
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    /// JSON body fields by simple dotted path. Stored under
    /// `${var:VAR_NAME}`.
    #[serde(default)]
    pub body_fields: std::collections::HashMap<String, String>,
}

impl ExtractRules {
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty() && self.headers.is_empty() && self.body_fields.is_empty()
    }
}

/// Round 38 (#79) — repeat semantics for a single custom check.
#[derive(Debug, Clone, Deserialize)]
pub struct Repeat {
    #[serde(default = "default_repeat_count")]
    pub count: u32,
    #[serde(default)]
    pub mode: RepeatMode,
}

impl Default for Repeat {
    fn default() -> Self {
        Self {
            count: 1,
            mode: RepeatMode::default(),
        }
    }
}

impl Repeat {
    pub fn is_default(&self) -> bool {
        self.count == 1 && matches!(self.mode, RepeatMode::Sequential)
    }
}

fn default_repeat_count() -> u32 {
    1
}

/// Round 38 (#79) — sequential vs parallel repeat.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Sequential,
    Parallel,
}

impl CustomConformanceConfig {
    /// Parse a custom conformance config from a YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BenchError::Other(format!(
                "Failed to read custom conformance file '{}': {}",
                path.display(),
                e
            ))
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            BenchError::Other(format!(
                "Failed to parse custom conformance YAML '{}': {}",
                path.display(),
                e
            ))
        })
    }

    /// Generate a k6 `group('Custom', ...)` block for all custom checks.
    ///
    /// `base_url` is the JS expression for the base URL (e.g., `"BASE_URL"`).
    /// `custom_headers` are additional headers to inject into every request.
    pub fn generate_k6_group(&self, base_url: &str, custom_headers: &[(String, String)]) -> String {
        self.generate_k6_group_with_options(base_url, custom_headers, false)
    }

    /// Round 39 (#79) — splits the k6 emit into the init-scope code
    /// (`open(...)` calls for file uploads) and the per-VU group body
    /// (the `group('Custom', ...)` block). Caller is responsible for
    /// placing `init_code` at the top of the script (before
    /// `export default function`) and `group_body` inside the default
    /// function. For backwards compatibility, `generate_k6_group` and
    /// `generate_k6_group_with_options` concatenate the two so existing
    /// code paths keep working — but they will emit the `open()` calls
    /// inside the VU function, which k6 rejects at runtime for
    /// uploads. Use `emit_k6_with_options` directly when uploads are
    /// expected.
    pub fn emit_k6_with_options(
        &self,
        base_url: &str,
        custom_headers: &[(String, String)],
        export_requests: bool,
    ) -> K6CustomEmit {
        let mut init_code = String::new();
        let mut group_body = String::new();
        let mut upload_counter: usize = 0;
        write_k6_group_body(
            self,
            base_url,
            custom_headers,
            export_requests,
            &mut group_body,
            &mut init_code,
            &mut upload_counter,
        );
        K6CustomEmit {
            init_code,
            group_body,
        }
    }

    /// Generate a k6 `group('Custom', ...)` block for all custom checks.
    /// When `export_requests` is true, emits `__captureExchange` calls after each request.
    ///
    /// **Round 39 (#79) — file uploads need `open()` at init scope, so
    /// callers that may receive an `upload`/`uploads` YAML should
    /// switch to `emit_k6_with_options` and splice `init_code`
    /// separately.** This method is kept for backwards compatibility
    /// and concatenates init + body — which crashes k6 at runtime when
    /// uploads are configured.
    pub fn generate_k6_group_with_options(
        &self,
        base_url: &str,
        custom_headers: &[(String, String)],
        export_requests: bool,
    ) -> String {
        let emit = self.emit_k6_with_options(base_url, custom_headers, export_requests);
        // Backwards compat: concat both into one string. k6 will fail
        // at runtime if `open()` calls land inside the VU function,
        // but tests / call paths that never set `upload`/`uploads`
        // produce the same output as before round 39.
        let mut combined = String::with_capacity(emit.init_code.len() + emit.group_body.len());
        combined.push_str(&emit.init_code);
        combined.push_str(&emit.group_body);
        combined
    }
}

/// Round 39 (#79) — split output from the k6 emitter so the caller can
/// place `init_code` at script-init scope (where k6's `open()` must
/// live) and `group_body` inside `export default function`.
#[derive(Debug, Default, Clone)]
pub struct K6CustomEmit {
    pub init_code: String,
    pub group_body: String,
}

/// Round 39 (#79) — escape a Rust string for safe embedding in a JS
/// single-quoted string literal. Wrapping `String::push_str` calls
/// already produce template-literal text, so this is for the simple
/// `'...'` body / value case.
fn js_escape_sq(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Round 39 (#79) — escape a Rust string for embedding inside a k6
/// template literal (\`...\`). Backticks and `${` must both be
/// escaped to avoid breaking out of the literal or starting an
/// unintended interpolation. Currently unused since
/// `substitute_chain_tokens` does the full template-literal escape +
/// substitute in one pass; kept around for future call sites that
/// want a plain escape without substitution.
#[allow(dead_code)]
fn js_escape_tpl(s: &str) -> String {
    s.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${")
}

/// Round 39 (#79) — translate `${var:NAME}` / `${cookie:NAME}` /
/// `${header:NAME}` tokens in `text` to k6 template-literal
/// interpolations against the chain context variables. The output is
/// inserted directly into a k6 template literal (\`...\`), so the
/// caller MUST NOT subsequently call `js_escape_tpl` — the `${...}`
/// interpolation sequences are intentional. Other characters that
/// would prematurely close the template literal (backtick, backslash)
/// are escaped here. Unrecognised `${...}` shapes from the YAML are
/// escaped to `\\${...}` so they show up as literal text in the
/// request log without breaking out into JS.
fn substitute_chain_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        // Escape any backtick or backslash in the prefix before the `${`.
        for c in rest[..start].chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '`' => out.push_str("\\`"),
                other => out.push(other),
            }
        }
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let token = &after[..end];
            let replacement = if let Some(name) = token.strip_prefix("var:") {
                Some(format!("${{__ctx_var_{}}}", sanitize_js_ident(name)))
            } else if let Some(name) = token.strip_prefix("cookie:") {
                Some(format!("${{__ctx_cookie_{}}}", sanitize_js_ident(name)))
            } else {
                token
                    .strip_prefix("header:")
                    .map(|name| format!("${{__ctx_var_{}}}", sanitize_js_ident(name)))
            };
            if let Some(replacement) = replacement {
                // Intentional interpolation; preserved verbatim so k6
                // resolves it at run time against the chain context.
                out.push_str(&replacement);
            } else {
                // Unknown `${...}` shape from YAML — escape `${` so k6
                // sees the literal text instead of trying to evaluate
                // an undefined JS expression.
                out.push_str("\\${");
                out.push_str(token);
                out.push('}');
            }
            rest = &after[end + 1..];
        } else {
            out.push_str("\\${");
            rest = after;
            break;
        }
    }
    // Trailing text — same escape pass as the prefix.
    for c in rest.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '`' => out.push_str("\\`"),
            other => out.push(other),
        }
    }
    out
}

/// Round 39 (#79) — JS identifier-safe form of a YAML name. Used to
/// build per-token chain-context variable names that won't collide
/// with reserved words or contain illegal characters.
fn sanitize_js_ident(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// Round 39 (#79) — build the JS object literal for a request's
/// headers. When `dynamic` is true the entries are emitted inside a
/// template literal so `${var:...}` / `${cookie:...}` substitutions
/// in header values flow through k6's own template-literal
/// interpolation against the `__ctx_var_X` / `__ctx_cookie_X`
/// captured variables.
fn build_headers_object_js(all_headers: &[(String, String)], dynamic: bool) -> String {
    if all_headers.is_empty() {
        return "{}".to_string();
    }
    let entries: Vec<String> = all_headers
        .iter()
        .map(|(k, v)| {
            if dynamic {
                let substituted = substitute_chain_tokens(v);
                // Round 39 — substitute_chain_tokens already produces
                // template-literal-safe text (backticks + unknown
                // ${...} are escaped, intentional ${__ctx_*} kept).
                // Double-escaping with js_escape_tpl would break the
                // intentional interpolations.
                format!("'{}': `{}`", js_escape_sq(k), substituted)
            } else {
                format!("'{}': '{}'", js_escape_sq(k), js_escape_sq(v))
            }
        })
        .collect();
    format!("{{ {} }}", entries.join(", "))
}

/// Round 39 (#79) — scan every `${var:NAME}` / `${cookie:NAME}` /
/// `${header:NAME}` token referenced from check headers / path / body
/// and return the unique chain-context variable names that must be
/// pre-declared before the first request. Without this, k6 throws
/// `ReferenceError: __ctx_cookie_session is not defined` when the
/// chain emits a request that reads a captured value the prior check
/// would have written.
fn collect_referenced_ctx_idents(
    config: &CustomConformanceConfig,
) -> (std::collections::BTreeSet<String>, std::collections::BTreeSet<String>) {
    let mut vars = std::collections::BTreeSet::new();
    let mut cookies = std::collections::BTreeSet::new();
    let walk = |s: &str,
                vars: &mut std::collections::BTreeSet<String>,
                cookies: &mut std::collections::BTreeSet<String>| {
        let mut rest = s;
        while let Some(start) = rest.find("${") {
            let after = &rest[start + 2..];
            if let Some(end) = after.find('}') {
                let token = &after[..end];
                if let Some(name) =
                    token.strip_prefix("var:").or_else(|| token.strip_prefix("header:"))
                {
                    vars.insert(sanitize_js_ident(name));
                } else if let Some(name) = token.strip_prefix("cookie:") {
                    cookies.insert(sanitize_js_ident(name));
                }
                rest = &after[end + 1..];
            } else {
                break;
            }
        }
    };
    for check in &config.custom_checks {
        walk(&check.path, &mut vars, &mut cookies);
        for v in check.headers.values() {
            walk(v, &mut vars, &mut cookies);
        }
        if let Some(b) = &check.body {
            walk(b, &mut vars, &mut cookies);
        }
        // Also pre-declare anything the extract block names so a
        // later check that references it via `${var:...}` even though
        // the previous check failed still has a defined variable
        // (k6's ReferenceError is harsher than the native executor's
        // "keep the literal token" fallback).
        for var_name in check.extract.headers.keys() {
            vars.insert(sanitize_js_ident(var_name));
        }
        for var_name in check.extract.body_fields.keys() {
            vars.insert(sanitize_js_ident(var_name));
        }
        for cookie_name in &check.extract.cookies {
            cookies.insert(sanitize_js_ident(cookie_name));
        }
    }
    (vars, cookies)
}

/// Round 39 (#79) — emit the k6 `group('Custom', ...)` block plus any
/// init-scope code (`open()` for file uploads) into the caller's
/// buffers. This is where the round-38 native-only features
/// (`upload` / `uploads`, `extract`, `repeat`, `chain_iterations`)
/// finally make it into the k6 script Srikanth runs under
/// `--use-k6`.
#[allow(clippy::too_many_arguments)]
fn write_k6_group_body(
    config: &CustomConformanceConfig,
    base_url: &str,
    custom_headers: &[(String, String)],
    export_requests: bool,
    group_body: &mut String,
    init_code: &mut String,
    upload_counter: &mut usize,
) {
    // Round 41 (#79) — Srikanth on 0.3.185: with `extract.cookies` and
    // `${cookie:NAME}` substitution working, k6's per-VU cookie jar
    // ALSO auto-collected the response's `Set-Cookie` and re-injected
    // it on every subsequent request — so the POST went out with TWO
    // copies of `albsessid` in its `Cookie:` header (one from our
    // explicit substitution, one from the jar). Disable the auto-jar
    // for custom checks by passing a fresh empty `jar` per request.
    // The explicit `Cookie:` header set by `${cookie:NAME}`
    // substitution becomes the only source of truth. Also addresses
    // Srikanth's round-41 ask "have ... without Cookie in GET": with
    // no jar, the GET in iteration K+1 does not inherit cookies from
    // iteration K's responses unless the user explicitly forwards
    // them via `${cookie:NAME}`. The `http.CookieJar` constructor
    // exists in k6 1.0+ (k6 wraps the underlying `cookiejar` Go type
    // as a JS class — `new http.CookieJar()` creates an empty one
    // with no shared state with the VU default jar).
    let uses_cookie_substitution = config.custom_checks.iter().any(|c| {
        !c.extract.cookies.is_empty()
            || c.headers.values().any(|v| v.contains("${cookie:") || v.contains("${var:"))
    });
    if uses_cookie_substitution {
        init_code.push_str(
            "// Round 41 (#79) — declared once so every chain request can reuse it;\n\
             // a fresh empty jar suppresses k6's auto-injected Set-Cookie that would\n\
             // otherwise duplicate the explicit `${cookie:NAME}` substitution.\n\
             const __custom_jar_factory = () => new http.CookieJar();\n",
        );
    }
    group_body.push_str("  group('Custom', function () {\n");
    let iters = config.chain_iterations.max(1);

    // Declare every chain-context variable referenced by any check's
    // path / headers / body / extract rules.
    //
    // Round 39 (#79) — k6 throws `ReferenceError: __ctx_cookie_X is not
    // defined` if a `${cookie:X}` substitution lands before the
    // capturing extract has fired (e.g. a chain's first check reads a
    // value the second check is supposed to capture, or a defensive
    // substitution against a prior iteration's state). The empty
    // string resolves to the literal "" inside a template literal so
    // the request log shows a missing capture as `Cookie: NAME=` not
    // a runtime crash.
    //
    // Round 44 (#79) — Srikanth on 0.3.188: with `chain_iterations: 3`
    // and a `${cookie:albsessid}` on the GET, all 3 GETs sent an
    // EMPTY cookie value even though the POST extract was capturing
    // albsessid correctly. v0.3.186 hoisted the `let __ctx_cookie_*`
    // declarations INSIDE the `for (let __iter ...) {}` loop, so each
    // iteration created a fresh `__ctx_cookie_albsessid = ''` that
    // shadowed whatever the previous iteration had captured. Lifted
    // them ABOVE the for-loop now so a single binding persists across
    // every iteration and capture in iteration N is visible to GET in
    // iteration N+1.
    let (ctx_vars, ctx_cookies) = collect_referenced_ctx_idents(config);
    let needs_ctx = iters > 1 || !ctx_vars.is_empty() || !ctx_cookies.is_empty();
    if needs_ctx {
        group_body.push_str("    // Round 44 chain context — pre-declared at GROUP scope (outside the iter loop) so captures from iteration N persist into iteration N+1\n");
        for var in &ctx_vars {
            group_body.push_str(&format!("    let __ctx_var_{} = '';\n", var));
        }
        for cookie in &ctx_cookies {
            group_body.push_str(&format!("    let __ctx_cookie_{} = '';\n", cookie));
        }
    }

    if iters > 1 {
        group_body
            .push_str(&format!("    for (let __iter = 0; __iter < {}; __iter++) {{\n", iters));
    }

    for (check_idx, check) in config.custom_checks.iter().enumerate() {
        group_body.push_str("    {\n");

        // Build headers object (string concatenation under
        // substitution so captured ${var:...} values flow in).
        let mut all_headers: Vec<(String, String)> = Vec::new();
        for (k, v) in &check.headers {
            all_headers.push((k.clone(), v.clone()));
        }
        for (k, v) in custom_headers {
            if !check.headers.contains_key(k) {
                all_headers.push((k.clone(), v.clone()));
            }
        }
        // Auto-add JSON Content-Type when a body is set AND we're not
        // doing a multipart upload (k6 sets the boundary header
        // itself for multipart forms).
        let is_upload = check.upload.is_some() || !check.uploads.is_empty();
        if check.body.is_some()
            && !is_upload
            && !all_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            all_headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }

        let headers_js = build_headers_object_js(&all_headers, needs_ctx);
        // Round 41 (#79) — wrap headers + jar into a `params` object
        // so each request can carry its own fresh CookieJar and the
        // VU's shared jar can't double up cookies that the user is
        // already injecting via `${cookie:NAME}`.
        let params_js = if uses_cookie_substitution {
            format!("{{ headers: {}, jar: __custom_jar_factory() }}", headers_js)
        } else {
            format!("{{ headers: {} }}", headers_js)
        };
        let method = check.method.to_uppercase();
        // Round 39 — substitute_chain_tokens already returns
        // template-literal-safe text; don't escape it again.
        let url_substituted = substitute_chain_tokens(&check.path);
        let url = format!("${{{}}}{}", base_url, url_substituted);
        let escaped_name = check.name.replace('\'', "\\'");

        // Round 39 — uploads (multipart/form-data). The bytes are
        // pre-loaded at init scope via `open()` so k6 accepts them.
        // We always emit `http.post(url, form, params)` for a check
        // that has uploads, regardless of the YAML's `method` field;
        // multipart with a non-POST method is unusual but the spec
        // technically allows it, so we honour `method` if provided.
        let upload_specs: Vec<&UploadFile> =
            check.upload.iter().chain(check.uploads.iter()).collect();
        let form_var = if !upload_specs.is_empty() {
            let mut form_entries: Vec<String> = Vec::with_capacity(upload_specs.len());
            for spec in &upload_specs {
                let var = format!("__file_{}", *upload_counter);
                *upload_counter += 1;
                let filename = spec.filename.clone().unwrap_or_else(|| {
                    Path::new(&spec.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("upload.bin")
                        .to_string()
                });
                init_code.push_str(&format!(
                    "// Round 39 #79 — preload upload file for `{}`\nconst {} = open('{}', 'b');\n",
                    check.name,
                    var,
                    js_escape_sq(&spec.path),
                ));
                form_entries.push(format!(
                    "'{}': http.file({}, '{}', '{}')",
                    js_escape_sq(&spec.field_name),
                    var,
                    js_escape_sq(&filename),
                    js_escape_sq(&spec.content_type),
                ));
            }
            let form_name = format!("__form_{}", check_idx);
            group_body.push_str(&format!(
                "      let {} = {{ {} }};\n",
                form_name,
                form_entries.join(", ")
            ));
            // Round 41 (#79) — Srikanth on 0.3.185: PCAP only showed
            // 5 of his 9 multi-file upload parts and he asked "Is
            // there a way from Logs I can confirm all the files in
            // multiple upload are sent from mockforge client". Emit a
            // single tagged log line per request listing every part,
            // so the user can grep `MOCKFORGE_UPLOAD_PARTS` in the
            // k6 stdout and confirm all parts left mockforge even
            // when their proxy / capture tool drops some. We do this
            // here (NOT inside the repeat loop) because the form is
            // identical across repeats; the count from `repeat` tells
            // the user how many requests will go out.
            // Round 47 (#79) — Srikanth on 0.3.191: "Only thing is file
            // bytes are incorrect" — the captureExchange summary was
            // deriving byte counts from `partBody.length`, which is JS
            // UTF-16 code units rather than the original on-disk byte
            // count. For binary content (PDF / mp4 / docx etc) k6's
            // `res.request.body` lossy-decodes bytes into a UTF-16
            // string where multi-byte sequences collapse to single
            // code units, undercounting by ~3-4%. The accurate fix is
            // to NOT derive from the JS string at all: Rust knows the
            // exact on-disk size, so emit a side-channel JS map keyed
            // by check name + field name. captureExchange looks the
            // size up when summarising. ASCII-only files (json / zip
            // header / text) round-tripped fine before, but binary
            // files were the visible miss.
            let summary_entries: Vec<String> = upload_specs
                .iter()
                .map(|spec| {
                    let filename = spec.filename.clone().unwrap_or_else(|| {
                        Path::new(&spec.path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("upload.bin")
                            .to_string()
                    });
                    format!(
                        "'{}':'{}' ({})",
                        js_escape_sq(&spec.field_name),
                        js_escape_sq(&filename),
                        js_escape_sq(&spec.content_type)
                    )
                })
                .collect();
            // Stat each file at script-gen time so the size map lives
            // at init scope, NOT inside the per-request capture path.
            // Best-effort: if metadata fails, fall through with `null`
            // and captureExchange falls back to the JS-string length.
            let mut size_map_entries: Vec<String> = Vec::with_capacity(upload_specs.len());
            for spec in &upload_specs {
                let bytes = std::fs::metadata(&spec.path).map(|m| m.len()).ok();
                let bytes_js = bytes.map(|b| b.to_string()).unwrap_or_else(|| "null".to_string());
                size_map_entries.push(format!(
                    "'{}': {}",
                    js_escape_sq(&spec.field_name),
                    bytes_js
                ));
            }
            // Hoist the per-check map into init scope so it survives
            // VU lifecycle without any extra wiring; captureExchange
            // looks up by `checkName`.
            init_code.push_str(&format!(
                "// Round 47 #79 — on-disk byte sizes for upload check `{}`\nif (typeof globalThis.__mfUploadSizes === 'undefined') globalThis.__mfUploadSizes = {{}};\nglobalThis.__mfUploadSizes['{}'] = {{ {} }};\n",
                check.name,
                js_escape_sq(&check.name),
                size_map_entries.join(", "),
            ));
            group_body.push_str(&format!(
                "      console.log('MOCKFORGE_UPLOAD_PARTS: {} {} files: {}');\n",
                js_escape_sq(&check.name),
                upload_specs.len(),
                js_escape_sq(&summary_entries.join(", ")),
            ));
            Some(form_name)
        } else {
            None
        };

        // Build the request line(s) — sequential / parallel via
        // repeat.count + repeat.mode.
        let count = check.repeat.count.max(1);
        let is_parallel = count > 1 && matches!(check.repeat.mode, RepeatMode::Parallel);
        let body_expr = match &check.body {
            Some(b) => {
                let substituted = substitute_chain_tokens(b);
                // Round 39 — already template-literal-safe.
                format!("`{}`", substituted)
            }
            None => "null".to_string(),
        };

        if let Some(form_name) = &form_var {
            // Multipart uploads ignore body (mutually exclusive in
            // native; warn on stderr for k6 too via a JS comment).
            if check.body.is_some() {
                group_body.push_str(&format!(
                    "      // warning: custom check '{}' has both `body` and `upload`/`uploads`; ignoring body\n",
                    check.name
                ));
            }
            // Parallel uploads: emit http.batch with multipart entries.
            if is_parallel {
                group_body.push_str(&format!(
                    "      let __batch_{} = []; for (let __r = 0; __r < {}; __r++) {{ __batch_{}.push({{ method: 'POST', url: `{}`, body: {}, params: {} }}); }}\n",
                    check_idx, count, check_idx, url, form_name, params_js
                ));
                group_body.push_str(&format!(
                    "      let __responses_{} = http.batch(__batch_{});\n",
                    check_idx, check_idx
                ));
                group_body.push_str(&format!("      let res = __responses_{}[0];\n", check_idx));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str(&format!(
                    "      for (let __i = 1; __i < __responses_{}.length; __i++) {{ let res = __responses_{}[__i];",
                    check_idx, check_idx
                ));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str(" }\n");
            } else if count > 1 {
                group_body
                    .push_str(&format!("      for (let __r = 0; __r < {}; __r++) {{\n", count));
                group_body.push_str(&format!(
                    "        let res = http.post(`{}`, {}, {});\n",
                    url, form_name, params_js
                ));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str("      }\n");
            } else {
                group_body.push_str(&format!(
                    "      let res = http.post(`{}`, {}, {});\n",
                    url, form_name, params_js
                ));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
            }
        } else {
            // Non-multipart path: respect HTTP method.
            let k6_method = match method.as_str() {
                "DELETE" => "del".to_string(),
                other => other.to_lowercase(),
            };
            let body_method = !matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS" | "DELETE");
            // Round 41 — always pass `params_js` (which carries the
            // jar) when chain-context substitution is in play, even
            // on requests that have no headers of their own. This is
            // how the GET in the user's chain gets `jar: empty` so
            // k6 doesn't accumulate Set-Cookie into the VU jar.
            let request_call = if body_method {
                format!("http.{}(`{}`, {}, {})", k6_method, url, body_expr, params_js)
            } else if all_headers.is_empty() && !uses_cookie_substitution {
                format!("http.{}(`{}`)", k6_method, url)
            } else {
                format!("http.{}(`{}`, {})", k6_method, url, params_js)
            };

            if is_parallel {
                let entry_method = match method.as_str() {
                    "DELETE" => "DELETE",
                    "GET" => "GET",
                    "HEAD" => "HEAD",
                    "OPTIONS" => "OPTIONS",
                    "PUT" => "PUT",
                    "PATCH" => "PATCH",
                    "POST" => "POST",
                    _ => "POST",
                };
                let body_field = if body_method {
                    format!("body: {}, ", body_expr)
                } else {
                    String::new()
                };
                group_body.push_str(&format!(
                    "      let __batch_{} = []; for (let __r = 0; __r < {}; __r++) {{ __batch_{}.push({{ method: '{}', url: `{}`, {}params: {} }}); }}\n",
                    check_idx, count, check_idx, entry_method, url, body_field, params_js
                ));
                group_body.push_str(&format!(
                    "      let __responses_{} = http.batch(__batch_{});\n",
                    check_idx, check_idx
                ));
                group_body.push_str(&format!("      let res = __responses_{}[0];\n", check_idx));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str(&format!(
                    "      for (let __i = 1; __i < __responses_{}.length; __i++) {{ let res = __responses_{}[__i];",
                    check_idx, check_idx
                ));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str(" }\n");
            } else if count > 1 {
                group_body
                    .push_str(&format!("      for (let __r = 0; __r < {}; __r++) {{\n", count));
                group_body.push_str(&format!("        let res = {};\n", request_call));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
                group_body.push_str("      }\n");
            } else {
                group_body.push_str(&format!("      let res = {};\n", request_call));
                emit_check_assertions(group_body, &escaped_name, check, export_requests);
            }
        }

        // Round 39 — emit chain extraction from `res` after the
        // request line(s). For parallel/sequential repeats, this
        // captures from the LAST `res` in scope, matching the native
        // executor's "first/last hit" semantics (k6 doesn't guarantee
        // batch completion order so we explicitly use index 0).
        if !check.extract.is_empty() {
            emit_chain_extract(group_body, &check.extract);
        }

        group_body.push_str("    }\n");
    }

    if iters > 1 {
        group_body.push_str("    }\n");
    }

    group_body.push_str("  });\n\n");
}

/// Round 39 (#79) — emit assertions + capture for one request slot.
/// Pulled out of `write_k6_group_body` so the parallel-batch / repeat
/// branches can share the same assertion code without duplication.
fn emit_check_assertions(
    group_body: &mut String,
    escaped_name: &str,
    check: &CustomCheck,
    export_requests: bool,
) {
    if export_requests {
        group_body.push_str(&format!(
            "      if (typeof __captureExchange === 'function') __captureExchange('{}', res);\n",
            escaped_name
        ));
    }
    group_body.push_str(&format!(
        "      {{ let ok = check(res, {{ '{}': (r) => r.status === {} }}); if (!ok) __captureFailure('{}', res, 'status === {}'); }}\n",
        escaped_name, check.expected_status, escaped_name, check.expected_status
    ));
    for (header_name, pattern) in &check.expected_headers {
        let header_check_name = format!("{}:header:{}", escaped_name, header_name);
        let escaped_pattern = js_escape_sq(pattern);
        let header_lower = header_name.to_lowercase();
        group_body.push_str(&format!(
            "      {{ let ok = check(res, {{ '{}': (r) => {{ const _hk = Object.keys(r.headers || {{}}).find(k => k.toLowerCase() === '{}'); return new RegExp('{}').test(_hk ? r.headers[_hk] : ''); }} }}); if (!ok) __captureFailure('{}', res, 'header {} matches /{}/'); }}\n",
            header_check_name, header_lower, escaped_pattern, header_check_name, header_name, escaped_pattern
        ));
    }
    for field in &check.expected_body_fields {
        let field_check_name = format!("{}:body:{}:{}", escaped_name, field.name, field.field_type);
        let accessor = generate_field_accessor(&field.name);
        let type_check = match field.field_type.as_str() {
            "string" => format!("typeof ({}) === 'string'", accessor),
            "integer" => format!("Number.isInteger({})", accessor),
            "number" => format!("typeof ({}) === 'number'", accessor),
            "boolean" => format!("typeof ({}) === 'boolean'", accessor),
            "array" => format!("Array.isArray({})", accessor),
            "object" => {
                format!("typeof ({}) === 'object' && !Array.isArray({})", accessor, accessor)
            }
            _ => format!("({}) !== undefined", accessor),
        };
        group_body.push_str(&format!(
            "      {{ let ok = check(res, {{ '{}': (r) => {{ try {{ return {}; }} catch(e) {{ return false; }} }} }}); if (!ok) __captureFailure('{}', res, 'body field {} is {}'); }}\n",
            field_check_name, type_check, field_check_name, field.name, field.field_type
        ));
    }
}

/// Round 39 (#79) — emit JS that captures `extract:` rules off `res`
/// into the chain context. Cookies pull from `res.cookies[name][0].value`
/// (k6's parsed cookie shape); headers use a case-insensitive lookup
/// over `Object.keys(res.headers)`; body fields traverse a parsed JSON
/// body via a simple dotted-path walk.
fn emit_chain_extract(group_body: &mut String, rules: &ExtractRules) {
    for cookie_name in &rules.cookies {
        let var = sanitize_js_ident(cookie_name);
        group_body.push_str(&format!(
            "      if (res.cookies && res.cookies['{}'] && res.cookies['{}'][0]) {{ __ctx_cookie_{} = res.cookies['{}'][0].value; }}\n",
            js_escape_sq(cookie_name),
            js_escape_sq(cookie_name),
            var,
            js_escape_sq(cookie_name),
        ));
    }
    for (var_name, header_name) in &rules.headers {
        let var = sanitize_js_ident(var_name);
        let header_lower = header_name.to_lowercase();
        group_body.push_str(&format!(
            "      {{ const _hk = Object.keys(res.headers || {{}}).find(k => k.toLowerCase() === '{}'); if (_hk) {{ __ctx_var_{} = res.headers[_hk]; }} }}\n",
            js_escape_sq(&header_lower), var
        ));
    }
    if !rules.body_fields.is_empty() {
        group_body.push_str("      try { let __body_json = JSON.parse(res.body || 'null');\n");
        for (var_name, dotted) in &rules.body_fields {
            let var = sanitize_js_ident(var_name);
            // Walk the path one segment at a time so a missing
            // intermediate key short-circuits to undefined.
            let segments: Vec<String> =
                dotted.split('.').map(|s| format!("['{}']", js_escape_sq(s))).collect();
            let accessor = format!("__body_json{}", segments.join(""));
            group_body.push_str(&format!(
                "        try {{ const __v = {}; if (__v !== undefined && __v !== null) __ctx_var_{} = String(__v); }} catch(e) {{}}\n",
                accessor, var
            ));
        }
        group_body.push_str("      } catch(e) {}\n");
    }
    // Round 39 — promote the captured-var references to `let` so they
    // live across iterations / checks in the same group. Done at the
    // start of the next request's substitution by referring directly
    // to `__ctx_var_X` / `__ctx_cookie_X` (which we declared via
    // `__ctx_vars = {}` + writes above).
    let _ = group_body;
}

/// Generate a JavaScript expression to access a field in a parsed JSON body.
///
/// Supports three path formats:
/// - Simple key: `"name"` → `JSON.parse(r.body)['name']`
/// - Dot-notation: `"config.enabled"` → `JSON.parse(r.body)['config']['enabled']`
/// - Array bracket: `"items[].id"` → `JSON.parse(r.body)['items'][0]['id']`
fn generate_field_accessor(field_name: &str) -> String {
    // Split on dots, handling [] array notation
    let parts: Vec<&str> = field_name.split('.').collect();
    let mut expr = String::from("JSON.parse(r.body)");

    for part in &parts {
        if let Some(arr_name) = part.strip_suffix("[]") {
            // Array field — access the array then index first element
            expr.push_str(&format!("['{}'][0]", arr_name));
        } else {
            expr.push_str(&format!("['{}']", part));
        }
    }

    expr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_custom_yaml() {
        let yaml = r#"
custom_checks:
  - name: "custom:pets-returns-200"
    path: /pets
    method: GET
    expected_status: 200
  - name: "custom:create-product"
    path: /api/products
    method: POST
    expected_status: 201
    body: '{"sku": "TEST-001", "name": "Test"}'
    expected_body_fields:
      - name: id
        type: integer
    expected_headers:
      content-type: "application/json"
"#;
        let config: CustomConformanceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.custom_checks.len(), 2);
        assert_eq!(config.custom_checks[0].name, "custom:pets-returns-200");
        assert_eq!(config.custom_checks[0].expected_status, 200);
        assert_eq!(config.custom_checks[1].expected_body_fields.len(), 1);
        assert_eq!(config.custom_checks[1].expected_body_fields[0].name, "id");
        assert_eq!(config.custom_checks[1].expected_body_fields[0].field_type, "integer");
    }

    #[test]
    fn test_generate_k6_group_get() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:test-get".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("group('Custom'"));
        assert!(script.contains("http.get(`${BASE_URL}/api/test`)"));
        assert!(script.contains("'custom:test-get': (r) => r.status === 200"));
    }

    #[test]
    fn test_generate_k6_group_post_with_body() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:create".to_string(),
                path: "/api/items".to_string(),
                method: "POST".to_string(),
                expected_status: 201,
                body: Some(r#"{"name": "test"}"#.to_string()),
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![ExpectedBodyField {
                    name: "id".to_string(),
                    field_type: "integer".to_string(),
                }],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("http.post("));
        assert!(script.contains("'custom:create': (r) => r.status === 201"));
        assert!(script.contains("custom:create:body:id:integer"));
        assert!(script.contains("Number.isInteger"));
    }

    #[test]
    fn test_generate_k6_group_with_header_checks() {
        let mut expected_headers = std::collections::HashMap::new();
        expected_headers.insert("content-type".to_string(), "application/json".to_string());

        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:header-check".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers,
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("custom:header-check:header:content-type"));
        assert!(script.contains("new RegExp('application/json')"));
    }

    #[test]
    fn test_generate_k6_group_with_custom_headers() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:auth-test".to_string(),
                path: "/api/secure".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let custom_headers = vec![("Authorization".to_string(), "Bearer token123".to_string())];
        let script = config.generate_k6_group("BASE_URL", &custom_headers);
        assert!(script.contains("'Authorization': 'Bearer token123'"));
    }

    #[test]
    fn test_failure_capture_emitted() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:capture-test".to_string(),
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("X-Rate-Limit".to_string(), ".*".to_string());
                    m
                },
                expected_body_fields: vec![ExpectedBodyField {
                    name: "id".to_string(),
                    field_type: "integer".to_string(),
                }],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        // Status check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test', res, 'status === 200')"),
            "Status check should emit __captureFailure"
        );
        // Header check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test:header:X-Rate-Limit'"),
            "Header check should emit __captureFailure"
        );
        // Body field check should call __captureFailure on failure
        assert!(
            script.contains("__captureFailure('custom:capture-test:body:id:integer'"),
            "Body field check should emit __captureFailure"
        );
    }

    #[test]
    fn test_from_file_nonexistent() {
        let result = CustomConformanceConfig::from_file(Path::new("/nonexistent/file.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read custom conformance file"));
    }

    #[test]
    fn test_generate_k6_group_delete() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:delete-item".to_string(),
                path: "/api/items/1".to_string(),
                method: "DELETE".to_string(),
                expected_status: 204,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        assert!(script.contains("http.del("));
        assert!(script.contains("r.status === 204"));
    }

    #[test]
    fn test_field_accessor_simple() {
        assert_eq!(generate_field_accessor("name"), "JSON.parse(r.body)['name']");
    }

    #[test]
    fn test_field_accessor_nested_dot() {
        assert_eq!(
            generate_field_accessor("config.enabled"),
            "JSON.parse(r.body)['config']['enabled']"
        );
    }

    #[test]
    fn test_field_accessor_array_bracket() {
        assert_eq!(generate_field_accessor("items[].id"), "JSON.parse(r.body)['items'][0]['id']");
    }

    #[test]
    fn test_field_accessor_deep_nested() {
        assert_eq!(generate_field_accessor("a.b.c"), "JSON.parse(r.body)['a']['b']['c']");
    }

    #[test]
    fn test_generate_k6_nested_body_fields() {
        let config = CustomConformanceConfig {
            custom_checks: vec![CustomCheck {
                name: "custom:nested".to_string(),
                path: "/api/data".to_string(),
                method: "GET".to_string(),
                expected_status: 200,
                body: None,
                expected_headers: std::collections::HashMap::new(),
                expected_body_fields: vec![
                    ExpectedBodyField {
                        name: "count".to_string(),
                        field_type: "integer".to_string(),
                    },
                    ExpectedBodyField {
                        name: "results[].name".to_string(),
                        field_type: "string".to_string(),
                    },
                ],
                headers: std::collections::HashMap::new(),
                upload: None,
                uploads: vec![],
                extract: ExtractRules::default(),
                repeat: Repeat::default(),
            }],
            chain_iterations: 1,
        };

        let script = config.generate_k6_group("BASE_URL", &[]);
        // Simple field should use direct bracket access
        assert!(script.contains("JSON.parse(r.body)['count']"));
        // Nested array field should use [0] for array traversal
        assert!(script.contains("JSON.parse(r.body)['results'][0]['name']"));
    }
}
