# Webhook Signatures

A provider signs each webhook it sends, so that the application can check that the event is the provider's and that nothing changed it on the way. This page is the table of how each does it: the header that carries the signature, what is signed, and how the signature is written.

Each row is taken from the provider's own documentation, read on 2026-11-27, and names the page it was taken from. It is kept in `mockforge_contracts::webhook_signatures`, where a test signs with a secret made up for each row, and a test holds this page to the table.

## The table

| Provider | Signature header | What is signed | Written as | Time allowed |
| --- | --- | --- | --- | --- |
| Clerk | `svix-signature`, with `svix-id` and `svix-timestamp` | id, time and body, each after a full stop | `v1,` then HMAC-SHA256 in base64 | not said |
| Discord | `X-Signature-Ed25519`, with `X-Signature-Timestamp` | time, then body | Ed25519 in hexadecimal | not said |
| GitHub | `X-Hub-Signature-256` | body | `sha256=` then HMAC-SHA256 in hexadecimal | not signed |
| Lemon Squeezy | `X-Signature` | body | HMAC-SHA256 in hexadecimal | not signed |
| Paddle | `Paddle-Signature` | time, a colon, then body | `ts=<time>;h1=<signature>` in hexadecimal | 5 seconds |
| Resend | `svix-signature`, with `svix-id` and `svix-timestamp` | id, time and body, each after a full stop | `v1,` then HMAC-SHA256 in base64 | not said |
| SendGrid | `X-Twilio-Email-Event-Webhook-Signature`, with `X-Twilio-Email-Event-Webhook-Timestamp` | time, then body | ECDSA with SHA-256 in base64 | not said |
| Shopify | `X-Shopify-Hmac-Sha256` | body | HMAC-SHA256 in base64 | not signed |
| Slack | `X-Slack-Signature`, with `X-Slack-Request-Timestamp` | `v0`, time and body, each after a colon | `v0=` then HMAC-SHA256 in hexadecimal | 5 minutes |
| Stripe | `Stripe-Signature` | time, a full stop, then body | `t=<time>,v1=<signature>` in hexadecimal | 5 minutes, as its libraries have it |
| Svix | `svix-signature`, with `svix-id` and `svix-timestamp` | id, time and body, each after a full stop | `v1,` then HMAC-SHA256 in base64 | not said |
| Twilio | `X-Twilio-Signature` | the URL, then each parameter of the form by name and value, sorted by name | HMAC-SHA1 in base64 | not signed |

Three things to know of it:

- **The body is signed as it was sent.** Nothing may parse it and write it again first: a key moved or a space added makes every signature wrong.
- **The time is there to refuse an old delivery sent again.** A provider that signs the time lets the application compare it with its own clock. The table says what the provider allows where the provider says; the rest is the application's to choose.
- **Two sign with a key of their own.** Discord and SendGrid have the application check with a public key. For them the table gives the bytes that are signed and not the signature.

## Checking a handler

What a provider would send for a delivery can be had directly, to check that a handler takes it, and that it refuses a signature that is not the provider's.

```rust
use mockforge_contracts::webhook_signatures::{headers, verified, Delivery, Provider};

let secret = "It's a Secret to Everybody";
let delivery = Delivery::now(br#"{"type":"user.created"}"#);

// The headers GitHub sends with this delivery.
let sent = headers(Provider::Github, secret, &delivery)?;
// sent["X-Hub-Signature-256"] is "sha256=" and the signature.

// What a handler asks of a request it was sent.
let taken = verified(Provider::Github, secret, &delivery, &sent["X-Hub-Signature-256"])?;
assert!(taken);
```

`verified` judges the signature alone. Where the signature header holds the time, as Stripe's and Paddle's do, that is the time signed; elsewhere it is the delivery's. `Provider::tolerance` says how far from now the provider lets a delivery be, where the provider says.

Twilio signs the URL and the parameters of a form, so its delivery is given both:

```rust
use mockforge_contracts::webhook_signatures::{Delivery, Provider};

let delivery = Delivery::now(b"")
    .with_form("https://app.example/webhooks/twilio", &[("To", "+15551234567"), ("From", "+15557654321")]);
let sent = headers(Provider::Twilio, "an auth token", &delivery)?;
```

For a JSON body Twilio adds a `bodySHA256` parameter to the URL, holding the SHA-256 of the body, and signs that URL.

## What the providers do not say

A row says only what its page says. These are left as the page leaves them:

- **Stripe** says the secret begins with `whsec_` and nothing more of it. The rest is used as it is written.
- **Discord** shows how it signs in code only, and says nothing of how old a delivery may be.
- **SendGrid** says ECDSA with SHA-256, and not which curve, in what form the key is given, or how old a delivery may be.
- **Svix** says to check the time against your own, and gives no number. Clerk and Resend send through Svix: Clerk's page names `svix-id` alone, and Resend's names the headers in its examples only.
- **Lemon Squeezy** and **Twilio** show how they sign in code and examples, and not in words.

Where a provider changes how it signs, the row changes with it, and the test with the row.

## Where it is going

The range's `webhooks` pack plants an endpoint that takes a provider's event with no signature at all. A mock that sends a provider's webhooks itself, signed as the provider signs them and with a delivery log to read them back from, is what this table is the first step of.
