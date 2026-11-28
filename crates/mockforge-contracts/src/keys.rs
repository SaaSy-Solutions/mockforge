//! A key by the shape of its value, and not by the name of the header or of
//! the field that holds it.
//!
//! The shapes are the ones providers give their keys: a prefix, a length and
//! a set of characters. They are taken from the table of secret detectors of
//! SemSec (Robusta Studio, MIT, https://github.com/RobustaStudio/semsec at
//! 521800cb), read on 2026-11-26: each pattern was read before it was taken,
//! only a pattern that is the shape of a key was, and the table is copied
//! here once. It is never fetched. A pattern that needed the name of a
//! variable, or a URL, around the key was left out, since a log holds values
//! and not the code that named them.
//!
//! What it is for: a log that is passed around, and a pack written from a
//! recording, do not carry a real key. In an exchange log a key is written
//! as its digest, wherever it is the whole value of a header or of a JSON
//! string. In a pack begun from a recording it is replaced by a made-up key
//! of the same shape.
//!
//! A shape says what a key looks like, and not that a value is one: a value
//! can be of a known shape and be made up, as the keys of the shipped packs
//! are. What a match means is that the value is not to be passed around.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The characters a part of a key is made of. A set that is one symbol wider
/// than a provider's own is not given a name of its own: `-` beside `_`
/// makes nothing match that is not a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alphabet {
    /// `0` to `9`.
    Digits,
    /// `a` to `z`.
    Lowercase,
    /// `a` to `z` and `0` to `9`.
    Small,
    /// `A` to `Z` and `0` to `9`.
    Capital,
    /// `0` to `9` and `a` to `f`.
    HexLower,
    /// `0` to `9`, `a` to `f` and `A` to `F`.
    Hex,
    /// Letters and digits.
    Alphanumeric,
    /// Letters, digits, `-` and `_`.
    Word,
    /// Letters, digits, `-`, `_` and `.`.
    Dotted,
    /// Letters, digits, `-`, `_`, `.`, `+` and `=`.
    Extended,
    /// The 58 characters Bitcoin addresses and Resend tokens use: no `0`,
    /// `O`, `I` or `l`.
    Base58,
    /// Letters, digits, `+`, `/` and `=`.
    Base64,
}

impl Alphabet {
    /// The characters of the set, each set spelled out once.
    fn characters(self) -> &'static [u8] {
        match self {
            Self::Digits => b"0123456789",
            Self::Lowercase => b"abcdefghijklmnopqrstuvwxyz",
            Self::Small => b"0123456789abcdefghijklmnopqrstuvwxyz",
            Self::Capital => b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            Self::HexLower => b"0123456789abcdef",
            Self::Hex => b"0123456789abcdefABCDEF",
            Self::Alphanumeric => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
            Self::Word => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-_",
            Self::Dotted => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-_.",
            Self::Extended => {
                b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-_.+="
            }
            Self::Base58 => b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz",
            Self::Base64 => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+/=",
        }
    }

    fn contains(self, character: char) -> bool {
        u8::try_from(character as u32).is_ok_and(|byte| self.characters().contains(&byte))
    }
}

/// One part of a key's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// These characters, as they are.
    Literal(&'static str),
    /// At least this many and at most that many characters of a set.
    Characters(Alphabet, usize, usize),
}

/// No upper bound on a part's length.
pub const UNBOUNDED: usize = usize::MAX;

/// The shape of one kind of key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyFormat {
    /// The provider that gives keys of this shape.
    pub provider: &'static str,
    /// What the provider calls it.
    pub name: &'static str,
    /// The parts, in order.
    pub parts: &'static [Part],
    /// The SemSec rule the shape was read from.
    pub from: &'static str,
}

const fn key(
    provider: &'static str,
    name: &'static str,
    parts: &'static [Part],
    from: &'static str,
) -> KeyFormat {
    KeyFormat {
        provider,
        name,
        parts,
        from,
    }
}

use Alphabet::{
    Alphanumeric as A, Base58, Base64, Capital, Digits, Dotted, Extended, Hex, HexLower, Lowercase,
    Small, Word,
};
use Part::{Characters as Of, Literal as L};

/// The shapes of keys, each read from the SemSec rule it names, in the
/// order the providers are usually met in. Where two shapes overlap the
/// provider whose shape is the narrower comes first.
pub const FORMATS: &[KeyFormat] = &[
    key(
        "AWS",
        "access key id",
        &[L("AKIA"), Of(Capital, 16, 16)],
        "robusta-secrets-aws-access-key",
    ),
    key(
        "AWS",
        "access key id",
        &[L("ABIA"), Of(Capital, 16, 16)],
        "robusta-secrets-aws-access-key",
    ),
    key(
        "AWS",
        "access key id",
        &[L("ACCA"), Of(Capital, 16, 16)],
        "robusta-secrets-aws-access-key",
    ),
    key(
        "AWS",
        "access key id",
        &[L("ASIA"), Of(Capital, 16, 16)],
        "robusta-secrets-aws-access-key",
    ),
    key(
        "Alibaba Cloud",
        "access key id",
        &[L("LTAI"), Of(A, 17, 21)],
        "robusta-secrets-alibaba-access-key",
    ),
    key(
        "Anthropic",
        "administrative API key",
        &[L("sk-ant-admin01-"), Of(Word, 93, 93), L("AA")],
        "robusta-secrets-anthropic-api-key",
    ),
    key(
        "Anthropic",
        "API key",
        &[L("sk-ant-api03-"), Of(Word, 93, 93), L("AA")],
        "robusta-secrets-anthropic-api-key",
    ),
    key(
        "Bitbucket",
        "app password",
        &[L("ATBB"), Of(Extended, 30, UNBOUNDED)],
        "robusta-secrets-bitbucket-app-password",
    ),
    key(
        "Buildkite",
        "token",
        &[L("bkua_"), Of(Small, 40, 40)],
        "robusta-secrets-buildkite-token",
    ),
    key(
        "CircleCI",
        "token",
        &[L("CCIPAT_"), Of(A, 22, 22), L("_"), Of(Hex, 40, 40)],
        "robusta-secrets-circleci-token",
    ),
    key(
        "Cloudflare",
        "API token",
        &[L("cfut_"), Of(A, 40, 40), Of(HexLower, 8, 8)],
        "robusta-secrets-cloudflare-api-token",
    ),
    key(
        "Cloudflare",
        "API token",
        &[L("cfat_"), Of(A, 40, 40), Of(HexLower, 8, 8)],
        "robusta-secrets-cloudflare-api-token",
    ),
    key(
        "Contentful",
        "personal access token",
        &[L("CFPAT-"), Of(Word, 43, 43)],
        "robusta-secrets-contentful-pat",
    ),
    key(
        "Databricks",
        "token",
        &[L("dapi"), Of(HexLower, 32, 32)],
        "robusta-secrets-databricks-token",
    ),
    key(
        "Deno",
        "deploy token",
        &[L("ddp_"), Of(A, 36, 36)],
        "robusta-secrets-deno-deploy-token",
    ),
    key(
        "Deno",
        "deploy token",
        &[L("ddw_"), Of(A, 36, 36)],
        "robusta-secrets-deno-deploy-token",
    ),
    key(
        "DigitalOcean",
        "personal access token",
        &[L("dop_v1_"), Of(HexLower, 64, 64)],
        "robusta-secrets-digitalocean-token",
    ),
    key(
        "DigitalOcean",
        "personal access token",
        &[L("doo_v1_"), Of(HexLower, 64, 64)],
        "robusta-secrets-digitalocean-token",
    ),
    key(
        "DigitalOcean",
        "personal access token",
        &[L("dor_v1_"), Of(HexLower, 64, 64)],
        "robusta-secrets-digitalocean-token",
    ),
    key(
        "Docker Hub",
        "personal access token",
        &[L("dckr_pat_"), Of(Word, 27, 27)],
        "robusta-secrets-dockerhub-pat",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.audit."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.ct."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.pt."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.sa."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.scim."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Doppler",
        "token",
        &[L("dp.st."), Of(A, 40, 44)],
        "robusta-secrets-doppler-token",
    ),
    key(
        "Dropbox",
        "token",
        &[L("sl.u."), Of(Word, 130, UNBOUNDED)],
        "robusta-secrets-dropbox-token",
    ),
    key(
        "Fireworks",
        "API key",
        &[L("fw_"), Of(A, 20, 40)],
        "robusta-secrets-fireworks-api-key",
    ),
    key(
        "Flutterwave",
        "secret key",
        &[L("FLWSECK-"), Of(Small, 32, 32), L("-X")],
        "robusta-secrets-flutterwave-secret-key",
    ),
    key(
        "Google",
        "API key",
        &[L("AIza"), Of(Word, 35, 35)],
        "robusta-secrets-gcp-api-key",
    ),
    key(
        "GitHub",
        "fine-grained personal access token",
        &[L("github_pat_"), Of(Word, 82, UNBOUNDED)],
        "robusta-secrets-github-fine-grained-pat",
    ),
    key(
        "GitHub",
        "OAuth token",
        &[L("gho_"), Of(Word, 36, 255)],
        "robusta-secrets-github-pat",
    ),
    key(
        "GitHub",
        "personal access token",
        &[L("ghp_"), Of(Word, 36, 255)],
        "robusta-secrets-github-pat",
    ),
    key(
        "GitHub",
        "refresh token",
        &[L("ghr_"), Of(Word, 36, 255)],
        "robusta-secrets-github-pat",
    ),
    key(
        "GitHub",
        "server-to-server token",
        &[L("ghs_"), Of(Word, 36, 255)],
        "robusta-secrets-github-pat",
    ),
    key(
        "GitHub",
        "user access token",
        &[L("ghu_"), Of(Word, 36, 255)],
        "robusta-secrets-github-pat",
    ),
    key(
        "GitLab",
        "organization access token",
        &[L("gloas-"), Of(HexLower, 64, 64)],
        "robusta-secrets-gitlab-oauth2",
    ),
    key(
        "GitLab",
        "personal access token",
        &[L("glpat-"), Of(Word, 20, UNBOUNDED)],
        "robusta-secrets-gitlab-token",
    ),
    key(
        "GoCardless",
        "access token",
        &[L("live_"), Of(Extended, 40, 40)],
        "robusta-secrets-gocardless-token",
    ),
    key(
        "Grafana",
        "cloud API token",
        &[L("glc_eyJ"), Of(Base64, 60, 160)],
        "robusta-secrets-grafana-cloud-token",
    ),
    key(
        "Grafana",
        "service account token",
        &[L("glsa_"), Of(Word, 41, 41)],
        "robusta-secrets-grafana-service-account",
    ),
    key("Groq", "API key", &[L("gsk_"), Of(A, 52, 52)], "robusta-secrets-groq-api-key"),
    key(
        "Harness",
        "personal access token",
        &[
            L("pat."),
            Of(Word, 22, 22),
            L("."),
            Of(HexLower, 24, 24),
            L("."),
            Of(A, 20, 20),
        ],
        "robusta-secrets-harness-pat",
    ),
    key(
        "HashiCorp",
        "Vault token",
        &[L("hvs."), Of(A, 24, UNBOUNDED)],
        "robusta-secrets-vault-token",
    ),
    key(
        "Hugging Face",
        "access token",
        &[L("hf_"), Of(A, 34, 34)],
        "robusta-secrets-huggingface-token",
    ),
    key(
        "Infisical",
        "service token",
        &[
            L("st."),
            Of(HexLower, 8, 8),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 12, 12),
            L("."),
            Of(HexLower, 32, 32),
        ],
        "robusta-secrets-infisical-token",
    ),
    key(
        "LangSmith",
        "API key",
        &[
            L("lsv2_pt_"),
            Of(HexLower, 32, 32),
            L("_"),
            Of(HexLower, 10, 10),
        ],
        "robusta-secrets-langsmith-api-key",
    ),
    key(
        "LangSmith",
        "API key",
        &[
            L("lsv2_sk_"),
            Of(HexLower, 32, 32),
            L("_"),
            Of(HexLower, 10, 10),
        ],
        "robusta-secrets-langsmith-api-key",
    ),
    key(
        "LaunchDarkly",
        "API key",
        &[
            L("api-"),
            Of(Small, 8, 8),
            L("-"),
            Of(Small, 4, 4),
            L("-4"),
            Of(Small, 3, 3),
            L("-"),
            Of(Small, 4, 4),
            L("-"),
            Of(Small, 12, 12),
        ],
        "robusta-secrets-launchdarkly-key",
    ),
    key(
        "LaunchDarkly",
        "SDK key",
        &[
            L("sdk-"),
            Of(Small, 8, 8),
            L("-"),
            Of(Small, 4, 4),
            L("-4"),
            Of(Small, 3, 3),
            L("-"),
            Of(Small, 4, 4),
            L("-"),
            Of(Small, 12, 12),
        ],
        "robusta-secrets-launchdarkly-key",
    ),
    key(
        "Linear",
        "API key",
        &[L("lin_api_"), Of(A, 40, 40)],
        "robusta-secrets-linear-api-key",
    ),
    key(
        "Mailgun",
        "API key",
        &[L("key-"), Of(A, 32, 32)],
        "robusta-secrets-mailgun-api-key",
    ),
    key(
        "Mapbox",
        "secret token",
        &[L("sk."), Of(Dotted, 80, 240)],
        "robusta-secrets-mapbox-secret-key",
    ),
    key(
        "NVIDIA",
        "API key",
        &[L("nvapi-"), Of(Word, 64, 64)],
        "robusta-secrets-nvidia-api-key",
    ),
    key(
        "New Relic",
        "user API key",
        &[L("NRAK-"), Of(Capital, 27, 27)],
        "robusta-secrets-newrelic-user-key",
    ),
    key(
        "Nightfall",
        "API key",
        &[L("NF-"), Of(A, 32, 32)],
        "robusta-secrets-nightfall-api-key",
    ),
    key(
        "Notion",
        "integration secret",
        &[L("secret_"), Of(A, 43, 43)],
        "robusta-secrets-notion-token",
    ),
    key(
        "NuGet",
        "API key",
        &[L("oy2"), Of(Small, 43, 43)],
        "robusta-secrets-nuget-api-key",
    ),
    key(
        "OpenAI",
        "API key",
        &[L("sk-"), Of(A, 20, 20), L("T3BlbkFJ"), Of(A, 20, 20)],
        "robusta-secrets-openai-api-key",
    ),
    key(
        "OpenAI",
        "project API key",
        &[L("sk-proj-"), Of(Word, 80, UNBOUNDED)],
        "robusta-secrets-openai-project-key",
    ),
    key(
        "OpenRouter",
        "API key",
        &[L("sk-or-v1-"), Of(HexLower, 64, 64)],
        "robusta-secrets-openrouter-api-key",
    ),
    key(
        "Stripe",
        "restricted key",
        &[L("rk_live_"), Of(A, 24, UNBOUNDED)],
        "robusta-secrets-stripe-restricted-key",
    ),
    key(
        "Stripe",
        "secret key",
        &[L("sk_live_"), Of(A, 24, UNBOUNDED)],
        "robusta-secrets-stripe-secret-key",
    ),
    key(
        "Paystack",
        "secret key",
        &[L("sk_"), Of(Lowercase, 1, 20), L("_"), Of(A, 40, 40)],
        "robusta-secrets-paystack-secret-key",
    ),
    key(
        "Pinecone",
        "API key",
        &[L("pcsk_"), Of(A, 5, 6), L("_"), Of(A, 63, 63)],
        "robusta-secrets-pinecone-api-key",
    ),
    key(
        "Plaid",
        "production access token",
        &[
            L("access-production-"),
            Of(HexLower, 8, 8),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 12, 12),
        ],
        "robusta-secrets-plaid-access-token",
    ),
    key(
        "Plaid",
        "sandbox access token",
        &[
            L("access-sandbox-"),
            Of(HexLower, 8, 8),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 4, 4),
            L("-"),
            Of(HexLower, 12, 12),
        ],
        "robusta-secrets-plaid-access-token",
    ),
    key(
        "PlanetScale",
        "password",
        &[L("pscale_pw_"), Of(Word, 43, 43)],
        "robusta-secrets-planetscale-password",
    ),
    key(
        "PlanetScale",
        "service token",
        &[L("pscale_tkn_"), Of(Word, 43, 43)],
        "robusta-secrets-planetscale-token",
    ),
    key(
        "PostHog",
        "personal API key",
        &[L("phx_"), Of(Word, 43, 48)],
        "robusta-secrets-posthog-api-key",
    ),
    key(
        "Postman",
        "API key",
        &[L("PMAK-"), Of(Word, 59, 59)],
        "robusta-secrets-postman-api-key",
    ),
    key(
        "Prefect",
        "API key",
        &[L("pnu_"), Of(A, 36, 36)],
        "robusta-secrets-prefect-api-key",
    ),
    key(
        "Pulumi",
        "access token",
        &[L("pul-"), Of(Small, 40, 40)],
        "robusta-secrets-pulumi-token",
    ),
    key(
        "PyPI",
        "token",
        &[L("pypi-"), Of(Word, 50, UNBOUNDED)],
        "robusta-secrets-pypi-token",
    ),
    key(
        "Razorpay",
        "secret key",
        &[L("rzp_live_"), Of(A, 14, 14)],
        "robusta-secrets-razorpay-key",
    ),
    key(
        "Replicate",
        "API token",
        &[L("r8_"), Of(Word, 37, 37)],
        "robusta-secrets-replicate-api-key",
    ),
    key(
        "Resend",
        "API key",
        &[L("re_"), Of(Base58, 8, 8), L("_"), Of(Base58, 24, 24)],
        "robusta-secrets-resend-api-key",
    ),
    key(
        "RubyGems",
        "API key",
        &[L("rubygems_"), Of(A, 48, 48)],
        "robusta-secrets-rubygems-api-key",
    ),
    key(
        "Salesforce",
        "session token",
        &[L("00"), Of(A, 13, 13), L("!"), Of(Extended, 96, 96)],
        "robusta-secrets-salesforce-token",
    ),
    key(
        "SendGrid",
        "API key",
        &[
            L("SG."),
            Of(Word, 22, UNBOUNDED),
            L("."),
            Of(Word, 22, UNBOUNDED),
        ],
        "robusta-secrets-sendgrid-api-key",
    ),
    key(
        "Sentry",
        "authentication token",
        &[L("sntrys_eyJ"), Of(Base64, 197, 197)],
        "robusta-secrets-sentry-org-token",
    ),
    key(
        "Shopify",
        "access token",
        &[L("shpat_"), Of(Hex, 32, 32)],
        "robusta-secrets-shopify-token",
    ),
    key(
        "Slack",
        "legacy workspace token",
        &[
            L("xoxa-"),
            Of(Digits, 10, 13),
            L("-"),
            Of(Word, 20, UNBOUNDED),
        ],
        "robusta-secrets-slack-token",
    ),
    key(
        "Slack",
        "bot token",
        &[
            L("xoxb-"),
            Of(Digits, 10, 13),
            L("-"),
            Of(Word, 20, UNBOUNDED),
        ],
        "robusta-secrets-slack-token",
    ),
    key(
        "Slack",
        "workspace token",
        &[
            L("xoxp-"),
            Of(Digits, 10, 13),
            L("-"),
            Of(Word, 20, UNBOUNDED),
        ],
        "robusta-secrets-slack-token",
    ),
    key(
        "Slack",
        "refresh token",
        &[
            L("xoxr-"),
            Of(Digits, 10, 13),
            L("-"),
            Of(Word, 20, UNBOUNDED),
        ],
        "robusta-secrets-slack-token",
    ),
    key(
        "Slack",
        "workspace browser token",
        &[
            L("xoxs-"),
            Of(Digits, 10, 13),
            L("-"),
            Of(Word, 20, UNBOUNDED),
        ],
        "robusta-secrets-slack-token",
    ),
    key(
        "Sourcegraph",
        "access token",
        &[L("slk_"), Of(HexLower, 64, 64)],
        "robusta-secrets-sourcegraph-cody",
    ),
    key(
        "Sourcegraph",
        "session token",
        &[L("sgp_"), Of(Hex, 16, 16), L("_"), Of(Hex, 40, 40)],
        "robusta-secrets-sourcegraph-token",
    ),
    key(
        "Square",
        "access token",
        &[L("EAAA"), Of(Extended, 60, 60)],
        "robusta-secrets-square-access-token",
    ),
    key(
        "Square",
        "OAuth application secret",
        &[L("sq0c"), Of(Lowercase, 2, 2), L("-"), Of(Word, 40, 50)],
        "robusta-secrets-square-app-secret",
    ),
    key(
        "Supabase",
        "access token",
        &[L("sbp_"), Of(Small, 40, 40)],
        "robusta-secrets-supabase-token",
    ),
    key(
        "Tailscale",
        "API key",
        &[
            L("tskey-"),
            Of(Lowercase, 1, UNBOUNDED),
            L("-"),
            Of(Word, 1, UNBOUNDED),
            L("-"),
            Of(Word, 1, UNBOUNDED),
        ],
        "robusta-secrets-tailscale-key",
    ),
    key(
        "Telegram",
        "bot token",
        &[Of(Digits, 8, 10), L(":"), Of(Word, 35, 35)],
        "robusta-secrets-telegram-bot-token",
    ),
    key(
        "Terraform",
        "team API token",
        &[Of(A, 14, 14), L(".atlasv1."), Of(A, 67, 67)],
        "robusta-secrets-terraform-cloud-token",
    ),
    key(
        "Twilio",
        "API key",
        &[L("SK"), Of(Hex, 32, 32)],
        "robusta-secrets-twilio-api-key",
    ),
    key(
        "npm",
        "access token",
        &[L("npm_"), Of(A, 36, UNBOUNDED)],
        "robusta-secrets-npm-token",
    ),
    key("xAI", "API key", &[L("xai-"), Of(Word, 80, 80)], "robusta-secrets-xai-api-key"),
];

impl KeyFormat {
    /// Whether the whole of `text` is a key of this shape.
    pub fn matches(&self, text: &str) -> bool {
        fits(self.parts, text)
    }

    /// A key of this shape, made up from `seed`: the same seed gives the same
    /// key, so that a pack written twice from one recording is the same pack.
    /// Where a part may be of several lengths, the seed picks one.
    pub fn made_up(&self, seed: u64) -> String {
        let mut random = Seed(seed);
        let mut made = String::new();
        for part in self.parts {
            match *part {
                Part::Literal(literal) => made.push_str(literal),
                Part::Characters(alphabet, least, most) => {
                    let characters = alphabet.characters();
                    let span = most.saturating_sub(least).saturating_add(1);
                    let length = least + (random.next() as usize) % span.min(64);
                    for _ in 0..length {
                        let picked = (random.next() as usize) % characters.len();
                        made.push(char::from(characters[picked]));
                    }
                }
            }
        }
        made
    }
}

/// Whether the parts, in order, take up the whole of `text`. A part that may
/// be of several lengths is tried from its longest, and a shorter one when
/// what follows does not fit.
fn fits(parts: &[Part], text: &str) -> bool {
    let Some((first, rest)) = parts.split_first() else {
        return text.is_empty();
    };
    match *first {
        Part::Literal(literal) => text.strip_prefix(literal).is_some_and(|after| fits(rest, after)),
        Part::Characters(alphabet, least, most) => {
            let taken = text
                .chars()
                .take(least.max(most.min(text.len())))
                .take_while(|character| alphabet.contains(*character))
                .count();
            (least..=taken.min(most)).rev().any(|length| fits(rest, &text[length..]))
        }
    }
}

/// A small generator of numbers from a seed: xorshift64.
struct Seed(u64);

impl Seed {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// The shape of the key `text` is, when the whole of it is a key of one.
pub fn format_of(text: &str) -> Option<&'static KeyFormat> {
    FORMATS.iter().find(|format| format.matches(text))
}

/// Whether the whole of `text` is a key of a known shape.
pub fn is_key(text: &str) -> bool {
    format_of(text).is_some()
}

/// A key of the shape of `text`, made up from what `text` is: the same key
/// gives the same replacement, and nothing of the key is kept.
pub fn made_up_like(text: &str) -> Option<String> {
    let format = format_of(text)?;
    let seed = u64::from_be_bytes(Sha256::digest(text.as_bytes())[..8].try_into().ok()?);
    Some(format.made_up(seed))
}

/// What was replaced by made-up keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Replaced {
    /// How many keys were.
    pub count: usize,
    /// The providers whose keys they were, each once, in alphabetical order.
    pub providers: Vec<&'static str>,
}

/// `value` with every string in it that is a key of a known shape replaced by
/// a made-up key of the same shape, and what was replaced. A string that is
/// the name of a field is not looked at: only what the API answered.
pub fn made_up_in(value: &mut Value) -> Replaced {
    let mut replaced = Replaced::default();
    replace_in(value, &mut replaced);
    replaced.providers.sort_unstable();
    replaced.providers.dedup();
    replaced
}

fn replace_in(value: &mut Value, replaced: &mut Replaced) {
    match value {
        Value::String(text) => {
            if let Some(made) = made_up_like(text) {
                if let Some(format) = format_of(text) {
                    replaced.providers.push(format.provider);
                }
                *text = made;
                replaced.count += 1;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| replace_in(item, replaced)),
        Value::Object(fields) => fields.values_mut().for_each(|field| replace_in(field, replaced)),
        _ => {}
    }
}

/// The characters, beside letters and digits, that a key can be made of. A
/// colon is among them for the one provider that puts one in its key.
const SIGNS: [char; 6] = ['_', '.', '+', '-', '/', ':'];

fn is_key_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || SIGNS.contains(&character)
}

/// What an escape of JSON text means for a run of a key's characters.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Escape {
    /// A character a key is made of, as `\/` or `\uXXXX` writes it: the run
    /// goes on.
    Character(char),
    /// An escape a key cannot hold, taken from the text: the run ends before
    /// it, and the character after the backslash starts nothing.
    Ends(char),
    /// Not an escape of JSON text: the backslash is an ordinary character,
    /// which ends the run, and what follows it is read again.
    None,
}

/// What the escape that follows a backslash in `rest` means for a run.
fn escaped(rest: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Escape {
    match rest.peek().copied() {
        Some('/') => {
            rest.next();
            Escape::Character('/')
        }
        Some('u') => {
            let mut look = rest.clone();
            look.next();
            let mut code = 0u32;
            for _ in 0..4 {
                let Some(digit) = look.next().and_then(|character| character.to_digit(16)) else {
                    return Escape::None;
                };
                code = code * 16 + digit;
            }
            match char::from_u32(code).filter(|named| is_key_character(*named)) {
                Some(named) => {
                    *rest = look;
                    Escape::Character(named)
                }
                None => Escape::None,
            }
        }
        Some(simple @ ('"' | '\\' | 'b' | 'f' | 'n' | 'r' | 't')) => {
            rest.next();
            Escape::Ends(simple)
        }
        _ => Escape::None,
    }
}

/// End a run: what `instead` gives for it takes its place when it is a key.
/// A run that ends with full stops is tried without them too, and keeps the
/// stops, so that a key at the end of a sentence is found.
fn keep_run(run: &mut String, out: &mut String, instead: &impl Fn(&str) -> Option<String>) {
    if run.is_empty() {
        return;
    }
    let taken = instead(run).or_else(|| {
        let bare = run.trim_end_matches('.');
        if bare.len() == run.len() {
            return None;
        }
        let mut with_stops = instead(bare)?;
        with_stops.push_str(&run[bare.len()..]);
        Some(with_stops)
    });
    match taken {
        Some(other) => out.push_str(&other),
        None => out.push_str(run),
    }
    run.clear();
}

/// The text with every run of the characters a key is made of that is a key
/// of a known shape replaced by what `instead` gives for it, wherever the run
/// is: the whole of a header's value or the key after `Bearer`, a parameter
/// of a query string, a string of a body of any kind. In JSON text `\/` and
/// `\uXXXX` count as the characters they name, so that a key with an escaped
/// character is found too. Everything that is not a key is left as it is.
pub fn replace_runs(text: &str, instead: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    let mut rest = text.chars().peekable();
    while let Some(character) = rest.next() {
        let held = if character == '\\' {
            escaped(&mut rest)
        } else {
            Escape::Character(character)
        };
        match held {
            Escape::Character(held) if is_key_character(held) => run.push(held),
            Escape::Character(other) => {
                keep_run(&mut run, &mut out, &instead);
                out.push(other);
            }
            Escape::Ends(after) => {
                keep_run(&mut run, &mut out, &instead);
                out.push('\\');
                out.push(after);
            }
            Escape::None => {
                keep_run(&mut run, &mut out, &instead);
                out.push(character);
            }
        }
    }
    keep_run(&mut run, &mut out, &instead);
    out
}

/// The text with every whole JSON string of a known shape's key replaced by
/// what `instead` gives for it. A string that is not whole, because the body
/// was cut inside it, is left as it is, and so is all the rest of the text.
pub fn replace_json_strings(text: &str, instead: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let body = &from[1..];
        let mut end = None;
        let mut escaped = false;
        for (index, character) in body.char_indices() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                end = Some(index);
                break;
            }
        }
        let Some(end) = end else {
            // The string does not end inside the text.
            out.push_str(from);
            return out;
        };
        let (held, after) = body.split_at(end);
        if !held.contains('\\') {
            match instead(held) {
                Some(other) => {
                    out.push('"');
                    out.push_str(&other);
                    out.push('"');
                }
                None => {
                    out.push('"');
                    out.push_str(held);
                    out.push('"');
                }
            }
        } else {
            out.push('"');
            out.push_str(held);
            out.push('"');
        }
        // Past the string's closing quote.
        rest = &after[1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn made(format: &KeyFormat) -> String {
        format.made_up(7)
    }

    /// A character of the set, to make a key one character longer.
    fn alphabet_characters(alphabet: Alphabet) -> char {
        char::from(alphabet.characters()[0])
    }

    #[test]
    fn every_shape_fits_a_key_made_up_from_it_and_nothing_shorter() {
        assert!(FORMATS.len() >= 60);
        for format in FORMATS {
            let key = made(format);
            assert!(format.matches(&key), "{} {} {key}", format.provider, format.name);
            assert_eq!(format_of(&key), Some(format), "{key}");
            // Where the last part is of one length, one character less, or
            // one more, is not the key.
            if let [.., Part::Characters(alphabet, least, most)] = format.parts {
                if least == most {
                    assert!(!format.matches(&key[..key.len() - 1]), "{key}");
                    let more = format!("{key}{}", alphabet_characters(*alphabet));
                    assert!(!format.matches(&more), "{more}");
                }
            }
        }
    }

    #[test]
    fn a_key_is_known_by_its_shape_and_its_maker() {
        let key = format_of("sk_live_4f2a9c81e7b3d6a04f2a9c81").unwrap();
        assert_eq!((key.provider, key.name), ("Stripe", "secret key"));
        let key = format_of("AKIAIOSFODNN7EXAMPLE").unwrap();
        assert_eq!((key.provider, key.name), ("AWS", "access key id"));
        let key = format_of("xoxb-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx").unwrap();
        assert_eq!(key.provider, "Slack");
        let key = format_of("SG.abcdefghijklmnopqrstuv.ABCDEFGHIJKLMNOPQRSTUVWXYZ").unwrap();
        assert_eq!(key.provider, "SendGrid");
        assert_eq!(
            format_of("ghp_0123456789012345678901234567890123456789").unwrap().provider,
            "GitHub"
        );
        assert_eq!(
            format_of("1098765432:AAHdqTcvbXHJKl-MnbxCvbnAsdfghjklpoi").unwrap().provider,
            "Telegram"
        );
    }

    #[test]
    fn what_is_not_a_key_is_not_taken_for_one() {
        for text in [
            "",
            "Bearer demo-user-a",
            "sha256:4f2a9c81e7b3d6a04f2a9c81e7b3d6a0",
            "demo-user-a",
            "u_1007",
            "acc_9",
            "5000",
            "paid",
            "application/json",
            "http://127.0.0.1:8080/webhooks/github",
            "/webhooks/github",
            "sha256=757107ea0eb2509fc211fa21ddde4799ea369409d6b13608b9b4134b9d23cd0d8",
            "sk_live_x",
            "sk-ant-api03-",
            "2026-11-25T18:02:11Z",
            "whsec_5f8e2c91d7a4",
            "3f8e2c91d7a4b603ac1f9d2e7c4b8a10",
            "a very much longer string that is not a key at all, whatever its length",
            "AKIAIOSFODNN7EXAMPLES",
            "sk_live_4f2a9c81e7b3d6a04f2a9c81 extra",
            " key sk_live_4f2a9c81e7b3d6a04f2a9c81",
        ] {
            assert!(!is_key(text), "{text}");
        }
    }

    #[test]
    fn a_made_up_key_keeps_the_shape_and_loses_the_key() {
        let key = "ghp_0123456789012345678901234567890123456789";
        let made = made_up_like(key).unwrap();
        assert_eq!(format_of(&made).unwrap().provider, "GitHub");
        assert_ne!(made, key);
        // The same key gives the same replacement, and another key another.
        assert_eq!(made_up_like(key).unwrap(), made);
        assert_ne!(made_up_like("ghp_9876543210987654321098765432109876543210").unwrap(), made);
        assert_eq!(made_up_like("nothing"), None);
    }

    #[test]
    fn json_strings_of_a_known_shape_are_replaced_and_the_rest_is_kept() {
        let body = r#"{"api_key": "sk_live_4f2a9c81e7b3d6a04f2a9c81", "id": 5000, "note": "a \"quoted\" sk_live_4f2a9c81e7b3d6a04f2a9c81", "token": "ghp_0123456789012345678901234567890123456789"}"#;
        let out = replace_json_strings(body, |held| is_key(held).then(|| digest_of(held)));
        assert_eq!(
            out,
            format!(
                "{{\"api_key\": \"{}\", \"id\": 5000, \"note\": \"a \\\"quoted\\\" sk_live_4f2a9c81e7b3d6a04f2a9c81\", \"token\": \"{}\"}}",
                digest_of("sk_live_4f2a9c81e7b3d6a04f2a9c81"),
                digest_of("ghp_0123456789012345678901234567890123456789")
            )
        );
        // Text that is not JSON, or a string cut before its end, is left as it is.
        for text in [
            "not json at all \"sk_live_4f2a9c81e7b3d6a04f2a9c81",
            "{\"api_key\": \"sk_live_4f2a9c81e7b3d6a04f2a9c81",
            "plain text with no quotes",
            "",
        ] {
            assert_eq!(
                replace_json_strings(text, |held| is_key(held).then(|| digest_of(held))),
                text,
                "{text}"
            );
        }
    }

    #[test]
    fn the_keys_of_a_recording_are_made_up_and_counted() {
        let mut answer = serde_json::json!({
            "id": 5000,
            "api_key": "sk_live_4f2a9c81e7b3d6a04f2a9c81",
            "keys": ["AKIAIOSFODNN7EXAMPLE", "v1"],
            "nested": { "token": "ghp_0123456789012345678901234567890123456789", "ghp_0123456789012345678901234567890123456789": "a name of a field" },
            "note": "see sk_live_4f2a9c81e7b3d6a04f2a9c81",
        });
        let replaced = made_up_in(&mut answer);
        assert_eq!(replaced.count, 3);
        assert_eq!(replaced.providers, ["AWS", "GitHub", "Stripe"]);
        // Each is a key of the same shape, and none is the key it replaces.
        assert_eq!(format_of(answer["api_key"].as_str().unwrap()).unwrap().provider, "Stripe");
        assert_ne!(answer["api_key"], "sk_live_4f2a9c81e7b3d6a04f2a9c81");
        assert_eq!(format_of(answer["keys"][0].as_str().unwrap()).unwrap().provider, "AWS");
        assert_eq!(answer["keys"][1], "v1");
        assert_eq!(
            format_of(answer["nested"]["token"].as_str().unwrap()).unwrap().provider,
            "GitHub"
        );
        assert_eq!(
            answer["nested"]["ghp_0123456789012345678901234567890123456789"],
            "a name of a field"
        );
        // What was not a whole key is as it was.
        assert_eq!(answer["note"], "see sk_live_4f2a9c81e7b3d6a04f2a9c81");
        assert_eq!(answer["id"], 5000);
    }

    fn digest_of(text: &str) -> String {
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    /// A key of Grafana's cloud shape, with a slash in it. It is made here
    /// and not written whole, so that the tree holds no example it does not
    /// name.
    fn grafana_key() -> String {
        format!("glc_eyJ{}/{}/{}", "a".repeat(20), "b".repeat(20), "c".repeat(20))
    }

    #[test]
    fn a_key_is_found_wherever_a_run_of_its_characters_is_one() {
        let stripe = "sk_live_4f2a9c81e7b3d6a04f2a9c81";
        let github = "ghp_0123456789012345678901234567890123456789";
        let made_stripe = made_up_like(stripe).unwrap();
        let made_github = made_up_like(github).unwrap();
        let text = format!("Bearer {stripe}?key={github}&after={stripe}");
        assert_eq!(
            replace_runs(&text, made_up_like),
            format!("Bearer {made_stripe}?key={made_github}&after={made_stripe}")
        );
        // A prefix of a key's shape with nothing after it is not a key.
        assert_eq!(replace_runs("sk_live_", made_up_like), "sk_live_");
        // Text with no key in it is left as it is.
        let plain = "https://api.example.com/v1/users/123?limit=5 text/html;charset=utf-8";
        assert_eq!(replace_runs(plain, made_up_like), plain);
    }

    #[test]
    fn a_key_at_the_end_of_a_sentence_is_found_and_the_stop_kept() {
        let key = "sk_live_4f2a9c81e7b3d6a04f2a9c81";
        let made = made_up_like(key).unwrap();
        assert_eq!(
            replace_runs(&format!("the key was {key}."), made_up_like),
            format!("the key was {made}.")
        );
        // The same key gives the same made-up key, and nothing of it is kept
        // beyond what its shape fixes.
        assert!(made.starts_with("sk_live_"));
        assert_ne!(made, key);
        assert!(!made.contains("4f2a9c81"));
    }

    #[test]
    fn an_escaped_key_of_json_text_is_found_and_the_escapes_left_as_written() {
        let key = grafana_key();
        assert!(is_key(&key));
        let made = made_up_like(&key).unwrap();
        // A slash may be written `\/`.
        let escaped = key.replace('/', "\\/");
        assert_eq!(
            replace_runs(&format!("{{\"token\": \"{escaped}\"}}"), made_up_like),
            format!("{{\"token\": \"{made}\"}}")
        );
        // A character may be written as the code it names.
        let named = key.replace('/', "\\u002f");
        assert_eq!(replace_runs(&named, made_up_like), made);
        // An escape a key cannot hold ends the run, and what follows it is
        // read from its start.
        let both = format!("\"{key}\\n{key}\"");
        assert_eq!(replace_runs(&both, made_up_like), format!("\"{made}\\n{made}\""));
        // A backslash that escapes nothing of JSON is an ordinary character.
        assert_eq!(replace_runs(&format!("{key}\\{key}"), made_up_like), format!("{made}\\{made}"));
    }
}
