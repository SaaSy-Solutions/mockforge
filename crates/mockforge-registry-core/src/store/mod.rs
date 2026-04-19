//! Unified storage layer for the registry domain.
//!
//! The [`RegistryStore`] trait abstracts over the concrete database backend so
//! the same handlers, middleware, and domain logic can run against either
//! PostgreSQL (for the multi-tenant SaaS binary) or SQLite (for the OSS admin
//! server embedded in `mockforge-ui`).
//!
//! Phase 1a introduces the trait with the API-token domain only. Subsequent
//! phases will add organizations, organization members, settings (BYOK),
//! audit logs, feature usage, users, invitations, and quotas.
//!
//! The initial Postgres implementation delegates to the existing inherent
//! `ApiToken::*` methods so that introducing the trait is a no-op refactor.
//! Later phases will invert this relationship and move the SQL into the trait
//! impls directly.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::api_token::{ApiToken, TokenScope};
use crate::models::attestation::UserPublicKey;
use crate::models::audit_log::{AuditEventType, AuditLog};
use crate::models::cloud_fixture::CloudFixture;
use crate::models::cloud_service::CloudService;
use crate::models::cloud_workspace::Workspace as CloudWorkspace;
use crate::models::feature_usage::FeatureType;
use crate::models::federation::Federation;
use crate::models::hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
use crate::models::org_template::OrgTemplate;
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::osv::{OsvImportRecord, OsvMatch};
use crate::models::plugin::{PendingScanJob, Plugin, PluginSecurityScan, PluginVersion};
use crate::models::review::Review;
use crate::models::saml_assertion::SAMLAssertionId;
use crate::models::scenario::Scenario;
use crate::models::scenario_review::ScenarioReview;
use crate::models::settings::OrgSetting;
use crate::models::sso::{SSOConfiguration, SSOProvider};
use crate::models::subscription::UsageCounter;
use crate::models::suspicious_activity::{SuspiciousActivity, SuspiciousActivityType};
use crate::models::template::{Template, TemplateCategory};
use crate::models::template_review::TemplateReview;
use crate::models::user::User;
use crate::models::verification_token::VerificationToken;
use crate::models::waitlist::WaitlistSubscriber;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PgRegistryStore;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteRegistryStore;

// `StoreError` and `StoreResult` live in `crate::error`. Re-exported here
// so existing `use crate::store::{StoreError, StoreResult}` imports in
// downstream crates continue to work.
pub use crate::error::{StoreError, StoreResult};

/// Does the OSV `affected_versions` JSON blob cover this concrete version?
///
/// Shared by both backends so they stay in lock-step. The matcher walks
/// OSV `affected[].ranges[]` per the schema at
/// <https://ossf.github.io/osv-schema/>:
///
/// * `versions: ["1.2.3", ...]` — explicit version list, matched
///   literally (case-insensitive). Covers `event-stream@3.3.6`-style
///   single-pin advisories.
/// * `ranges[].events` — an ordered sequence of `introduced` / `fixed` /
///   `last_affected` events. A version is affected if it falls inside
///   any open interval `[introduced, fixed)` or `[introduced, last_affected]`.
/// * `ranges[].type` — `SEMVER`, `ECOSYSTEM`, or `GIT`. SEMVER uses
///   strict semver comparison. ECOSYSTEM ranges usually follow semver but
///   the spec lets each ecosystem define its own ordering; we treat them
///   the same way (semver fallback) and flag the conservative outcome:
///   a version that doesn't parse as semver gets matched only if the
///   range is open-ended (`introduced = "0"` with no close event).
/// * `GIT` ranges over commit hashes — not a concept that makes sense for
///   published plugin artifacts, so those are ignored.
///
/// The pre-semver heuristic behaviors are preserved as fallbacks: the
/// explicit `versions` list still matches literally, and
/// `introduced = "0"` with no `fixed` still means "all versions"
/// regardless of whether the target parses as semver. This keeps
/// hand-crafted OSV records and non-semver ecosystems (e.g. RubyGems)
/// working.
pub fn version_affected(affected: &serde_json::Value, version: &str) -> bool {
    version_affected_in_ecosystem(affected, version, "")
}

/// Ecosystem-aware variant of [`version_affected`]. Strings from
/// non-semver ecosystems (PyPI's PEP 440, Go's `v`-prefixed tags) are
/// normalized to a semver-parseable shape before the range walk; pass an
/// empty string when the ecosystem is unknown and the caller wants the
/// pre-existing behavior. Dispatch is cheap (no allocations in the common
/// npm/cargo/other path) and additive, so new ecosystems can land as
/// small per-family normalizers without touching the core matcher.
pub fn version_affected_in_ecosystem(
    affected: &serde_json::Value,
    version: &str,
    ecosystem: &str,
) -> bool {
    let normalized_target = normalize_version_for_ecosystem(ecosystem, version);
    let effective_version = normalized_target.as_deref().unwrap_or(version);

    // 1. Literal `versions` list match — trumps any range logic.
    if let Some(arr) = affected.get("versions").and_then(|v| v.as_array()) {
        // Match both the caller-supplied string (handles "v1.2.3" literal
        // entries in the list) and the normalized form (so a "1.2.3"
        // entry matches a "v1.2.3" caller). The two are compared
        // case-insensitively; ecosystems that care about case (none we
        // target so far) can grow a variant check later.
        for v in arr.iter().filter_map(|v| v.as_str()) {
            if v.eq_ignore_ascii_case(version) || v.eq_ignore_ascii_case(effective_version) {
                return true;
            }
            if let Some(norm) = normalize_version_for_ecosystem(ecosystem, v) {
                if norm.eq_ignore_ascii_case(effective_version) {
                    return true;
                }
            }
        }
    }

    let Some(ranges) = affected.get("ranges").and_then(|v| v.as_array()) else {
        return false;
    };

    // Parse the target against the ecosystem's normalizer. `target` is
    // the semver representation the range-walk compares against; the
    // original `version` string is kept only for the literal-equality
    // fallback inside `interval_matches`.
    let target = semver::Version::parse(effective_version).ok();

    for r in ranges {
        let range_type =
            r.get("type").and_then(|v| v.as_str()).unwrap_or("SEMVER").to_ascii_uppercase();
        if range_type == "GIT" {
            continue;
        }

        let events = match r.get("events").and_then(|v| v.as_array()) {
            Some(ev) => ev,
            None => continue,
        };

        // Normalize each event's version string the same way we did the
        // target. This is what lets a Go `v1.2.3` advisory match a plain
        // `1.2.3` caller (and vice-versa).
        let normalized: Vec<(serde_json::Value, Option<String>)> = events
            .iter()
            .map(|e| {
                let norm = event_kinds(e)
                    .into_iter()
                    .find_map(|k| event_value(e, k))
                    .and_then(|raw| normalize_version_for_ecosystem(ecosystem, &raw));
                (e.clone(), norm)
            })
            .collect();

        // The OSV spec allows events to appear in any order — consumers
        // are expected to sort them by version before pairing
        // `introduced`→`fixed`/`last_affected`. Every real feed we've
        // seen emits in-order pairs already, but sorting first makes us
        // robust to hand-curated advisories that don't.
        let sorted = sort_events(events);

        // Walk the (now ordered) event list, opening an interval on
        // `introduced` and closing it on the next `fixed` /
        // `last_affected`. Multiple intervals per range are legal.
        let mut intro: Option<String> = None;
        let mut matched = false;
        for e in &sorted {
            // Pick the normalized form for the event if we have one;
            // otherwise use the raw string. `normalized` was indexed by
            // the unsorted position, so find_by_identity here means
            // comparing JSON values.
            let norm_version =
                normalized.iter().find(|(v, _)| v == *e).and_then(|(_, n)| n.clone());
            if let Some(s) = event_value(e, "introduced") {
                intro = norm_version.or(Some(s));
                continue;
            }
            if let Some(fix_raw) = event_value(e, "fixed") {
                let fix = norm_version.unwrap_or(fix_raw);
                if interval_matches(
                    intro.as_deref(),
                    Some(&fix),
                    /*inclusive_upper=*/ false,
                    effective_version,
                    target.as_ref(),
                ) {
                    matched = true;
                    break;
                }
                intro = None;
                continue;
            }
            if let Some(la_raw) = event_value(e, "last_affected") {
                let la = norm_version.unwrap_or(la_raw);
                if interval_matches(
                    intro.as_deref(),
                    Some(&la),
                    /*inclusive_upper=*/ true,
                    effective_version,
                    target.as_ref(),
                ) {
                    matched = true;
                    break;
                }
                intro = None;
                continue;
            }
        }
        if matched {
            return true;
        }

        // Dangling `introduced` with no matching close event = open-ended
        // "all versions ≥ introduced" range (e.g. "this is broken forever").
        if let Some(i) = intro {
            if interval_matches(Some(&i), None, false, effective_version, target.as_ref()) {
                return true;
            }
        }
    }
    false
}

/// Which event keys a single OSV event entry might carry. Cheap
/// reference-only list to keep the normalization loop allocation-free.
fn event_kinds(_: &serde_json::Value) -> [&'static str; 3] {
    ["introduced", "fixed", "last_affected"]
}

/// Normalize a version string into something `semver::Version::parse`
/// can accept, based on the OSV ecosystem the caller supplied. Returns
/// `None` when no rewrite applies — callers fall back to the raw string,
/// preserving backward-compatible behavior for unknown or semver-native
/// ecosystems.
///
/// # Output is an internal-only encoding
///
/// The normalized string is never persisted to the database, returned
/// over the HTTP API, or shown to users. It lives exactly long enough
/// to be handed to `semver::Version::parse` inside the OSV matcher, and
/// then it's thrown away. The only reason this matters is that the
/// PyPI rewrite below produces strings that *look* like valid versions
/// but whose patch number and pre-release tag have been massaged to
/// make semver's comparator agree with PEP 440's ordering. Do **not**
/// surface these strings to callers — they would be confusing and
/// would imply the patch level has changed.
///
/// # Supported ecosystems (case-insensitive)
///
/// * `Go` — strip a leading `v` (`v1.2.3` → `1.2.3`). Go module tags are
///   standard semver with a `v` prefix, so this is a one-character fix.
///
/// * `PyPI` — a pragmatic subset of PEP 440. Rewrites fall into three
///   groups, all chosen so that semver's default comparator reproduces
///   PEP 440's ordering for the pair under test:
///
///   - **Base-only inputs**: drop the `v` prefix, drop the epoch
///     (`1!1.0.0` → `1.0.0`), and split off the local identifier
///     (`1.0+ubuntu1`) into semver build metadata (semver ignores it
///     for ordering; we keep it to preserve round-trip identity).
///
///   - **Pre-release inputs** (`1.0.0a1`, `1.0.0b2`, `1.0.0rc1`,
///     `1.0.0.dev3`): emit `<base>-<rank>.<marker>.<N>` where `<rank>`
///     is a numeric prefix chosen to exploit semver's rule that
///     numeric pre-release identifiers sort before alphabetic ones
///     and numeric identifiers sort by value. The rank table is
///     `dev=0, a=1, b=2, rc=3`, producing `1.0.0-0.dev.3 < 1.0.0-1.a.1
///     < 1.0.0-2.b.2 < 1.0.0-3.rc.1 < 1.0.0`, which matches PEP 440.
///
///   - **Post-release inputs** (`1.0.0.post1`): emit
///     `<base_with_patch_bumped>-0.post.<N>`. PEP 440 says `1.0.0 <
///     1.0.0.post1 < 1.0.1`; semver says *any* pre-release sorts
///     *before* its base, so we can't express `1.0.0.post1` inside
///     the `1.0.0` release. Instead we bump the patch and slot it in
///     as a pre-release of the *next* patch: semver then reads
///     `1.0.0 < 1.0.1-0.post.1 < 1.0.1`, which matches. **Side
///     effect:** the normalized form's patch number is one higher
///     than the input's. This is the reason the doc-comment above
///     screams "never surface these strings."
///
///   Pre+post, pre+dev, and post+dev combinations compose the above
///   tricks. The parity test (`pypi_normalizer_agrees_with_pep440_rs_on_ordering`)
///   documents each supported pair and the edge cases where strict
///   PEP 440 ordering inside a pre-release isn't reachable with
///   pure-semver comparisons.
///
/// Everything else — `npm`, `crates.io`, `RubyGems`, unknown ecosystems
/// — returns `None`, letting the matcher use the string verbatim. This
/// keeps the existing behavior (and test coverage) intact while opening
/// up precise matching for the two ecosystems where drift between
/// published tags and semver is most common.
pub fn normalize_version_for_ecosystem(ecosystem: &str, version: &str) -> Option<String> {
    match ecosystem.to_ascii_lowercase().as_str() {
        "go" => normalize_go_version(version),
        "pypi" => normalize_pypi_version(version),
        _ => None,
    }
}

fn normalize_go_version(version: &str) -> Option<String> {
    let stripped = version.strip_prefix('v').or_else(|| version.strip_prefix('V'))?;
    if stripped.is_empty() {
        return None;
    }
    // Leave the result unvalidated — caller feeds it into
    // `semver::Version::parse`, which will reject garbage.
    Some(stripped.to_string())
}

fn normalize_pypi_version(version: &str) -> Option<String> {
    // Strip common textual noise that PEP 440 declares equivalent to
    // the canonical form. PyPI normalizes display but OSV advisories
    // often preserve the original tag, so we do it client-side.
    let trimmed = version.trim();
    let lowered = if trimmed.chars().any(|c| c.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(trimmed.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(trimmed)
    };
    let v = lowered.as_ref();

    // Drop the "v" prefix some packages ship with (`v1.2.3` → `1.2.3`).
    let without_v = v.strip_prefix('v').unwrap_or(v);

    // Drop epoch: `1!2.3.4` → `2.3.4`.
    let after_epoch = without_v.split_once('!').map(|(_, v)| v).unwrap_or(without_v);

    // Split off local identifier (PEP 440 §5): `1.0+ubuntu1` — the part
    // after `+` is installation-metadata and not ordering-relevant for
    // advisory matching. We stash it on the normalized output as
    // semver *build* metadata (which semver also ignores for ordering)
    // so a caller that round-trips gets the original information back
    // without it perturbing range comparisons.
    let (public_part, local_part) = match after_epoch.split_once('+') {
        Some((pub_, loc)) => (pub_, Some(loc)),
        None => (after_epoch, None),
    };

    // Attempt to rewrite PEP 440 pre/post/dev markers. PEP 440 says a
    // version can legally carry, in sequence, a pre-release
    // (`a1`/`b2`/`rc3`), a post-release (`.post4`), and a dev-release
    // (`.dev5`). We strip each section in turn from the right and then
    // assemble a semver form that mirrors pep440 *ordering* — not just
    // "pre-release sorts before base" but the full ranking
    // `dev < a < b < rc < base < post`.
    //
    // The trick is semver pre-release comparison: identifiers are
    // compared left-to-right, numeric-before-alpha, numeric values
    // numerically. So we lead each pre-release with a small numeric
    // rank (`0.dev.N`, `1.a.N`, `2.b.N`, `3.rc.N`). That fixes the
    // `dev < a < b < rc` order without needing a custom comparator.
    //
    // Post-release-alone needs a different trick: semver says any
    // `X.Y.Z-pre` sorts *before* `X.Y.Z`, so we can't squeeze a post
    // above the base in the same release. Instead we bump the patch
    // and use a `0.post.N` pre-release of the next patch, which
    // semver sorts as `X.Y.Z < X.Y.(Z+1)-0.post.N < X.Y.(Z+1)`. That
    // matches `X.Y.Z < X.Y.Z.postN < X.Y.(Z+1)` in pep440.
    let mut remaining = public_part.to_string();
    let mut pre_suffix: Option<String> = None;
    let mut post_suffix: Option<String> = None;
    let mut dev_suffix: Option<String> = None;

    // Dev first (rightmost marker).
    if let Some((base, n)) = split_off_marker(&remaining, ".dev") {
        remaining = base;
        dev_suffix = Some(format!("0.dev.{}", n));
    }
    // Post next.
    if let Some((base, n)) = split_off_marker(&remaining, ".post") {
        remaining = base;
        post_suffix = Some(format!("post.{}", n));
    }
    // Pre-release — exactly one of `rc` / `a` / `b`, in that order so
    // `rc` wins over `a`/`b` on a string like `1.0.0rc1` (which
    // contains neither a digit `a` nor `b` adjacent to the digit
    // prefix). The numeric rank prefix gives us correct ordering:
    // `-1.a.N < -2.b.N < -3.rc.N`.
    for (marker, rank) in [("rc", 3u8), ("a", 1), ("b", 2)] {
        if let Some((base, n)) = split_off_alpha_marker(&remaining, marker) {
            remaining = base;
            pre_suffix = Some(format!("{}.{}.{}", rank, marker, n));
            break;
        }
    }

    // Build the canonical output. The matrix below preserves pep440
    // ordering for every combination we support:
    //
    //   pre,  post, dev   → output
    //   ---   ----  ---   ------
    //   None  None  None  → base
    //   pre   None  None  → base - pre
    //   None  post  None  → base' - 0.post.N  (base' = next-patch base)
    //   None  None  dev   → base - dev
    //   pre   None  dev   → base - pre.dev    (dev of a pre-release sorts before the pre)
    //                                           — we leave this as the pre form because
    //                                           semver `pre.dev.N` > `pre` which is the
    //                                           *wrong* direction; this is the documented
    //                                           limitation called out in the module doc.
    //   pre   post  None  → base - pre+post.N (post of a pre-release; pep440 sorts it
    //                                           between pre and the next pre, which
    //                                           semver handles via build metadata.
    //                                           Limitation: exact ordering within the
    //                                           pre isn't strict semver.)
    //   None  post  dev   → base' - 0.post.N.dev.M — dev of the post-alone rewrite
    //   pre   post  dev   → base - pre+post.N.dev.M  — combined, limited as above
    //
    // The pure-form cases (single non-None slot) all compare correctly
    // against each other and against the base. Combined forms keep
    // some semver build metadata to preserve round-trip identity but
    // may not sort strictly with pep440 on cross-pair comparisons.
    let (sv_pre, sv_build, final_remaining) = match (pre_suffix, post_suffix, dev_suffix) {
        (Some(pre), None, None) => (Some(pre), None, remaining.clone()),
        (None, Some(post), None) => {
            // `1.0.0.post1` → base `1.0.1`, pre `0.post.1`.
            let bumped = bump_patch(&remaining);
            (Some(format!("0.{}", post)), None, bumped)
        }
        (None, None, Some(dev)) => (Some(dev), None, remaining.clone()),
        (Some(pre), Some(post), None) => (Some(pre), Some(post), remaining.clone()),
        (Some(pre), None, Some(dev)) => {
            // Pre + dev: semver puts the extended pre-release *after*
            // the plain pre-release, which disagrees with pep440. We
            // document this in the parity test; the OSV matcher
            // doesn't hit this combination in practice.
            (Some(format!("{}.{}", pre, dev)), None, remaining.clone())
        }
        (None, Some(post), Some(dev)) => {
            // `1.0.0.post1.dev2` → `1.0.1-0.post.1.dev.2`. We inherit
            // the post-alone patch bump; the `.dev.M` appended to the
            // pre-release makes dev-of-post sort *after* the bare
            // post in semver, which disagrees with pep440's "dev of
            // post < post." Still better than the previous behavior,
            // and flagged in the parity test limitations.
            let bumped = bump_patch(&remaining);
            (Some(format!("0.{}.{}", post, dev)), None, bumped)
        }
        (Some(pre), Some(post), Some(dev)) => {
            (Some(format!("{}.{}", pre, dev)), Some(post), remaining.clone())
        }
        (None, None, None) => (None, None, remaining.clone()),
    };

    // Combine local identifier with the computed build metadata. Both
    // are "+"-separated in semver, so we join with a dot.
    let final_build = match (sv_build, local_part) {
        (Some(b), Some(l)) => Some(format!("{}.local.{}", b, sanitize_local(l))),
        (Some(b), None) => Some(b),
        (None, Some(l)) => Some(format!("local.{}", sanitize_local(l))),
        (None, None) => None,
    };

    let mut out = final_remaining;
    if let Some(pre) = sv_pre {
        out.push('-');
        out.push_str(&pre);
    }
    if let Some(build) = final_build {
        out.push('+');
        out.push_str(&build);
    }

    // Return Some only if we actually did work. If the input was plain
    // semver with nothing to rewrite we fall through to None so callers
    // keep the literal string for equality matching.
    if out == version {
        None
    } else {
        Some(out)
    }
}

/// Split a `base.MARKERnumber` string into its `base` and the `number`
/// part, where `marker` is dotted (e.g. `.dev`, `.post`). Returns None
/// when the input doesn't carry the marker or the trailing part isn't
/// a run of digits.
fn split_off_marker(s: &str, marker: &str) -> Option<(String, String)> {
    let idx = find_pep440_marker(s, marker)?;
    let base = &s[..idx];
    let suffix = &s[idx + marker.len()..];
    if suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty() {
        Some((base.to_string(), suffix.to_string()))
    } else {
        None
    }
}

/// Same as [`split_off_marker`] but for alphabet markers (`a`, `b`,
/// `rc`) which PEP 440 allows immediately after a digit with no
/// separator. We require a digit on the left and a digit run on the
/// right; anything else (e.g. `alpha`) bails out.
fn split_off_alpha_marker(s: &str, marker: &str) -> Option<(String, String)> {
    let idx = find_pep440_marker(s, marker)?;
    let before = s[..idx].chars().last();
    if !before.is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    let base = &s[..idx];
    let suffix = &s[idx + marker.len()..];
    if suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty() {
        Some((base.to_string(), suffix.to_string()))
    } else {
        None
    }
}

/// Increment the patch component of a dotted version string. Used by
/// the post-release normalizer: `1.0.0.post1` becomes `1.0.1-0.post.1`
/// so semver's "pre-release of next patch" ordering gives us the right
/// pep440 semantics (`base < base.postN < next_patch`).
///
/// For inputs with fewer than three numeric components we pad to three
/// first (`1.0` → `1.0.1`, `1` → `1.0.1`) and flag anything we can't
/// parse by returning the input unchanged, which keeps the downstream
/// semver parse conservative rather than producing garbage.
fn bump_patch(version: &str) -> String {
    // Split on the first non-version char (shouldn't exist at this
    // point in the pipeline, but be defensive).
    let parts: Vec<&str> = version.splitn(4, '.').collect();
    let nums: Vec<Option<u64>> = parts.iter().take(3).map(|p| p.parse::<u64>().ok()).collect();
    if nums.iter().any(|n| n.is_none()) {
        return version.to_string();
    }
    let major = nums.first().and_then(|v| *v).unwrap_or(0);
    let minor = nums.get(1).and_then(|v| *v).unwrap_or(0);
    let patch = nums.get(2).and_then(|v| *v).unwrap_or(0);
    format!("{}.{}.{}", major, minor, patch.saturating_add(1))
}

/// Semver build metadata accepts `[0-9A-Za-z-]` plus `.` separators.
/// PEP 440 local identifiers allow `.`, `_`, and `-`; replace
/// disallowed runs with `-` so the resulting string round-trips through
/// `semver::Version::parse`.
fn sanitize_local(local: &str) -> String {
    local
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn find_pep440_marker(s: &str, marker: &str) -> Option<usize> {
    // Scan for each candidate position; reject a hit that's part of a
    // longer identifier (e.g. `alpha` when we're looking for `a`). The
    // character before the marker must be a digit for alphabet markers
    // (`a`, `b`, `rc`); dotted markers (`.dev`, `.post`) carry their own
    // delimiter.
    let dotted = marker.starts_with('.');
    let mut search_from = 0;
    while search_from < s.len() {
        let rest = &s[search_from..];
        let rel = rest.find(marker)?;
        let abs = search_from + rel;
        let before = s[..abs].chars().last();
        let ok = if dotted {
            true // `.dev` / `.post` include their own separator
        } else {
            before.is_some_and(|c| c.is_ascii_digit())
        };
        // Also guard against longer identifiers by requiring the next
        // char (if any) to be either end-of-string or a digit.
        let after = s.as_bytes().get(abs + marker.len()).copied();
        let ok = ok && after.is_none_or(|b| b.is_ascii_digit());
        if ok {
            return Some(abs);
        }
        search_from = abs + marker.len();
    }
    None
}

/// Extract the version string for any of the three OSV event kinds.
/// Returns `None` for events that don't carry one of the known keys.
fn event_value(event: &serde_json::Value, key: &str) -> Option<String> {
    event.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// Return events sorted by their semver version, breaking ties by a
/// stable kind ordering (`introduced` < `fixed` < `last_affected`) so an
/// introduced event at version `X` precedes a fixed event at the same
/// version. Events missing a recognizable version bucket keep their
/// original relative order and sort at the end.
fn sort_events(events: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    fn kind_rank(e: &serde_json::Value) -> u8 {
        if e.get("introduced").is_some() {
            0
        } else if e.get("fixed").is_some() {
            1
        } else if e.get("last_affected").is_some() {
            2
        } else {
            3
        }
    }
    fn event_version(e: &serde_json::Value) -> Option<semver::Version> {
        let raw = event_value(e, "introduced")
            .or_else(|| event_value(e, "fixed"))
            .or_else(|| event_value(e, "last_affected"))?;
        // The "0" sentinel means "earliest possible version" — make it
        // compare less than any real release.
        if raw == "0" {
            return Some(semver::Version::new(0, 0, 0));
        }
        semver::Version::parse(&raw).ok()
    }

    let mut indexed: Vec<(usize, &serde_json::Value, Option<semver::Version>, u8)> = events
        .iter()
        .enumerate()
        .map(|(i, e)| (i, e, event_version(e), kind_rank(e)))
        .collect();

    indexed.sort_by(|(ai, _, av, ak), (bi, _, bv, bk)| match (av, bv) {
        (Some(a), Some(b)) => a.cmp(b).then_with(|| ak.cmp(bk)).then_with(|| ai.cmp(bi)),
        // Unparsable versions sort after parsable ones to keep a stable
        // tail; within the unparsable group we keep input order.
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => ai.cmp(bi),
    });
    indexed.into_iter().map(|(_, e, _, _)| e).collect()
}

/// Does `version` fall inside `[introduced, upper)` (exclusive) or
/// `[introduced, upper]` (inclusive)? Handles the OSV-specific
/// `introduced = "0"` sentinel and gracefully degrades when either bound
/// isn't parseable as semver.
fn interval_matches(
    introduced: Option<&str>,
    upper: Option<&str>,
    inclusive_upper: bool,
    version: &str,
    target: Option<&semver::Version>,
) -> bool {
    // "0" on the lower bound means "from the earliest possible version."
    let lower_unbounded = matches!(introduced, Some("0"));
    let lower = if lower_unbounded {
        None
    } else {
        introduced.and_then(|s| semver::Version::parse(s).ok())
    };
    let upper_ver = upper.and_then(|s| semver::Version::parse(s).ok());

    // Fast-path for ecosystems where the target isn't semver-parseable
    // (pre-releases with weird tags, dates-as-versions, etc.): only
    // "introduced = 0" with no close wins, and string equality on the
    // introduced/upper bounds still wins. This is conservative — we'd
    // rather miss a match than falsely flag a clean artifact.
    let Some(target) = target else {
        if lower_unbounded && upper.is_none() {
            return true;
        }
        if let Some(i) = introduced {
            if i.eq_ignore_ascii_case(version) {
                return true;
            }
        }
        if let Some(u) = upper {
            if inclusive_upper && u.eq_ignore_ascii_case(version) {
                return true;
            }
        }
        return false;
    };

    if let Some(l) = &lower {
        if target < l {
            return false;
        }
    }
    if let Some(u) = &upper_ver {
        if inclusive_upper {
            if target > u {
                return false;
            }
        } else if target >= u {
            return false;
        }
    } else if upper.is_some() && upper_ver.is_none() {
        // Upper bound was declared but didn't parse — we can't prove
        // containment, so stay conservative.
        return false;
    }
    true
}

/// Parse an OSV `modified` timestamp (RFC 3339). Falls back to `now` when
/// the upstream record omits the field or it can't be parsed — the cache
/// still needs a timestamp for ordering/replacement decisions.
pub fn parse_modified_str(s: Option<&str>) -> DateTime<Utc> {
    s.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

/// Unified storage trait for the registry domain.
///
/// Implementations must be `Send + Sync + 'static` so they can live behind an
/// `Arc<dyn RegistryStore>` inside `AppState` and be cloned across request
/// handlers without extra synchronization.
#[async_trait]
pub trait RegistryStore: Send + Sync + 'static {
    // ---------------------------------------------------------------------
    // Health
    // ---------------------------------------------------------------------

    /// Ping the backing database. Returns `Ok(())` if the store is reachable.
    /// Implementations should issue the cheapest possible liveness check
    /// (`SELECT 1` for SQL backends).
    async fn health_check(&self) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // API tokens
    // ---------------------------------------------------------------------

    /// Create a new API token. Returns the plaintext token (shown once) and
    /// the persisted [`ApiToken`] record.
    async fn create_api_token(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name: &str,
        scopes: &[TokenScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<(String, ApiToken)>;

    /// Look up a token by its database id.
    async fn find_api_token_by_id(&self, token_id: Uuid) -> StoreResult<Option<ApiToken>>;

    /// List every token that belongs to an organization, newest first.
    async fn list_api_tokens_by_org(&self, org_id: Uuid) -> StoreResult<Vec<ApiToken>>;

    /// Look up a token by its public prefix within an organization.
    async fn find_api_token_by_prefix(
        &self,
        org_id: Uuid,
        prefix: &str,
    ) -> StoreResult<Option<ApiToken>>;

    /// Verify a plaintext token string against stored hashes, updating
    /// `last_used_at` on success. Returns `None` for invalid or expired tokens.
    async fn verify_api_token(&self, token: &str) -> StoreResult<Option<ApiToken>>;

    /// Permanently delete a token.
    async fn delete_api_token(&self, token_id: Uuid) -> StoreResult<()>;

    /// Rotate an existing token — create a replacement with the same scopes
    /// and optionally delete the old one. Returns the new plaintext token,
    /// the new record, and the deleted record (when `delete_old` was `true`).
    async fn rotate_api_token(
        &self,
        token_id: Uuid,
        new_name: Option<&str>,
        delete_old: bool,
    ) -> StoreResult<(String, ApiToken, Option<ApiToken>)>;

    /// Find tokens older than `days_old`, optionally scoped to a single org.
    async fn find_api_tokens_needing_rotation(
        &self,
        org_id: Option<Uuid>,
        days_old: i64,
    ) -> StoreResult<Vec<ApiToken>>;

    // ---------------------------------------------------------------------
    // Organization settings (JSON key/value per org)
    // ---------------------------------------------------------------------

    /// Fetch a single org-level setting by key, returning `None` when absent.
    async fn get_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<Option<OrgSetting>>;

    /// Upsert an org-level setting, returning the persisted record.
    async fn set_org_setting(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> StoreResult<OrgSetting>;

    /// Delete an org-level setting by key. Idempotent.
    async fn delete_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Organizations
    // ---------------------------------------------------------------------

    /// Create a new organization and auto-create the owner membership.
    async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        owner_id: Uuid,
        plan: Plan,
    ) -> StoreResult<Organization>;

    /// Look up an organization by id.
    async fn find_organization_by_id(&self, org_id: Uuid) -> StoreResult<Option<Organization>>;

    /// Look up an organization by slug.
    async fn find_organization_by_slug(&self, slug: &str) -> StoreResult<Option<Organization>>;

    /// List all organizations a user belongs to (as owner or member).
    async fn list_organizations_by_user(&self, user_id: Uuid) -> StoreResult<Vec<Organization>>;

    /// Update an organization's display name.
    async fn update_organization_name(&self, org_id: Uuid, name: &str) -> StoreResult<()>;

    /// Update an organization's slug.
    async fn update_organization_slug(&self, org_id: Uuid, slug: &str) -> StoreResult<()>;

    /// Update an organization's plan (and refresh limits).
    async fn update_organization_plan(&self, org_id: Uuid, plan: Plan) -> StoreResult<()>;

    /// Check whether an organization has an active or trialing subscription.
    async fn organization_has_active_subscription(&self, org_id: Uuid) -> StoreResult<bool>;

    /// Permanently delete an organization (cascades to related rows).
    async fn delete_organization(&self, org_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Organization members
    // ---------------------------------------------------------------------

    /// Add a user to an organization with the given role.
    async fn create_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<OrgMember>;

    /// Look up a specific (org, user) membership.
    async fn find_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<Option<OrgMember>>;

    /// List every member of an organization, oldest first.
    async fn list_org_members(&self, org_id: Uuid) -> StoreResult<Vec<OrgMember>>;

    /// Update a member's role.
    async fn update_org_member_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<()>;

    /// Remove a member from an organization.
    async fn delete_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Audit logs
    // ---------------------------------------------------------------------

    /// Best-effort audit event recording. Failures are logged and swallowed
    /// so they never block the caller's primary operation.
    #[allow(clippy::too_many_arguments)]
    async fn record_audit_event(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event_type: AuditEventType,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    );

    /// List audit logs for an organization with optional filters.
    async fn list_audit_logs(
        &self,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<Vec<AuditLog>>;

    /// Count audit logs matching the filter (for pagination).
    async fn count_audit_logs(
        &self,
        org_id: Uuid,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Feature usage
    // ---------------------------------------------------------------------

    /// Record a feature-usage event. Failures are logged and swallowed.
    async fn record_feature_usage(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        feature: FeatureType,
        metadata: Option<serde_json::Value>,
    );

    /// Count how many times an org used a feature over the last `days` days.
    async fn count_feature_usage_by_org(
        &self,
        org_id: Uuid,
        feature: FeatureType,
        days: i64,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Suspicious activity — record_suspicious_activity is below after Users
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Users
    // ---------------------------------------------------------------------

    /// Create a new user with an already-hashed password.
    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> StoreResult<User>;

    /// Look up a user by id.
    async fn find_user_by_id(&self, user_id: Uuid) -> StoreResult<Option<User>>;

    /// Look up a user by email.
    async fn find_user_by_email(&self, email: &str) -> StoreResult<Option<User>>;

    /// Look up a user by username.
    async fn find_user_by_username(&self, username: &str) -> StoreResult<Option<User>>;

    /// Batch lookup by id to avoid N+1 queries.
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> StoreResult<Vec<User>>;

    /// Set the persistent API token on a user record.
    async fn set_user_api_token(&self, user_id: Uuid, token: &str) -> StoreResult<()>;

    /// Enable TOTP 2FA for a user with the given secret and hashed backup codes.
    async fn enable_user_2fa(
        &self,
        user_id: Uuid,
        secret: &str,
        backup_codes: &[String],
    ) -> StoreResult<()>;

    /// Disable 2FA and clear stored secret + backup codes.
    async fn disable_user_2fa(&self, user_id: Uuid) -> StoreResult<()>;

    /// Refresh the 2FA verified timestamp (e.g. after a successful TOTP challenge).
    async fn update_user_2fa_verified(&self, user_id: Uuid) -> StoreResult<()>;

    /// Remove a consumed backup code by index.
    async fn remove_user_backup_code(&self, user_id: Uuid, code_index: usize) -> StoreResult<()>;

    /// Look up a user by their GitHub account id.
    async fn find_user_by_github_id(&self, github_id: &str) -> StoreResult<Option<User>>;

    /// Look up a user by their Google account id.
    async fn find_user_by_google_id(&self, google_id: &str) -> StoreResult<Option<User>>;

    /// Link an existing user to a GitHub account (sets github_id, auth_provider, avatar_url).
    async fn link_user_github_account(
        &self,
        user_id: Uuid,
        github_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()>;

    /// Link an existing user to a Google account (sets google_id, auth_provider, avatar_url).
    async fn link_user_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()>;

    /// Create a new verified user from an OAuth provider (random password hash).
    #[allow(clippy::too_many_arguments)]
    async fn create_oauth_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        auth_provider: &str,
        github_id: Option<&str>,
        google_id: Option<&str>,
        avatar_url: Option<&str>,
    ) -> StoreResult<User>;

    /// Fetch or create a user's personal/default organization.
    async fn get_or_create_personal_org(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> StoreResult<Organization>;

    /// Replace a user's password hash (no-op on verification).
    async fn update_user_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> StoreResult<()>;

    /// Mark a user's email as verified.
    async fn mark_user_verified(&self, user_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Verification / password-reset tokens
    // ---------------------------------------------------------------------

    /// Create a new verification token for a user (24h default expiry).
    async fn create_verification_token(&self, user_id: Uuid) -> StoreResult<VerificationToken>;

    /// Shorten a verification token's expiry to `hours` from now.
    /// Used by password-reset to override the default 24h window.
    async fn set_verification_token_expiry_hours(
        &self,
        token_id: Uuid,
        hours: i64,
    ) -> StoreResult<()>;

    /// Look up a verification token by its plaintext token string.
    async fn find_verification_token_by_token(
        &self,
        token: &str,
    ) -> StoreResult<Option<VerificationToken>>;

    /// Mark a verification token as consumed.
    async fn mark_verification_token_used(&self, token_id: Uuid) -> StoreResult<()>;

    #[allow(clippy::too_many_arguments)]
    async fn record_suspicious_activity(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        activity_type: SuspiciousActivityType,
        severity: &str,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    );

    // ---------------------------------------------------------------------
    // Federations
    // ---------------------------------------------------------------------

    async fn create_federation(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> StoreResult<Federation>;

    async fn find_federation_by_id(&self, id: Uuid) -> StoreResult<Option<Federation>>;

    async fn list_federations_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Federation>>;

    async fn update_federation(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> StoreResult<Option<Federation>>;

    async fn delete_federation(&self, id: Uuid) -> StoreResult<()>;

    /// List unresolved suspicious activities with optional filters.
    async fn list_unresolved_suspicious_activities(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> StoreResult<Vec<SuspiciousActivity>>;

    /// Count unresolved suspicious activities for an org.
    async fn count_unresolved_suspicious_activities(&self, org_id: Uuid) -> StoreResult<i64>;

    /// Mark a suspicious activity as resolved by the given user.
    async fn resolve_suspicious_activity(
        &self,
        activity_id: Uuid,
        resolved_by: Uuid,
    ) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud workspaces
    // ---------------------------------------------------------------------

    async fn create_cloud_workspace(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
    ) -> StoreResult<CloudWorkspace>;

    async fn find_cloud_workspace_by_id(&self, id: Uuid) -> StoreResult<Option<CloudWorkspace>>;

    async fn list_cloud_workspaces_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudWorkspace>>;

    async fn update_cloud_workspace(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudWorkspace>>;

    async fn delete_cloud_workspace(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud services
    // ---------------------------------------------------------------------

    async fn create_cloud_service(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        base_url: &str,
    ) -> StoreResult<CloudService>;

    async fn find_cloud_service_by_id(&self, id: Uuid) -> StoreResult<Option<CloudService>>;

    async fn list_cloud_services_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudService>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_cloud_service(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        base_url: Option<&str>,
        enabled: Option<bool>,
        tags: Option<&serde_json::Value>,
        routes: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudService>>;

    async fn delete_cloud_service(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud fixtures
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_cloud_fixture(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        path: &str,
        method: &str,
        content: Option<&serde_json::Value>,
    ) -> StoreResult<CloudFixture>;

    async fn find_cloud_fixture_by_id(&self, id: Uuid) -> StoreResult<Option<CloudFixture>>;

    async fn list_cloud_fixtures_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudFixture>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_cloud_fixture(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        path: Option<&str>,
        method: Option<&str>,
        content: Option<&serde_json::Value>,
        tags: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudFixture>>;

    async fn delete_cloud_fixture(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Hosted mocks (deployments)
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_hosted_mock(
        &self,
        org_id: Uuid,
        project_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: Option<&str>,
        config_json: serde_json::Value,
        openapi_spec_url: Option<&str>,
        region: Option<&str>,
    ) -> StoreResult<HostedMock>;

    async fn find_hosted_mock_by_id(&self, id: Uuid) -> StoreResult<Option<HostedMock>>;

    async fn find_hosted_mock_by_slug(
        &self,
        org_id: Uuid,
        slug: &str,
    ) -> StoreResult<Option<HostedMock>>;

    async fn list_hosted_mocks_by_org(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>>;

    async fn update_hosted_mock_status(
        &self,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> StoreResult<()>;

    async fn update_hosted_mock_urls(
        &self,
        id: Uuid,
        deployment_url: Option<&str>,
        internal_url: Option<&str>,
    ) -> StoreResult<()>;

    async fn update_hosted_mock_health(
        &self,
        id: Uuid,
        health_status: HealthStatus,
        health_check_url: Option<&str>,
    ) -> StoreResult<()>;

    async fn delete_hosted_mock(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Waitlist
    // ---------------------------------------------------------------------

    async fn subscribe_waitlist(
        &self,
        email: &str,
        source: &str,
    ) -> StoreResult<WaitlistSubscriber>;

    async fn unsubscribe_waitlist_by_token(&self, token: Uuid) -> StoreResult<bool>;

    // ---------------------------------------------------------------------
    // Usage counters
    // ---------------------------------------------------------------------

    async fn get_or_create_current_usage_counter(&self, org_id: Uuid) -> StoreResult<UsageCounter>;

    async fn list_usage_counters_by_org(&self, org_id: Uuid) -> StoreResult<Vec<UsageCounter>>;

    // ---------------------------------------------------------------------
    // SSO configuration
    // ---------------------------------------------------------------------

    async fn find_sso_config_by_org(&self, org_id: Uuid) -> StoreResult<Option<SSOConfiguration>>;

    #[allow(clippy::too_many_arguments)]
    async fn upsert_sso_config(
        &self,
        org_id: Uuid,
        provider: SSOProvider,
        saml_entity_id: Option<&str>,
        saml_sso_url: Option<&str>,
        saml_slo_url: Option<&str>,
        saml_x509_cert: Option<&str>,
        saml_name_id_format: Option<&str>,
        attribute_mapping: Option<serde_json::Value>,
        require_signed_assertions: bool,
        require_signed_responses: bool,
        allow_unsolicited_responses: bool,
    ) -> StoreResult<SSOConfiguration>;

    async fn enable_sso_config(&self, org_id: Uuid) -> StoreResult<()>;
    async fn disable_sso_config(&self, org_id: Uuid) -> StoreResult<()>;
    async fn delete_sso_config(&self, org_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // SAML replay prevention
    // ---------------------------------------------------------------------

    async fn is_saml_assertion_used(&self, assertion_id: &str, org_id: Uuid) -> StoreResult<bool>;

    #[allow(clippy::too_many_arguments)]
    async fn record_saml_assertion_used(
        &self,
        assertion_id: &str,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name_id: Option<&str>,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> StoreResult<SAMLAssertionId>;

    // ---------------------------------------------------------------------
    // Organization templates
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_org_template(
        &self,
        org_id: Uuid,
        name: &str,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        created_by: Uuid,
        is_default: bool,
    ) -> StoreResult<OrgTemplate>;

    async fn find_org_template_by_id(&self, id: Uuid) -> StoreResult<Option<OrgTemplate>>;

    async fn list_org_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<OrgTemplate>>;

    async fn update_org_template(
        &self,
        template: &OrgTemplate,
        name: Option<&str>,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        is_default: Option<bool>,
    ) -> StoreResult<OrgTemplate>;

    async fn delete_org_template(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Marketplace templates
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_template(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        version: &str,
        category: TemplateCategory,
        content_json: serde_json::Value,
    ) -> StoreResult<Template>;

    async fn find_template_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> StoreResult<Option<Template>>;

    async fn list_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Template>>;

    async fn search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Template>>;

    async fn count_search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Marketplace scenarios
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_scenario(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        current_version: &str,
        category: &str,
        license: &str,
        manifest_json: serde_json::Value,
    ) -> StoreResult<Scenario>;

    async fn find_scenario_by_name(&self, name: &str) -> StoreResult<Option<Scenario>>;

    async fn list_scenarios_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Scenario>>;

    #[allow(clippy::too_many_arguments)]
    async fn search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Scenario>>;

    async fn count_search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Marketplace plugins
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        language: Option<&str>,
        tags: &[String],
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Plugin>>;

    async fn count_search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        language: Option<&str>,
        tags: &[String],
    ) -> StoreResult<i64>;

    async fn find_plugin_by_name(&self, name: &str) -> StoreResult<Option<Plugin>>;

    async fn get_plugin_tags(&self, plugin_id: Uuid) -> StoreResult<Vec<String>>;

    #[allow(clippy::too_many_arguments)]
    async fn create_plugin(
        &self,
        name: &str,
        description: &str,
        version: &str,
        category: &str,
        license: &str,
        repository: Option<&str>,
        homepage: Option<&str>,
        author_id: Uuid,
        language: &str,
    ) -> StoreResult<Plugin>;

    async fn list_plugin_versions(&self, plugin_id: Uuid) -> StoreResult<Vec<PluginVersion>>;

    async fn find_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
    ) -> StoreResult<Option<PluginVersion>>;

    #[allow(clippy::too_many_arguments)]
    async fn create_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
        download_url: &str,
        checksum: &str,
        file_size: i64,
        min_mockforge_version: Option<&str>,
        sbom_json: Option<&serde_json::Value>,
    ) -> StoreResult<PluginVersion>;

    /// Fetch the SBOM (CycloneDX JSON or similar) stored alongside a
    /// plugin version at publish time. Returns `None` when the version
    /// predates SBOM support or the publisher didn't include one.
    async fn get_plugin_version_sbom(
        &self,
        plugin_version_id: Uuid,
    ) -> StoreResult<Option<serde_json::Value>>;

    async fn yank_plugin_version(&self, version_id: Uuid) -> StoreResult<()>;

    async fn get_plugin_version_dependencies(
        &self,
        version_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<String, String>>;

    async fn add_plugin_version_dependency(
        &self,
        version_id: Uuid,
        plugin_name: &str,
        version_req: &str,
    ) -> StoreResult<()>;

    // --- Plugin security scans ---

    /// Upsert a security scan result for a specific plugin version. Each
    /// version has at most one row (latest scan wins).
    async fn upsert_plugin_security_scan(
        &self,
        plugin_version_id: Uuid,
        status: &str,
        score: i16,
        findings: &serde_json::Value,
        scanner_version: Option<&str>,
    ) -> StoreResult<()>;

    /// Fetch the latest security scan for a plugin's current version.
    async fn latest_security_scan_for_plugin(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<Option<PluginSecurityScan>>;

    /// List up to `limit` pending security scans with enough context
    /// (plugin name + version + declared file size) for a worker to
    /// re-download the artifact and run checks.
    async fn list_pending_security_scans(&self, limit: i64) -> StoreResult<Vec<PendingScanJob>>;

    // --- OSV vulnerability cache ---

    /// Find cached OSV advisories that affect this specific
    /// `(ecosystem, name, version)`. Returns an empty vec when the cache
    /// has no hit — the caller should treat that as "clean", not "error".
    async fn find_osv_matches(
        &self,
        ecosystem: &str,
        package_name: &str,
        version: &str,
    ) -> StoreResult<Vec<OsvMatch>>;

    /// Import a single OSV advisory record. Idempotent — the
    /// `(advisory_id, ecosystem, package_name)` uniqueness constraint
    /// means repeat imports just refresh `modified_at` and
    /// `affected_versions`. Returns the number of `(ecosystem, package)`
    /// rows the import landed into (an advisory can cover several
    /// packages; each lands as its own row).
    async fn upsert_osv_advisory(&self, record: &OsvImportRecord) -> StoreResult<usize>;

    /// Count rows in the OSV cache. Used by the scanner to decide whether
    /// to fall back to the hardcoded seed list — an empty cache means the
    /// sync worker hasn't run yet, so a fresh install should still surface
    /// findings against a built-in baseline rather than returning silence.
    async fn count_osv_advisories(&self) -> StoreResult<i64>;

    // --- Publisher attestation keys ---

    /// List every non-revoked public key registered against a user. The
    /// attestation verifier tries each one at publish time.
    async fn list_user_public_keys(&self, user_id: Uuid) -> StoreResult<Vec<UserPublicKey>>;

    /// Register a new public key on a user's account. The caller has
    /// already validated that `public_key_b64` decodes to the right
    /// length for the algorithm. Returns the persisted record so the
    /// handler can echo the id to the client.
    async fn create_user_public_key(
        &self,
        user_id: Uuid,
        algorithm: &str,
        public_key_b64: &str,
        label: &str,
    ) -> StoreResult<UserPublicKey>;

    /// Soft-revoke a key (sets `revoked_at`). Revoked keys are skipped
    /// by the attestation verifier but stay in the table so historical
    /// signatures stay traceable. Returns `true` if a row was actually
    /// updated, so the handler can distinguish "not yours" (or "already
    /// revoked") from "done."
    async fn revoke_user_public_key(&self, user_id: Uuid, key_id: Uuid) -> StoreResult<bool>;

    /// Record a verified SBOM attestation against a plugin version. No-op
    /// when `key_id` is `None` (publisher didn't submit a signature).
    async fn record_plugin_version_attestation(
        &self,
        plugin_version_id: Uuid,
        key_id: Option<Uuid>,
    ) -> StoreResult<()>;

    /// Fetch a stored attestation pointer for the scanner worker. Returns
    /// the verifying key id + signed-at timestamp, or `None` when the
    /// version wasn't signed.
    async fn get_plugin_version_attestation(
        &self,
        plugin_version_id: Uuid,
    ) -> StoreResult<Option<(Uuid, chrono::DateTime<Utc>)>>;

    // --- Plugin reviews ---

    async fn get_plugin_reviews(
        &self,
        plugin_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Review>>;

    async fn count_plugin_reviews(&self, plugin_id: Uuid) -> StoreResult<i64>;

    async fn create_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
        version: &str,
        rating: i16,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<Review>;

    /// Returns (average_rating, total_reviews) for a plugin.
    async fn get_plugin_review_stats(&self, plugin_id: Uuid) -> StoreResult<(f64, i64)>;

    /// Returns map of rating -> count for a plugin.
    async fn get_plugin_review_distribution(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<i16, i64>>;

    async fn find_existing_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<Uuid>>;

    async fn update_plugin_rating_stats(
        &self,
        plugin_id: Uuid,
        avg: f64,
        count: i32,
    ) -> StoreResult<()>;

    async fn increment_plugin_review_vote(
        &self,
        plugin_id: Uuid,
        review_id: Uuid,
        helpful: bool,
    ) -> StoreResult<()>;

    /// Lookup (id, username) for a user.
    async fn get_user_public_info(&self, user_id: Uuid) -> StoreResult<Option<(String, String)>>;

    // --- Template reviews ---

    async fn get_template_reviews(
        &self,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<TemplateReview>>;

    async fn count_template_reviews(&self, template_id: Uuid) -> StoreResult<i64>;

    async fn create_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<TemplateReview>;

    async fn update_template_review_stats(&self, template_id: Uuid) -> StoreResult<()>;

    async fn find_existing_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>>;

    // --- Template stars ---

    /// Toggle a template star for a user.
    /// Returns `(now_starred, new_count)`.
    async fn toggle_template_star(
        &self,
        template_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<(bool, i64)>;

    /// Whether `user_id` has starred `template_id`.
    async fn is_template_starred_by(&self, template_id: Uuid, user_id: Uuid) -> StoreResult<bool>;

    /// Live star count for a single template.
    async fn count_template_stars(&self, template_id: Uuid) -> StoreResult<i64>;

    /// Batch-fetch star counts for many templates in a single query.
    /// Templates with zero stars are absent from the map — callers default to 0.
    async fn count_template_stars_batch(
        &self,
        template_ids: &[Uuid],
    ) -> StoreResult<std::collections::HashMap<Uuid, i64>>;

    // --- Scenario reviews ---

    async fn get_scenario_reviews(
        &self,
        scenario_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<ScenarioReview>>;

    async fn count_scenario_reviews(&self, scenario_id: Uuid) -> StoreResult<i64>;

    async fn create_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<ScenarioReview>;

    async fn update_scenario_review_stats(&self, scenario_id: Uuid) -> StoreResult<()>;

    async fn find_existing_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>>;

    // --- Admin analytics snapshots ---

    /// Fetch a single aggregated snapshot covering every metric surfaced by
    /// the admin analytics dashboard. Encapsulates ~40 raw SQL queries so
    /// handlers stay thin and SQLite implementations can specialize.
    async fn get_admin_analytics_snapshot(&self) -> StoreResult<AdminAnalyticsSnapshot>;

    /// Fetch conversion funnel counts for the given textual Postgres interval
    /// (e.g. "7 days", "30 days"). SQLite implementations may parse this.
    async fn get_conversion_funnel_snapshot(
        &self,
        interval: &str,
    ) -> StoreResult<ConversionFunnelSnapshot>;

    // --- GDPR data export and deletion ---

    async fn list_user_settings_raw(&self, user_id: Uuid) -> StoreResult<Vec<UserSettingRow>>;

    async fn list_user_api_tokens(&self, user_id: Uuid) -> StoreResult<Vec<ApiToken>>;

    async fn get_org_membership_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<String>>;

    async fn list_org_settings_raw(&self, org_id: Uuid) -> StoreResult<Vec<OrgSettingRow>>;

    async fn list_org_projects_raw(&self, org_id: Uuid) -> StoreResult<Vec<ProjectRow>>;

    async fn list_org_subscriptions_raw(&self, org_id: Uuid) -> StoreResult<Vec<SubscriptionRow>>;

    async fn list_org_hosted_mocks_raw(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>>;

    /// Transactionally erase a user's personal data (GDPR right to erasure),
    /// transferring solo-owned orgs with other members to the next admin and
    /// cascade-deleting orgs with no remaining members. Returns the number of
    /// owned organizations affected (for audit logging).
    async fn delete_user_data_cascade(&self, user_id: Uuid) -> StoreResult<usize>;
}

/// Aggregated admin analytics data, corresponding to a single dashboard load.
#[derive(Debug, Clone)]
pub struct AdminAnalyticsSnapshot {
    pub total_users: i64,
    pub verified_users: i64,
    pub auth_providers: Vec<(Option<String>, i64)>,
    pub new_users_7d: i64,
    pub new_users_30d: i64,
    pub total_orgs: i64,
    pub plan_distribution: Vec<(String, i64)>,
    pub active_subs: i64,
    pub trial_orgs: i64,
    pub total_requests: Option<i64>,
    pub total_storage: Option<i64>,
    pub total_ai_tokens: Option<i64>,
    pub top_orgs: Vec<(Uuid, String, String, i64, i64)>,
    pub hosted_mocks_count: i64,
    pub hosted_mocks_orgs: i64,
    pub hosted_mocks_30d: i64,
    pub plugins_count: i64,
    pub plugins_orgs: i64,
    pub plugins_30d: i64,
    pub templates_count: i64,
    pub templates_orgs: i64,
    pub templates_30d: i64,
    pub scenarios_count: i64,
    pub scenarios_orgs: i64,
    pub scenarios_30d: i64,
    pub api_tokens_count: i64,
    pub api_tokens_orgs: i64,
    pub api_tokens_30d: i64,
    pub user_growth_30d: Vec<(chrono::NaiveDate, i64)>,
    pub org_growth_30d: Vec<(chrono::NaiveDate, i64)>,
    pub logins_24h: i64,
    pub logins_7d: i64,
    pub api_requests_24h: i64,
    pub api_requests_7d: i64,
}

/// Aggregated conversion funnel counts for admin dashboards.
#[derive(Debug, Clone)]
pub struct ConversionFunnelSnapshot {
    pub signups: i64,
    pub verified: i64,
    pub logged_in: i64,
    pub org_created: i64,
    pub feature_users: i64,
    pub checkout_initiated: i64,
    pub paid_subscribers: i64,
    pub time_to_convert_days: Option<f64>,
}

/// Raw user_settings row used by GDPR export.
#[derive(Debug, Clone)]
pub struct UserSettingRow {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw org_settings row used by GDPR export.
#[derive(Debug, Clone)]
pub struct OrgSettingRow {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw projects row used by GDPR export.
#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: Uuid,
    pub name: String,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw subscriptions row used by GDPR export.
#[derive(Debug, Clone)]
pub struct SubscriptionRow {
    pub id: Uuid,
    pub plan: String,
    pub status: String,
    pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod osv_matcher_tests {
    use super::{normalize_version_for_ecosystem, version_affected, version_affected_in_ecosystem};
    use serde_json::json;

    #[test]
    fn literal_versions_list_matches() {
        let affected = json!({
            "versions": ["1.2.3", "1.2.4"],
            "ranges": []
        });
        assert!(version_affected(&affected, "1.2.3"));
        assert!(version_affected(&affected, "1.2.4"));
        assert!(!version_affected(&affected, "1.2.5"));
    }

    #[test]
    fn introduced_zero_all_versions() {
        let affected = json!({
            "versions": [],
            "ranges": [{
                "type": "SEMVER",
                "events": [{"introduced": "0"}]
            }]
        });
        assert!(version_affected(&affected, "0.0.1"));
        assert!(version_affected(&affected, "999.0.0"));
    }

    #[test]
    fn introduced_fixed_exclusive_upper() {
        // Advisory says fixed in 1.5.0 — anything from 1.2.0 up to
        // (but not including) 1.5.0 is vulnerable.
        let affected = json!({
            "versions": [],
            "ranges": [{
                "type": "SEMVER",
                "events": [
                    {"introduced": "1.2.0"},
                    {"fixed": "1.5.0"}
                ]
            }]
        });
        assert!(!version_affected(&affected, "1.1.9"));
        assert!(version_affected(&affected, "1.2.0"));
        assert!(version_affected(&affected, "1.3.0"));
        assert!(version_affected(&affected, "1.4.99"));
        assert!(!version_affected(&affected, "1.5.0"), "fixed is exclusive");
        assert!(!version_affected(&affected, "2.0.0"));
    }

    #[test]
    fn last_affected_inclusive_upper() {
        let affected = json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [
                    {"introduced": "1.0.0"},
                    {"last_affected": "1.2.3"}
                ]
            }]
        });
        assert!(version_affected(&affected, "1.0.0"));
        assert!(version_affected(&affected, "1.2.3"), "last_affected is inclusive");
        assert!(!version_affected(&affected, "1.2.4"));
    }

    #[test]
    fn multiple_intervals_per_range() {
        // Real-world shape: a regression introduced in 1.0, fixed in
        // 1.1, reintroduced in 2.0, fixed in 2.1.
        let affected = json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [
                    {"introduced": "1.0.0"},
                    {"fixed": "1.1.0"},
                    {"introduced": "2.0.0"},
                    {"fixed": "2.1.0"}
                ]
            }]
        });
        assert!(version_affected(&affected, "1.0.5"));
        assert!(!version_affected(&affected, "1.5.0"), "between fixes");
        assert!(version_affected(&affected, "2.0.0"));
        assert!(!version_affected(&affected, "2.1.0"));
    }

    #[test]
    fn git_ranges_ignored() {
        let affected = json!({
            "ranges": [{
                "type": "GIT",
                "events": [{"introduced": "0"}]
            }]
        });
        // GIT ranges don't apply to published plugin versions.
        assert!(!version_affected(&affected, "1.2.3"));
    }

    #[test]
    fn go_ecosystem_strips_v_prefix() {
        // Go advisories tag versions as `v1.2.3`. The SEMVER range type
        // assumes plain semver, so without normalization nothing matches.
        let affected = serde_json::json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [{"introduced": "v1.2.0"}, {"fixed": "v1.5.0"}]
            }]
        });
        // Without ecosystem hint → no match (the literal `v1.2.0` fails
        // to parse as semver in the interval walker).
        assert!(!version_affected(&affected, "1.3.0"));
        // With the Go ecosystem hint, both event versions and the target
        // get normalized and match.
        assert!(version_affected_in_ecosystem(&affected, "1.3.0", "Go"));
        assert!(version_affected_in_ecosystem(&affected, "v1.3.0", "Go"));
        assert!(!version_affected_in_ecosystem(&affected, "1.5.0", "Go"), "fixed exclusive");
        assert!(!version_affected_in_ecosystem(&affected, "1.1.9", "Go"));
    }

    #[test]
    fn pypi_prerelease_tags_normalize() {
        // Pre-release markers carry a numeric rank prefix so
        // `dev < a < b < rc` falls out of semver's identifier
        // comparison (numeric-before-alpha, then numeric by value).
        assert_eq!(
            normalize_version_for_ecosystem("PyPI", "1.0.0a1").as_deref(),
            Some("1.0.0-1.a.1")
        );
        assert_eq!(
            normalize_version_for_ecosystem("PyPI", "1.2.3rc2").as_deref(),
            Some("1.2.3-3.rc.2")
        );
        assert_eq!(
            normalize_version_for_ecosystem("PyPI", "2.0.0.dev7").as_deref(),
            Some("2.0.0-0.dev.7")
        );
        // Post-release-alone bumps the patch so semver puts it between
        // the base and the next patch: `3.1.4 < 3.1.5-0.post.1 < 3.1.5`.
        assert_eq!(
            normalize_version_for_ecosystem("PyPI", "3.1.4.post1").as_deref(),
            Some("3.1.5-0.post.1")
        );
    }

    #[test]
    fn pypi_epoch_gets_stripped() {
        assert_eq!(normalize_version_for_ecosystem("PyPI", "2!1.0.0").as_deref(), Some("1.0.0"),);
    }

    /// PEP 440 conformance table. Rows are `(input, expected_output)`
    /// where `None` means "leave literal" (already plain semver or
    /// unrecognized shape). Covers the shapes that actually show up in
    /// PyPI OSV advisories: epoch, release-only, pre-release, post-
    /// release, dev, local version, "v"-prefix, mixed-case alpha
    /// markers, and compound pre+post / pre+dev / post+dev combos.
    #[test]
    fn pypi_conformance_table() {
        let cases: &[(&str, Option<&str>)] = &[
            // Plain semver — no rewrite.
            ("1.2.3", None),
            ("0.0.1", None),
            // Epoch stripping.
            ("1!2.3.4", Some("2.3.4")),
            ("10!0.1.0", Some("0.1.0")),
            // "v" prefix is common in ported Go/npm advisories.
            ("v1.2.3", Some("1.2.3")),
            ("V1.2.3", Some("1.2.3")),
            // Pre-releases, all three markers — each gets a numeric
            // rank prefix so semver's identifier comparison reproduces
            // pep440's `dev < a < b < rc` order.
            ("1.0.0a1", Some("1.0.0-1.a.1")),
            ("1.0.0b2", Some("1.0.0-2.b.2")),
            ("1.2.3rc4", Some("1.2.3-3.rc.4")),
            // Case-insensitive marker matching.
            ("1.0.0RC1", Some("1.0.0-3.rc.1")),
            ("1.0.0A1", Some("1.0.0-1.a.1")),
            // Dev and post releases.
            // `.dev` is a pre-release (earlier than the base version)
            // with rank 0. `.post` bumps the patch so semver sorts it
            // between the base and the next patch — matching pep440
            // (`X.Y.Z < X.Y.Z.postN < X.Y.(Z+1)`).
            ("2.0.0.dev7", Some("2.0.0-0.dev.7")),
            ("3.1.4.post1", Some("3.1.5-0.post.1")),
            // Local identifiers become build metadata.
            ("1.0.0+ubuntu1", Some("1.0.0+local.ubuntu1")),
            ("1.0.0+deb.9", Some("1.0.0+local.deb.9")),
            // Illegal characters in a local id get sanitized to hyphens
            // so the result stays a valid semver build-metadata string.
            ("1.0.0+has_underscore", Some("1.0.0+local.has-underscore")),
            // Compound suffixes: pre-release + local.
            ("1.0.0a1+ubuntu", Some("1.0.0-1.a.1+local.ubuntu")),
            // Pre-release + post. Pre-release slot holds the pre +
            // rank; build slot holds the post.
            ("1.0.0rc1.post2", Some("1.0.0-3.rc.1+post.2")),
            // Pre-release + dev: pre-release slot accumulates both.
            // Documented limitation — this sorts incorrectly vs plain
            // pre (`1.0.0b2.dev3 > 1.0.0b2` in semver, the opposite of
            // pep440), but OSV advisories don't use this combination.
            ("1.0.0b2.dev3", Some("1.0.0-2.b.2.0.dev.3")),
            // Post + dev: patch bumped (as with plain post) and dev
            // appended into the pre-release slot.
            ("1.0.0.post1.dev2", Some("1.0.1-0.post.1.0.dev.2")),
            // Epoch + pre-release + local.
            ("2!1.0.0a1+ubuntu", Some("1.0.0-1.a.1+local.ubuntu")),
            // Words that contain `a`/`b`/`rc` as substrings must NOT
            // trigger the marker path.
            ("1.0.0-alpha", None),
            ("1.2.3-beta", None),
        ];

        for (input, expected) in cases {
            let got = normalize_version_for_ecosystem("PyPI", input);
            assert_eq!(
                got.as_deref(),
                *expected,
                "normalize_version_for_ecosystem(\"PyPI\", {:?}) mismatch — got {:?}, expected {:?}",
                input,
                got,
                expected,
            );
        }
    }

    /// Cross-check our PyPI normalizer against the canonical
    /// `pep440_rs` implementation. For every `(a, b)` pair in the
    /// fixture table, `pep440_rs::Version::cmp(a, b)` must agree with
    /// the ordering our normalizer produces. The point isn't to
    /// replicate every edge case of PEP 440 — it's to make sure our
    /// rewrite preserves the *ordering* invariants the OSV matcher
    /// relies on across the shapes that actually ship in advisories.
    #[test]
    fn pypi_normalizer_agrees_with_pep440_rs_on_ordering() {
        use pep440_rs::Version as PyVer;
        use std::cmp::Ordering;

        // The pairs below exercise the complete pep440 order
        // `dev < a < b < rc < base < post`. Our normalizer now handles
        // this by:
        //
        // * prefixing each pre-release kind with a numeric rank
        //   (`0.dev`, `1.a`, `2.b`, `3.rc`), which semver compares
        //   numerically before alphanumerically — giving the correct
        //   left-to-right order;
        // * bumping the patch for post-releases so
        //   `X.Y.Z.postN` → `X.Y.(Z+1)-0.post.N`, which semver sorts
        //   strictly between `X.Y.Z` and `X.Y.(Z+1)` exactly matching
        //   pep440's definition.
        //
        // Combined forms (pre + post, pre + dev, post + dev) are
        // exercised in the conformance table for structural
        // correctness but intentionally not here; the simplification
        // notes at the top of `normalize_pypi_version` explain why
        // those can drift from pep440 ordering.
        let pairs: &[(&str, &str)] = &[
            // Plain releases.
            ("1.0.0", "1.0.1"),
            ("1.0.0", "1.1.0"),
            ("0.9.9", "1.0.0"),
            // Pre-releases sort before the base.
            ("1.0.0a1", "1.0.0"),
            ("1.0.0b2", "1.0.0rc1"),
            ("1.0.0rc1", "1.0.0"),
            // `dev < a < b < rc` — the reason we added numeric rank prefixes.
            ("1.0.0.dev1", "1.0.0a1"),
            ("1.0.0a1", "1.0.0b1"),
            ("1.0.0b1", "1.0.0rc1"),
            // Dev sorts before the base it pre-releases.
            ("1.0.0.dev1", "1.0.0"),
            // Post sorts after the base but before the next patch.
            ("1.0.0", "1.0.0.post1"),
            ("1.0.0.post1", "1.0.0.post2"),
            ("1.0.0.post1", "1.0.1"),
            // "v" prefix should round-trip equal to the bare version.
            ("v1.0.0", "1.0.1"),
        ];

        // Local-identifier pairs (`1.0.0` vs `1.0.0+ubuntu`) are
        // checked separately because semver *ignores* build metadata
        // for ordering while pep440 sorts the local version higher.
        // OSV advisories never name a local version so this gap is
        // irrelevant for matching, but we sanity-check the public
        // release agrees instead.
        let public_release_pairs: &[(&str, &str)] =
            &[("1.0.0", "1.0.0+ubuntu"), ("1.0.0+deb", "1.0.0+ubuntu")];
        for (a_raw, b_raw) in public_release_pairs {
            let a = a_raw.split('+').next().unwrap();
            let b = b_raw.split('+').next().unwrap();
            let py_a: PyVer = a.parse().unwrap();
            let py_b: PyVer = b.parse().unwrap();
            assert_eq!(
                py_a.cmp(&py_b),
                Ordering::Equal,
                "public releases of {:?} / {:?} should compare equal",
                a_raw,
                b_raw,
            );
        }

        for (a_raw, b_raw) in pairs {
            let py_a: PyVer =
                a_raw.parse().unwrap_or_else(|e| panic!("pep440_rs reject {}: {}", a_raw, e));
            let py_b: PyVer =
                b_raw.parse().unwrap_or_else(|e| panic!("pep440_rs reject {}: {}", b_raw, e));
            let oracle = py_a.cmp(&py_b);

            // Our side: normalize, drop the build metadata so semver's
            // ordering matches pep440_rs's "public release is the
            // sort key for local/post" semantics.
            let norm_a =
                normalize_version_for_ecosystem("PyPI", a_raw).unwrap_or_else(|| a_raw.to_string());
            let norm_b =
                normalize_version_for_ecosystem("PyPI", b_raw).unwrap_or_else(|| b_raw.to_string());
            let strip_meta = |s: &str| s.split('+').next().unwrap_or(s).to_string();
            let (sa, sb) = (strip_meta(&norm_a), strip_meta(&norm_b));
            let ours_a = semver::Version::parse(&sa)
                .unwrap_or_else(|e| panic!("{} → {} not semver: {}", a_raw, sa, e));
            let ours_b = semver::Version::parse(&sb)
                .unwrap_or_else(|e| panic!("{} → {} not semver: {}", b_raw, sb, e));
            let ours = ours_a.cmp(&ours_b);

            // For pairs pep440 considers equal (e.g. `1.0.0` vs
            // `1.0.0+local`), semver strips of build metadata also
            // sorts equal. Otherwise the pair must be strictly less.
            if oracle == Ordering::Equal {
                assert_eq!(
                    ours,
                    Ordering::Equal,
                    "oracle equal but ours not equal for ({:?}, {:?})",
                    a_raw,
                    b_raw,
                );
            } else {
                assert_eq!(
                    ours, oracle,
                    "ordering mismatch for ({:?}, {:?}): pep440_rs {:?} ours {:?} via ({:?}, {:?})",
                    a_raw, b_raw, oracle, ours, sa, sb,
                );
            }
        }
    }

    #[test]
    fn pypi_local_identifiers_are_semver_parsable() {
        // Every rewritten output must be accepted by semver::Version::parse —
        // if it isn't, the downstream matcher silently falls into the
        // "unparsable target → conservative" path and we'd lose the
        // precision the normalizer exists to add.
        let inputs = &[
            "1.0.0+ubuntu1",
            "1.0.0+deb.9",
            "1.0.0+has_underscore",
            "1.0.0a1+ubuntu",
            "2!1.0.0a1+ubuntu",
            "1.0.0rc1.post2",
            "1.0.0b2.dev3",
            "1.0.0.post1.dev2",
        ];
        for input in inputs {
            let normalized = normalize_version_for_ecosystem("PyPI", input)
                .unwrap_or_else(|| panic!("{} did not normalize", input));
            semver::Version::parse(&normalized).unwrap_or_else(|e| {
                panic!("normalized form of {} ({}) failed semver parse: {}", input, normalized, e)
            });
        }
    }

    #[test]
    fn pypi_interval_match_with_prerelease() {
        // CVE says "vulnerable up to 1.0.0 inclusive," including pre-releases.
        let affected = serde_json::json!({
            "ranges": [{
                "type": "ECOSYSTEM",
                "events": [{"introduced": "0"}, {"last_affected": "1.0.0"}]
            }]
        });
        assert!(version_affected_in_ecosystem(&affected, "0.9.0", "PyPI"));
        assert!(version_affected_in_ecosystem(&affected, "1.0.0a1", "PyPI"));
        assert!(version_affected_in_ecosystem(&affected, "1.0.0", "PyPI"));
        assert!(!version_affected_in_ecosystem(&affected, "1.0.1", "PyPI"));
    }

    #[test]
    fn unknown_ecosystem_falls_through_unchanged() {
        assert_eq!(normalize_version_for_ecosystem("RubyGems", "1.2.3"), None);
        assert_eq!(normalize_version_for_ecosystem("", "1.2.3"), None);
        // Behavior must match the pre-existing `version_affected` path
        // exactly so the existing tests (cargo, npm) keep passing.
        let affected = serde_json::json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [{"introduced": "1.0.0"}, {"fixed": "2.0.0"}]
            }]
        });
        assert_eq!(
            version_affected(&affected, "1.5.0"),
            version_affected_in_ecosystem(&affected, "1.5.0", "RubyGems"),
        );
    }

    #[test]
    fn events_are_sorted_before_pairing() {
        // Same advisory as `multiple_intervals_per_range`, but the events
        // are supplied out of order. The matcher must sort them first,
        // otherwise a naive walk would pair "introduced 1.0 → fixed 2.1"
        // (one giant interval) and falsely match 1.5 which is between
        // the two real fixes.
        let affected = serde_json::json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [
                    {"fixed": "2.1.0"},
                    {"introduced": "1.0.0"},
                    {"fixed": "1.1.0"},
                    {"introduced": "2.0.0"}
                ]
            }]
        });
        assert!(version_affected(&affected, "1.0.5"));
        assert!(!version_affected(&affected, "1.5.0"), "sorting pairs (1.0,1.1) not (1.0,2.1)");
        assert!(version_affected(&affected, "2.0.0"));
        assert!(!version_affected(&affected, "2.1.0"));
    }

    #[test]
    fn non_semver_target_conservative_fallback() {
        // Target version isn't valid semver ("2024-01-15"). The matcher
        // must not panic and should only match literal equality or
        // fully-open intervals.
        let literal = json!({
            "versions": ["2024-01-15"]
        });
        assert!(version_affected(&literal, "2024-01-15"));

        let open = json!({
            "ranges": [{"type": "SEMVER", "events": [{"introduced": "0"}]}]
        });
        assert!(version_affected(&open, "2024-01-15"));

        let bounded = json!({
            "ranges": [{
                "type": "SEMVER",
                "events": [{"introduced": "1.0.0"}, {"fixed": "2.0.0"}]
            }]
        });
        // Can't semver-compare "2024-01-15"; stay conservative.
        assert!(!version_affected(&bounded, "2024-01-15"));
    }
}
