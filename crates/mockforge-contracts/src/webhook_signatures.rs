//! How the providers sign the webhooks they send, in one table.
//!
//! A provider signs each webhook it sends, so that the application can check
//! that the event is the provider's and that nothing changed it on the way.
//! Each row of the table names the header that carries the signature and
//! says how the signature is computed. Each is taken from the provider's own
//! documentation, read on 2026-11-27 and named with the row, and not from a
//! rule set that says only that a header went unchecked. A test below signs
//! with a secret made up for each row.
//!
//! What it is for. The range's `webhooks` pack plants an endpoint that takes
//! an event with no signature at all (#1735). A mock that sends a provider's
//! webhooks itself, signed as the provider signs them, is #1703: these are
//! the headers it will have to send. Until then, what is here checks a
//! handler in a test. [`headers`] gives what a provider sends with a
//! delivery, and [`verified`] says whether a signature is one the provider
//! could have sent.
//!
//! What is computed and what is not. A shared secret is computed here: the
//! HMAC of the body, of the time and the body, or of the URL and its
//! parameters. Two providers sign with a key of their own and have the
//! application check with the public one, Discord and SendGrid. For them the
//! bytes that are signed are given ([`signed_bytes`]), and the signature
//! itself is left to #1703 and a library of key pairs.
//!
//! What a row says of the time. A provider that puts the time of signing in
//! the signature lets an application refuse an old delivery that is sent
//! again. Where the provider says how old is too old, the row says it. A
//! check of the time is not made here: [`Provider::tolerance`] is for the
//! application, against its own clock.

use std::collections::BTreeMap;
use std::time::Duration;

use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;
type HmacSha1 = Hmac<sha1::Sha1>;

const BASE64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// A signature as a header offers it, with the bytes it is over.
type Offered = (Vec<u8>, Vec<u8>);

/// A provider whose webhooks carry a signature the receiving application is
/// to check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provider {
    /// Sends through Svix, with `svix-signature`.
    Clerk,
    /// `X-Signature-Ed25519`: a key pair.
    Discord,
    /// `X-Hub-Signature-256`.
    Github,
    /// `X-Signature`.
    LemonSqueezy,
    /// `Paddle-Signature`.
    Paddle,
    /// Sends through Svix, with `svix-signature`.
    Resend,
    /// `X-Twilio-Email-Event-Webhook-Signature`: a key pair.
    Sendgrid,
    /// `X-Shopify-Hmac-Sha256`.
    Shopify,
    /// `X-Slack-Signature`.
    Slack,
    /// `Stripe-Signature`.
    Stripe,
    /// `svix-signature`.
    Svix,
    /// `X-Twilio-Signature`.
    Twilio,
}

impl Provider {
    /// Every provider of the table, by name.
    pub const ALL: [Provider; 12] = [
        Provider::Clerk,
        Provider::Discord,
        Provider::Github,
        Provider::LemonSqueezy,
        Provider::Paddle,
        Provider::Resend,
        Provider::Sendgrid,
        Provider::Shopify,
        Provider::Slack,
        Provider::Stripe,
        Provider::Svix,
        Provider::Twilio,
    ];

    /// The provider a name of the table names, as [`Self::id`] writes it:
    /// `github`, `lemon-squeezy`. `None` for any other name.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|provider| provider.id() == name)
    }

    /// The provider's name in the table and in the `webhooks` pack.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Clerk => "clerk",
            Provider::Discord => "discord",
            Provider::Github => "github",
            Provider::LemonSqueezy => "lemon-squeezy",
            Provider::Paddle => "paddle",
            Provider::Resend => "resend",
            Provider::Sendgrid => "sendgrid",
            Provider::Shopify => "shopify",
            Provider::Slack => "slack",
            Provider::Stripe => "stripe",
            Provider::Svix => "svix",
            Provider::Twilio => "twilio",
        }
    }

    /// The provider as it writes its own name.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Clerk => "Clerk",
            Provider::Discord => "Discord",
            Provider::Github => "GitHub",
            Provider::LemonSqueezy => "Lemon Squeezy",
            Provider::Paddle => "Paddle",
            Provider::Resend => "Resend",
            Provider::Sendgrid => "SendGrid",
            Provider::Shopify => "Shopify",
            Provider::Slack => "Slack",
            Provider::Stripe => "Stripe",
            Provider::Svix => "Svix",
            Provider::Twilio => "Twilio",
        }
    }

    /// The header that carries the signature.
    pub fn header(self) -> &'static str {
        match self {
            Provider::Clerk | Provider::Resend | Provider::Svix => "svix-signature",
            Provider::Discord => "X-Signature-Ed25519",
            Provider::Github => "X-Hub-Signature-256",
            Provider::LemonSqueezy => "X-Signature",
            Provider::Paddle => "Paddle-Signature",
            Provider::Sendgrid => "X-Twilio-Email-Event-Webhook-Signature",
            Provider::Shopify => "X-Shopify-Hmac-Sha256",
            Provider::Slack => "X-Slack-Signature",
            Provider::Stripe => "Stripe-Signature",
            Provider::Twilio => "X-Twilio-Signature",
        }
    }

    /// The header that carries the time of signing, where the provider
    /// sends it in a header of its own. Stripe and Paddle put the time in
    /// the signature header itself.
    pub fn time_header(self) -> Option<&'static str> {
        match self {
            Provider::Clerk | Provider::Resend | Provider::Svix => Some("svix-timestamp"),
            Provider::Discord => Some("X-Signature-Timestamp"),
            Provider::Sendgrid => Some("X-Twilio-Email-Event-Webhook-Timestamp"),
            Provider::Slack => Some("X-Slack-Request-Timestamp"),
            Provider::Github
            | Provider::LemonSqueezy
            | Provider::Paddle
            | Provider::Shopify
            | Provider::Stripe
            | Provider::Twilio => None,
        }
    }

    /// The header that carries the event's id, where the signature covers it.
    pub fn id_header(self) -> Option<&'static str> {
        match self {
            Provider::Clerk | Provider::Resend | Provider::Svix => Some("svix-id"),
            Provider::Discord
            | Provider::Github
            | Provider::LemonSqueezy
            | Provider::Paddle
            | Provider::Sendgrid
            | Provider::Shopify
            | Provider::Slack
            | Provider::Stripe
            | Provider::Twilio => None,
        }
    }

    /// How the signature is computed.
    pub fn scheme(self) -> Scheme {
        match self {
            Provider::Clerk | Provider::Resend | Provider::Svix => Scheme::IdTimeAndBody,
            Provider::Discord => Scheme::Ed25519TimeAndBody,
            Provider::Github | Provider::LemonSqueezy => Scheme::BodyInHex,
            Provider::Paddle => Scheme::TimeColonBody,
            Provider::Sendgrid => Scheme::EcdsaTimeAndBody,
            Provider::Shopify => Scheme::BodyInBase64,
            Provider::Slack => Scheme::VersionTimeAndBody,
            Provider::Stripe => Scheme::TimeDotBody,
            Provider::Twilio => Scheme::UrlAndParameters,
        }
    }

    /// How far from the time of signing a delivery may be, where the
    /// provider says. `None` where it says nothing.
    pub fn tolerance(self) -> Option<Duration> {
        match self {
            // "verify that the timestamp does not differ from local time by more than five minutes"
            Provider::Slack => Some(Duration::from_secs(5 * 60)),
            // "Our libraries have a default tolerance of 5 minutes"
            Provider::Stripe => Some(Duration::from_secs(5 * 60)),
            // "The SDK helper methods enforce a five-second timestamp tolerance by default"
            Provider::Paddle => Some(Duration::from_secs(5)),
            Provider::Clerk
            | Provider::Discord
            | Provider::Github
            | Provider::LemonSqueezy
            | Provider::Resend
            | Provider::Sendgrid
            | Provider::Shopify
            | Provider::Svix
            | Provider::Twilio => None,
        }
    }

    /// The provider's page that says all this.
    pub fn documentation(self) -> &'static str {
        match self {
            Provider::Clerk => "https://clerk.com/docs/guides/development/webhooks/overview",
            Provider::Discord => "https://docs.discord.com/developers/events/webhook-events",
            Provider::Github => {
                "https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries"
            }
            Provider::LemonSqueezy => "https://docs.lemonsqueezy.com/help/webhooks/signing-requests",
            Provider::Paddle => "https://developer.paddle.com/webhooks/about/signature-verification",
            Provider::Resend => "https://resend.com/docs/dashboard/webhooks/verify-webhooks-requests",
            Provider::Sendgrid => {
                "https://www.twilio.com/docs/sendgrid/for-developers/tracking-events/getting-started-event-webhook-security-features"
            }
            Provider::Shopify => "https://shopify.dev/docs/apps/build/webhooks/subscribe/https",
            Provider::Slack => "https://docs.slack.dev/authentication/verifying-requests-from-slack/",
            Provider::Stripe => "https://docs.stripe.com/webhooks",
            Provider::Svix => "https://docs.svix.com/receiving/verifying-payloads/how-manual",
            Provider::Twilio => "https://www.twilio.com/docs/usage/webhooks/webhooks-security",
        }
    }
}

/// How a signature is computed: what is signed and how it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// The HMAC-SHA256 of the body, in hexadecimal. GitHub writes `sha256=`
    /// before it; Lemon Squeezy writes it alone.
    BodyInHex,
    /// The HMAC-SHA256 of the body, in base64: Shopify.
    BodyInBase64,
    /// The HMAC-SHA256 of `v0`, the time of signing and the body, each after
    /// a colon, in hexadecimal after `v0=`: Slack.
    VersionTimeAndBody,
    /// The HMAC-SHA256 of the time of signing, a full stop and the body, in
    /// hexadecimal after `v1=` in `t=<time>,v1=<signature>`: Stripe. A
    /// second `v1` is there while a secret is being replaced.
    TimeDotBody,
    /// The HMAC-SHA256 of the event's id, its time of signing and its body,
    /// each after a full stop, in base64 after `v1,`: Svix, and Clerk and
    /// Resend by way of it.
    IdTimeAndBody,
    /// The HMAC-SHA256 of the time of signing, a colon and the body, in
    /// hexadecimal in `ts=<time>;h1=<signature>`: Paddle.
    TimeColonBody,
    /// The HMAC-SHA1 of the URL and of the parameters of a form, sorted by
    /// name and each written as its name then its value, in base64: Twilio.
    UrlAndParameters,
    /// Ed25519 of the time of signing and the body, in hexadecimal: Discord.
    Ed25519TimeAndBody,
    /// ECDSA with SHA-256 of the time of signing and the body, in base64:
    /// SendGrid.
    EcdsaTimeAndBody,
}

impl Scheme {
    /// Whether the signature is computed here, from a secret the provider
    /// and the application share.
    pub fn with_shared_secret(self) -> bool {
        !matches!(self, Scheme::Ed25519TimeAndBody | Scheme::EcdsaTimeAndBody)
    }

    /// Whether the time of signing is among the bytes signed.
    pub fn signs_time(self) -> bool {
        !matches!(self, Scheme::BodyInHex | Scheme::BodyInBase64 | Scheme::UrlAndParameters)
    }
}

/// What a provider signs when it sends an event.
#[derive(Clone, Copy, Debug)]
pub struct Delivery<'a> {
    /// The body as it is sent: nothing has parsed it and written it again.
    pub body: &'a [u8],
    /// The time of signing, in seconds since the epoch.
    pub at: u64,
    /// The event's id, where the provider sends one and the signature covers it.
    pub id: Option<&'a str>,
    /// The URL the event is sent to, as Twilio is given it.
    pub url: Option<&'a str>,
    /// The parameters of the form Twilio sends, by name and value.
    pub parameters: &'a [(&'a str, &'a str)],
}

impl<'a> Delivery<'a> {
    /// A delivery of `body`, signed now, with nothing else.
    pub fn now(body: &'a [u8]) -> Self {
        let since = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Self {
            body,
            at: since,
            id: None,
            url: None,
            parameters: &[],
        }
    }

    /// The same delivery, signed at `at`.
    pub fn at(mut self, at: u64) -> Self {
        self.at = at;
        self
    }

    /// The same delivery, carrying `id`.
    pub fn with_id(mut self, id: &'a str) -> Self {
        self.id = Some(id);
        self
    }

    /// The same delivery, sent to `url` with the parameters of a form.
    pub fn with_form(mut self, url: &'a str, parameters: &'a [(&'a str, &'a str)]) -> Self {
        self.url = Some(url);
        self.parameters = parameters;
        self
    }
}

/// Why a signature is not one this table computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsigned {
    /// The provider signs with a key of its own, and the application checks
    /// with the public one. The bytes to sign are given by [`signed_bytes`].
    KeyPair,
    /// The signature covers the event's id, and the delivery has none.
    NoId,
    /// The signature covers the URL, and the delivery has none.
    NoUrl,
    /// The provider gives its secret as base64 after a prefix, and the part
    /// of this one after the prefix is not base64.
    SecretNotBase64,
}

impl std::fmt::Display for Unsigned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unsigned::KeyPair => {
                write!(f, "signed with a key pair, which this table does not compute")
            }
            Unsigned::NoId => {
                write!(f, "the signature covers the event's id, and the delivery has none")
            }
            Unsigned::NoUrl => write!(f, "the signature covers the URL, and the delivery has none"),
            Unsigned::SecretNotBase64 => write!(f, "the secret is not base64 after its prefix"),
        }
    }
}

/// The bytes the provider signs for `delivery`.
///
/// For a JSON body Twilio signs the URL with a `bodySHA256` parameter added
/// to it, holding the SHA-256 of the body as hexadecimal; the URL is given
/// with it already.
pub fn signed_bytes(provider: Provider, delivery: &Delivery<'_>) -> Result<Vec<u8>, Unsigned> {
    let body = delivery.body;
    let at = delivery.at;
    let time_then_body = |start: String| {
        let mut signed = start.into_bytes();
        signed.extend_from_slice(body);
        signed
    };
    Ok(match provider.scheme() {
        Scheme::BodyInHex | Scheme::BodyInBase64 => body.to_vec(),
        Scheme::VersionTimeAndBody => time_then_body(format!("v0:{at}:")),
        Scheme::TimeDotBody => time_then_body(format!("{at}.")),
        Scheme::IdTimeAndBody => {
            time_then_body(format!("{}.{at}.", delivery.id.ok_or(Unsigned::NoId)?))
        }
        Scheme::TimeColonBody => time_then_body(format!("{at}:")),
        Scheme::Ed25519TimeAndBody | Scheme::EcdsaTimeAndBody => time_then_body(at.to_string()),
        Scheme::UrlAndParameters => {
            let mut signed = delivery.url.ok_or(Unsigned::NoUrl)?.as_bytes().to_vec();
            let mut parameters: Vec<&(&str, &str)> = delivery.parameters.iter().collect();
            parameters.sort();
            for (name, value) in parameters {
                signed.extend_from_slice(name.as_bytes());
                signed.extend_from_slice(value.as_bytes());
            }
            signed
        }
    })
}

/// The key a shared secret gives. Svix, and Clerk and Resend by way of it,
/// give a secret whose part after its prefix is base64. Stripe's secret
/// begins with `whsec_` too; its page says nothing more of it, and the rest
/// is taken as it is written.
fn key(provider: Provider, secret: &str) -> Result<Vec<u8>, Unsigned> {
    match provider {
        Provider::Clerk | Provider::Resend | Provider::Svix => BASE64
            .decode(secret.split_once('_').map_or(secret, |(_, rest)| rest))
            .map_err(|_| Unsigned::SecretNotBase64),
        Provider::Stripe => Ok(secret.strip_prefix("whsec_").unwrap_or(secret).as_bytes().to_vec()),
        Provider::Discord
        | Provider::Github
        | Provider::LemonSqueezy
        | Provider::Paddle
        | Provider::Sendgrid
        | Provider::Shopify
        | Provider::Slack
        | Provider::Twilio => Ok(secret.as_bytes().to_vec()),
    }
}

/// An HMAC state, keyed with the shared secret, for the hash the scheme
/// uses: SHA-1 for Twilio, SHA-256 for the rest.
enum State {
    /// HMAC-SHA256, every provider but Twilio.
    Sha256(HmacSha256),
    /// HMAC-SHA1, Twilio.
    Sha1(HmacSha1),
}

impl State {
    fn new(scheme: Scheme, key: &[u8]) -> Self {
        if scheme == Scheme::UrlAndParameters {
            Self::Sha1(HmacSha1::new_from_slice(key).expect("HMAC takes a key of any length"))
        } else {
            Self::Sha256(HmacSha256::new_from_slice(key).expect("HMAC takes a key of any length"))
        }
    }

    /// The signature over `signed`, as bytes.
    fn seal(self, signed: &[u8]) -> Vec<u8> {
        match self {
            Self::Sha256(mut mac) => {
                mac.update(signed);
                mac.finalize().into_bytes().to_vec()
            }
            Self::Sha1(mut mac) => {
                mac.update(signed);
                mac.finalize().into_bytes().to_vec()
            }
        }
    }

    /// Whether `offered` is the signature over `signed`. The comparison
    /// takes the same time whatever differs.
    fn holds(self, signed: &[u8], offered: &[u8]) -> bool {
        match self {
            Self::Sha256(mut mac) => {
                mac.update(signed);
                mac.verify_slice(offered).is_ok()
            }
            Self::Sha1(mut mac) => {
                mac.update(signed);
                mac.verify_slice(offered).is_ok()
            }
        }
    }
}

/// The headers the provider sends with the signature of `delivery`, signed
/// with `secret`, by the names the provider writes them.
///
/// For Discord and SendGrid the result is [`Unsigned::KeyPair`]: they sign
/// with a key of their own, and [`signed_bytes`] gives what is signed.
///
/// # Errors
///
/// [`Unsigned::KeyPair`] for the two providers that sign with a key pair,
/// [`Unsigned::NoId`] where the signature covers an id the delivery has
/// none, [`Unsigned::NoUrl`] for Twilio without a URL, and
/// [`Unsigned::SecretNotBase64`] where the secret is to be base64 after its
/// prefix and is not.
pub fn headers(
    provider: Provider,
    secret: &str,
    delivery: &Delivery<'_>,
) -> Result<BTreeMap<&'static str, String>, Unsigned> {
    if !provider.scheme().with_shared_secret() {
        return Err(Unsigned::KeyPair);
    }
    let signed = signed_bytes(provider, delivery)?;
    let seal = State::new(provider.scheme(), &key(provider, secret)?).seal(&signed);
    let written = match provider.scheme() {
        Scheme::BodyInHex if provider == Provider::Github => {
            format!("sha256={}", hex::encode(seal))
        }
        Scheme::BodyInHex => hex::encode(seal),
        Scheme::BodyInBase64 | Scheme::UrlAndParameters => BASE64.encode(seal),
        Scheme::VersionTimeAndBody => format!("v0={}", hex::encode(seal)),
        Scheme::TimeDotBody => format!("t={},v1={}", delivery.at, hex::encode(seal)),
        Scheme::IdTimeAndBody => format!("v1,{}", BASE64.encode(seal)),
        Scheme::TimeColonBody => format!("ts={};h1={}", delivery.at, hex::encode(seal)),
        Scheme::Ed25519TimeAndBody | Scheme::EcdsaTimeAndBody => String::new(),
    };
    let mut headers = BTreeMap::new();
    headers.insert(provider.header(), written);
    if let Some(time) = provider.time_header() {
        headers.insert(time, delivery.at.to_string());
    }
    if let Some(id) = provider.id_header() {
        headers.insert(id, delivery.id.ok_or(Unsigned::NoId)?.to_string());
    }
    Ok(headers)
}

/// The signatures in the value of the provider's signature header, as bytes,
/// each with the bytes they are over. What the header holds of the time of
/// signing is used, where it holds it; elsewhere the delivery's is.
fn offered(
    provider: Provider,
    delivery: &Delivery<'_>,
    signature: &str,
) -> Result<Vec<Offered>, Unsigned> {
    let scheme = provider.scheme();
    let mut found = Vec::new();
    let mut take = |seal: &str, signed: Vec<u8>| {
        let decoded = if matches!(
            scheme,
            Scheme::BodyInBase64 | Scheme::IdTimeAndBody | Scheme::UrlAndParameters
        ) {
            BASE64.decode(seal.trim()).ok()
        } else {
            hex::decode(seal.trim()).ok()
        };
        if let Some(seal) = decoded {
            found.push((signed, seal));
        }
    };
    // The bytes signed with a time of the header's own, before the body.
    let time_then_body = |time: &str| {
        let mut signed = match scheme {
            Scheme::TimeDotBody => format!("{time}.").into_bytes(),
            Scheme::TimeColonBody => format!("{time}:").into_bytes(),
            _ => Vec::new(),
        };
        signed.extend_from_slice(delivery.body);
        signed
    };
    match scheme {
        Scheme::BodyInHex if provider == Provider::Github => {
            if let Some(seal) = signature.strip_prefix("sha256=") {
                take(seal, signed_bytes(provider, delivery)?);
            }
        }
        Scheme::BodyInHex | Scheme::BodyInBase64 | Scheme::UrlAndParameters => {
            take(signature, signed_bytes(provider, delivery)?);
        }
        Scheme::VersionTimeAndBody => {
            if let Some(seal) = signature.strip_prefix("v0=") {
                take(seal, signed_bytes(provider, delivery)?);
            }
        }
        // Entries the provider has not told of are passed over, as Stripe
        // says an application must.
        Scheme::TimeDotBody => {
            let time = signature
                .split(',')
                .filter_map(|entry| entry.strip_prefix("t="))
                .next()
                .unwrap_or_default();
            for seal in signature.split(',').filter_map(|entry| entry.strip_prefix("v1=")) {
                take(seal, time_then_body(time));
            }
        }
        Scheme::IdTimeAndBody => {
            for seal in signature.split(' ').filter_map(|part| part.strip_prefix("v1,")) {
                take(seal, signed_bytes(provider, delivery)?);
            }
        }
        Scheme::TimeColonBody => {
            let time = signature
                .split(';')
                .filter_map(|entry| entry.strip_prefix("ts="))
                .next()
                .unwrap_or_default();
            for seal in signature.split(';').filter_map(|entry| entry.strip_prefix("h1=")) {
                take(seal, time_then_body(time));
            }
        }
        // The signature of a key pair is not compared here.
        Scheme::Ed25519TimeAndBody | Scheme::EcdsaTimeAndBody => {}
    }
    Ok(found)
}

/// Whether `signature`, the value of the provider's signature header, is
/// one the provider could have sent for `delivery` with `secret`.
///
/// Where the signature header holds the time of signing, that time is the
/// one signed, as the provider signs it; elsewhere it is the delivery's.
/// The time itself is not judged: [`Provider::tolerance`] says what the
/// provider allows, against the application's clock.
///
/// # Errors
///
/// What [`headers`] returns, for the same reasons.
pub fn verified(
    provider: Provider,
    secret: &str,
    delivery: &Delivery<'_>,
    signature: &str,
) -> Result<bool, Unsigned> {
    if !provider.scheme().with_shared_secret() {
        return Err(Unsigned::KeyPair);
    }
    let key = key(provider, secret)?;
    let scheme = provider.scheme();
    Ok(offered(provider, delivery, signature)?
        .into_iter()
        .any(|(signed, seal)| State::new(scheme, &key).holds(&signed, &seal)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A secret made up for the tests.
    const SECRET: &str = "it's a secret to everybody";
    /// Svix's secret is base64 after its prefix, as Svix gives one.
    const SVIX_SECRET: &str = "whsec_U2VjcmV0IHRvIGV2ZXJ5Ym9keQ==";
    const STRIPE_SECRET: &str = "whsec_X2hzZWN0";
    const TEXT: &str = r#"{"type":"ping","at":"2026-11-27T16:00:00Z"}"#;
    const BODY: &[u8] = TEXT.as_bytes();
    const AT: u64 = 1_764_250_000;

    const FORM: [(&str, &str); 3] = [
        ("To", "+15551234567"),
        ("From", "+15557654321"),
        ("Body", "Hi"),
    ];

    fn delivery() -> Delivery<'static> {
        Delivery::now(BODY)
            .at(AT)
            .with_id("msg_7f3e")
            .with_form("https://example.test/webhooks/twilio", &FORM)
    }

    fn hmac_sha256(key: &[u8], signed: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(signed);
        mac.finalize().into_bytes().to_vec()
    }

    /// The secret of a row, as its provider gives one.
    fn secret_of(provider: Provider) -> &'static str {
        match provider {
            Provider::Clerk | Provider::Resend | Provider::Svix => SVIX_SECRET,
            Provider::Stripe => STRIPE_SECRET,
            _ => SECRET,
        }
    }

    /// Another secret than the row's, well made for the row.
    fn other_secret(provider: Provider) -> &'static str {
        match provider {
            Provider::Clerk | Provider::Resend | Provider::Svix => "whsec_YW5vdGhlciBzZWNyZXQ=",
            _ => "another secret altogether",
        }
    }

    fn signature_of(provider: Provider, delivery: &Delivery<'_>) -> String {
        headers(provider, secret_of(provider), delivery).unwrap()[provider.header()].clone()
    }

    #[test]
    fn every_row_names_what_its_provider_documents() {
        for provider in Provider::ALL {
            let own = provider.documentation();
            let domain = match provider {
                Provider::Clerk => "clerk.com/",
                Provider::Discord => "docs.discord.com/",
                Provider::Github => "docs.github.com/",
                Provider::LemonSqueezy => "docs.lemonsqueezy.com/",
                Provider::Paddle => "developer.paddle.com/",
                Provider::Resend => "resend.com/",
                Provider::Sendgrid => "twilio.com/docs/sendgrid/",
                Provider::Shopify => "shopify.dev/",
                Provider::Slack => "docs.slack.dev/",
                Provider::Stripe => "docs.stripe.com/",
                Provider::Svix => "docs.svix.com/",
                Provider::Twilio => "twilio.com/docs/usage/webhooks/",
            };
            assert!(own.starts_with("https://") && own.contains(domain), "{provider:?}: {own}");
            assert_eq!(Provider::named(provider.id()), Some(provider));
            // A time that is signed is sent, in a header or in the signature.
            assert!(provider.scheme().signs_time() || provider.time_header().is_none());
        }
        assert_eq!(Provider::named("lemon-squeezy"), Some(Provider::LemonSqueezy));
        assert_eq!(Provider::named("nobody"), None);
        assert_eq!(Provider::ALL.len(), 12);
    }

    #[test]
    fn github_is_as_its_own_example() {
        // The secret, the payload and the signature of the page named with
        // the row.
        let delivery = Delivery::now(b"Hello, World!").at(1);
        let sent = headers(Provider::Github, "It's a Secret to Everybody", &delivery).unwrap();
        assert_eq!(
            sent[Provider::Github.header()],
            "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17"
        );
        assert_eq!(sent.len(), 1, "GitHub sends the time in no header");
    }

    #[test]
    fn a_signature_of_the_body_is_the_hmac_of_the_body() {
        let delivery = delivery();
        let of_body = hmac_sha256(SECRET.as_bytes(), BODY);
        assert_eq!(
            signature_of(Provider::Github, &delivery),
            format!("sha256={}", hex::encode(&of_body))
        );
        assert_eq!(signature_of(Provider::LemonSqueezy, &delivery), hex::encode(&of_body));
        assert_eq!(signature_of(Provider::Shopify, &delivery), BASE64.encode(&of_body));
        // The body as it was sent, not as it would be written again.
        assert_ne!(
            signature_of(
                Provider::LemonSqueezy,
                &Delivery::now(b"{\"at\":\"2026-11-27T16:00:00Z\",\"type\":\"ping\"}").at(AT)
            ),
            hex::encode(&of_body)
        );
    }

    #[test]
    fn a_signature_of_the_time_and_the_body_is_the_hmac_of_both() {
        let delivery = delivery();
        let slack = headers(Provider::Slack, SECRET, &delivery).unwrap();
        assert_eq!(slack["X-Slack-Request-Timestamp"], AT.to_string());
        assert_eq!(
            slack[Provider::Slack.header()],
            format!(
                "v0={}",
                hex::encode(hmac_sha256(SECRET.as_bytes(), format!("v0:{AT}:{TEXT}").as_bytes()))
            )
        );
        assert_eq!(slack.len(), 2);

        let stripe = headers(Provider::Stripe, STRIPE_SECRET, &delivery).unwrap();
        let expected = hex::encode(hmac_sha256(b"X2hzZWN0", format!("{AT}.{TEXT}").as_bytes()));
        assert_eq!(stripe[Provider::Stripe.header()], format!("t={AT},v1={expected}"));
        assert_eq!(stripe.len(), 1, "the time is in the signature header");

        let paddle = headers(Provider::Paddle, SECRET, &delivery).unwrap();
        let expected =
            hex::encode(hmac_sha256(SECRET.as_bytes(), format!("{AT}:{TEXT}").as_bytes()));
        assert_eq!(paddle[Provider::Paddle.header()], format!("ts={AT};h1={expected}"));
    }

    #[test]
    fn a_signature_of_the_id_the_time_and_the_body_is_svix_s() {
        let delivery = delivery();
        // The part of the secret after `whsec_`, read as base64.
        let expected = BASE64.encode(hmac_sha256(
            b"Secret to everybody",
            format!("msg_7f3e.{AT}.{TEXT}").as_bytes(),
        ));
        for provider in [Provider::Svix, Provider::Clerk, Provider::Resend] {
            let sent = headers(provider, SVIX_SECRET, &delivery).unwrap();
            assert_eq!(sent[provider.header()], format!("v1,{expected}"), "{provider:?}");
            assert_eq!(sent["svix-timestamp"], AT.to_string());
            assert_eq!(sent["svix-id"], "msg_7f3e");
            assert_eq!(sent.len(), 3);
        }
    }

    #[test]
    fn twilio_signs_the_url_and_the_sorted_form() {
        let delivery = delivery();
        // Sorted by name: Body, From, To; each as its name then its value.
        let signed = "https://example.test/webhooks/twilioBodyHiFrom+15557654321To+15551234567";
        let mut mac = HmacSha1::new_from_slice(SECRET.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let expected = BASE64.encode(mac.finalize().into_bytes());
        assert_eq!(signature_of(Provider::Twilio, &delivery), expected);
        assert_eq!(signed_bytes(Provider::Twilio, &delivery).unwrap(), signed.as_bytes());
    }

    #[test]
    fn the_two_who_sign_with_a_key_pair_give_the_bytes_and_no_signature() {
        let delivery = delivery();
        for provider in [Provider::Discord, Provider::Sendgrid] {
            assert!(!provider.scheme().with_shared_secret());
            assert_eq!(headers(provider, SECRET, &delivery), Err(Unsigned::KeyPair));
            assert_eq!(verified(provider, SECRET, &delivery, "00"), Err(Unsigned::KeyPair));
            // The time of signing as it is sent, then the body.
            assert_eq!(
                signed_bytes(provider, &delivery).unwrap(),
                format!("{AT}{TEXT}").as_bytes()
            );
            assert!(provider.time_header().is_some());
        }
        assert_eq!(Provider::Discord.time_header(), Some("X-Signature-Timestamp"));
        assert_eq!(
            Provider::Sendgrid.time_header(),
            Some("X-Twilio-Email-Event-Webhook-Timestamp")
        );
    }

    #[test]
    fn what_a_row_signs_is_what_it_takes_and_nothing_else() {
        let delivery = delivery();
        for provider in Provider::ALL {
            if !provider.scheme().with_shared_secret() {
                continue;
            }
            let sent = signature_of(provider, &delivery);
            assert_eq!(
                verified(provider, secret_of(provider), &delivery, &sent),
                Ok(true),
                "{provider:?}"
            );
            // A body changed by one byte, or for Twilio a parameter of its
            // form changed by one: Twilio signs the form and not the body
            // beside it.
            let changed =
                Delivery::now(b"{\"type\":\"ping\",\"at\":\"2026-11-27T16:00:01Z\"}").at(AT);
            let changed = match provider.id_header() {
                Some(_) => changed.with_id("msg_7f3e"),
                None => changed,
            };
            let changed = match provider.scheme() {
                Scheme::UrlAndParameters => changed.with_form(
                    "https://example.test/webhooks/twilio",
                    &[
                        ("To", "+15551234567"),
                        ("From", "+15557654321"),
                        ("Body", "Ho"),
                    ],
                ),
                _ => changed,
            };
            assert_eq!(
                verified(provider, secret_of(provider), &changed, &sent),
                Ok(false),
                "{provider:?}: what was signed changed is taken"
            );
            // Another secret.
            assert_eq!(
                verified(provider, other_secret(provider), &delivery, &sent),
                Ok(false),
                "{provider:?}: another secret's signature is taken"
            );
        }
    }

    #[test]
    fn a_time_signed_is_the_time_sent() {
        let delivery = delivery();
        for provider in [
            Provider::Slack,
            Provider::Discord,
            Provider::Sendgrid,
            Provider::Svix,
            Provider::Clerk,
            Provider::Resend,
        ] {
            if !provider.scheme().with_shared_secret() {
                continue;
            }
            let sent = signature_of(provider, &delivery);
            let later = Delivery::now(BODY).at(AT + 60 * 60 * 24 * 365).with_id("msg_7f3e");
            assert_eq!(
                verified(provider, secret_of(provider), &later, &sent),
                Ok(false),
                "{provider:?}: the time signed is the time of the header"
            );
        }
        // Stripe and Paddle hold the time in the signature header itself.
        let stripe = signature_of(Provider::Stripe, &delivery);
        let replaced = stripe.replace(&format!("t={AT},"), &format!("t={},", AT + 31_536_000));
        assert_eq!(verified(Provider::Stripe, STRIPE_SECRET, &delivery, &replaced), Ok(false));
        assert_eq!(verified(Provider::Stripe, STRIPE_SECRET, &delivery, &stripe), Ok(true));
        let paddle = signature_of(Provider::Paddle, &delivery);
        let replaced = paddle.replace(&format!("ts={AT};"), &format!("ts={},", AT + 31_536_000));
        assert_eq!(verified(Provider::Paddle, SECRET, &delivery, &replaced), Ok(false));
    }

    #[test]
    fn several_signatures_and_entries_untold_of_are_taken_as_the_providers_say() {
        let delivery = delivery();
        // Stripe: a second signature from the time a secret is replaced, an
        // entry of another scheme, and entries that are nobody's business.
        let v1 = signature_of(Provider::Stripe, &delivery);
        let v1 = v1.strip_prefix(&format!("t={AT},")).unwrap();
        let wrong = hex::encode([0xaa; 32]);
        let header = format!("t={AT},v1={wrong},v0=zzz,secret=whsec_****,{v1},destination=we_1");
        assert_eq!(verified(Provider::Stripe, STRIPE_SECRET, &delivery, &header), Ok(true));
        let header = format!("t={AT},v0={v1},v1={wrong},destination=we_1");
        assert_eq!(verified(Provider::Stripe, STRIPE_SECRET, &delivery, &header), Ok(false));

        // Svix: a space-separated list, where one is the delivery's.
        let v1 = signature_of(Provider::Svix, &delivery);
        let header = format!("v1,AAAA{}, {v1}", "A".repeat(20));
        assert_eq!(verified(Provider::Svix, SVIX_SECRET, &delivery, &header), Ok(true));
        assert_eq!(
            verified(Provider::Svix, SVIX_SECRET, &delivery, &v1.replace("v1,", "v2,")),
            Ok(false)
        );

        // Paddle: more than one h1 while a secret is replaced.
        let h1 = signature_of(Provider::Paddle, &delivery);
        let header = format!("{};h1={}", h1, hex::encode([0u8; 32]));
        assert_eq!(verified(Provider::Paddle, SECRET, &delivery, &header), Ok(true));
    }

    #[test]
    fn what_is_missing_is_named() {
        let delivery = Delivery::now(BODY);
        assert_eq!(headers(Provider::Svix, SVIX_SECRET, &delivery), Err(Unsigned::NoId));
        assert_eq!(
            verified(Provider::Svix, SVIX_SECRET, &delivery, "v1,AAAA"),
            Err(Unsigned::NoId)
        );
        assert_eq!(headers(Provider::Twilio, SECRET, &delivery), Err(Unsigned::NoUrl));
        assert_eq!(verified(Provider::Twilio, SECRET, &delivery, "AAAA"), Err(Unsigned::NoUrl));
        assert_eq!(
            headers(Provider::Svix, "whsec_not base64!", &Delivery::now(BODY).with_id("x")),
            Err(Unsigned::SecretNotBase64)
        );
        assert_eq!(
            Unsigned::NoId.to_string(),
            "the signature covers the event's id, and the delivery has none"
        );
        assert_eq!(
            Unsigned::KeyPair.to_string(),
            "signed with a key pair, which this table does not compute"
        );
    }

    #[test]
    fn the_reference_page_lists_every_provider_as_the_table_does() {
        let page = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../book/src/reference/webhook-signatures.md");
        let page = std::fs::read_to_string(&page)
            .unwrap_or_else(|error| panic!("{}: {error}", page.display()));
        for provider in Provider::ALL {
            let row = page
                .lines()
                .find(|line| line.starts_with(&format!("| {} |", provider.name())))
                .unwrap_or_else(|| panic!("{} has no row on the page", provider.name()));
            for header in [
                Some(provider.header()),
                provider.time_header(),
                provider.id_header(),
            ]
            .into_iter()
            .flatten()
            {
                assert!(
                    row.contains(&format!("`{header}`")),
                    "{header} is not in {provider:?}'s row: {row}"
                );
            }
            let allowed = match provider.tolerance() {
                None if provider.scheme().signs_time() => "not said",
                None => "not signed",
                Some(time) if time.as_secs() < 60 => "5 seconds",
                Some(_) => "5 minutes",
            };
            assert!(row.contains(allowed), "{allowed} is not in {provider:?}'s row: {row}");
        }
    }

    #[test]
    fn a_tolerance_is_said_where_the_provider_says_one() {
        assert_eq!(Provider::Slack.tolerance(), Some(Duration::from_secs(300)));
        assert_eq!(Provider::Stripe.tolerance(), Some(Duration::from_secs(300)));
        assert_eq!(Provider::Paddle.tolerance(), Some(Duration::from_secs(5)));
        for provider in [
            Provider::Clerk,
            Provider::Discord,
            Provider::Github,
            Provider::LemonSqueezy,
            Provider::Resend,
            Provider::Sendgrid,
            Provider::Shopify,
            Provider::Svix,
            Provider::Twilio,
        ] {
            assert_eq!(provider.tolerance(), None, "{provider:?}");
        }
    }
}
