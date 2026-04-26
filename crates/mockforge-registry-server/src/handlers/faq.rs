//! FAQ handlers

use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FAQResponse {
    pub faqs: Vec<FAQItem>,
}

#[derive(Debug, Serialize)]
pub struct FAQItem {
    pub id: String,
    pub category: String,
    pub question: String,
    pub answer: String,
}

/// Get FAQ items
pub async fn get_faq() -> Json<FAQResponse> {
    let faqs = vec![
        FAQItem {
            id: "1".to_string(),
            category: "Getting Started".to_string(),
            question: "How do I get started with MockForge Cloud?".to_string(),
            answer: "Sign up at app.mockforge.dev, choose your plan, and start deploying mocks. You can use the web UI or CLI. See our Getting Started guide for detailed instructions.".to_string(),
        },
        FAQItem {
            id: "2".to_string(),
            category: "Getting Started".to_string(),
            question: "What's the difference between Free, Pro, and Team plans?".to_string(),
            answer: "Free plan includes 10,000 requests/month, 1GB storage, and BYOK for AI features. Pro ($29/month) includes 250,000 requests, 20GB storage, 100K hosted AI tokens, and priority support. Team ($99/month) includes 1,000,000 requests, 100GB storage, 1M AI tokens, SSO, and a 24-hour support SLA.".to_string(),
        },
        FAQItem {
            id: "3".to_string(),
            category: "Billing".to_string(),
            question: "How does billing work?".to_string(),
            answer: "All paid plans are billed monthly. You can upgrade, downgrade, or cancel at any time. Changes take effect immediately. We accept credit cards via Stripe.".to_string(),
        },
        FAQItem {
            id: "4".to_string(),
            category: "Billing".to_string(),
            question: "What happens if I exceed my plan limits?".to_string(),
            answer: "You'll receive email notifications when approaching limits. Once exceeded, you can either upgrade your plan or wait for the next billing cycle. We offer grace periods for occasional overages.".to_string(),
        },
        FAQItem {
            id: "5".to_string(),
            category: "Billing".to_string(),
            question: "Can I get a refund?".to_string(),
            answer: "We offer a 14-day money-back guarantee for new subscriptions. Contact support@mockforge.dev for refund requests.".to_string(),
        },
        FAQItem {
            id: "6".to_string(),
            category: "Features".to_string(),
            question: "What is BYOK (Bring Your Own Key)?".to_string(),
            answer: "BYOK allows Free plan users to use their own OpenAI API key for AI features. This lets you use AI-powered mock generation without us incurring costs. Pro and Team plans include hosted AI features without needing your own key.".to_string(),
        },
        FAQItem {
            id: "7".to_string(),
            category: "Features".to_string(),
            question: "How do hosted mocks work?".to_string(),
            answer: "Hosted mocks are deployed mock services accessible via URLs (e.g., https://your-org.mockforge.dev/your-mock). They automatically scale and include health monitoring. Available on Pro and Team plans.".to_string(),
        },
        FAQItem {
            id: "8".to_string(),
            category: "Features".to_string(),
            question: "What is the Plugin Marketplace?".to_string(),
            answer: "The Plugin Marketplace lets you discover and install WASM plugins that extend MockForge functionality. Plugins can add custom behaviors, transformations, and integrations. You can also publish your own plugins.".to_string(),
        },
        FAQItem {
            id: "9".to_string(),
            category: "Features".to_string(),
            question: "What are Templates and Scenarios?".to_string(),
            answer: "Templates are pre-built chaos orchestration configurations you can apply to your projects. Scenarios are complete mock configurations that can be shared and reused. Both are available in their respective marketplaces.".to_string(),
        },
        FAQItem {
            id: "10".to_string(),
            category: "Technical".to_string(),
            question: "How do I migrate from local MockForge to Cloud?".to_string(),
            answer: "Export your local configuration, sign up for Cloud, authenticate, and import your configuration. See our Migration Guide for step-by-step instructions.".to_string(),
        },
        FAQItem {
            id: "11".to_string(),
            category: "Technical".to_string(),
            question: "What API rate limits apply?".to_string(),
            answer: "Two limits apply. (1) A short-window per-minute limit protects the platform from bursts; the active value is returned in the X-RateLimit-* response headers. (2) Your plan's monthly request quota (10,000 / 250,000 / 1,000,000 for Free / Pro / Team). Exceeding either returns HTTP 429.".to_string(),
        },
        FAQItem {
            id: "12".to_string(),
            category: "Technical".to_string(),
            question: "How do I authenticate with the API?".to_string(),
            answer: "You can use JWT tokens (from web login) or API tokens (Personal Access Tokens). API tokens are recommended for CLI and automation. Create them in Settings → API Tokens.".to_string(),
        },
        FAQItem {
            id: "13".to_string(),
            category: "Technical".to_string(),
            question: "Can I run MockForge on-premise instead of Cloud?".to_string(),
            answer: "Yes! MockForge is open-source and can be run locally or on your own infrastructure. Cloud is our hosted offering with additional features like hosted mocks and team collaboration.".to_string(),
        },
        FAQItem {
            id: "14".to_string(),
            category: "Support".to_string(),
            question: "What support do you offer?".to_string(),
            answer: "Free plan: Community support (best effort). Pro: Email support with 48-hour SLA. Team: Priority email support with 24-hour SLA. All plans include documentation and FAQ access.".to_string(),
        },
        FAQItem {
            id: "15".to_string(),
            category: "Support".to_string(),
            question: "How do I contact support?".to_string(),
            answer: "Submit a support request in-app (Support page), email support@mockforge.dev, or check our documentation at docs.mockforge.dev.".to_string(),
        },
        FAQItem {
            id: "16".to_string(),
            category: "Security".to_string(),
            question: "How secure is my data?".to_string(),
            answer: "We use industry-standard security practices: encrypted data in transit (TLS) and at rest, regular security audits, and compliance with GDPR. See our Privacy Policy and DPA for details.".to_string(),
        },
        FAQItem {
            id: "17".to_string(),
            category: "Security".to_string(),
            question: "Can I export or delete my data?".to_string(),
            answer: "Yes — both are self-serve. Open Account Settings → Privacy: 'Export my data' downloads a JSON archive (GDPR right to portability) and 'Delete my account' permanently erases your data. Programmatic equivalents are GET /api/v1/gdpr/export and DELETE /api/v1/gdpr/erase. Deletion completes within 30 days per GDPR requirements.".to_string(),
        },
        FAQItem {
            id: "18".to_string(),
            category: "Organizations".to_string(),
            question: "How do organizations work?".to_string(),
            answer: "Organizations are containers for your projects and resources. Each user has a personal organization. Team plans can create team organizations with multiple members and role-based access control.".to_string(),
        },
        FAQItem {
            id: "19".to_string(),
            category: "Organizations".to_string(),
            question: "Can I switch between organizations?".to_string(),
            answer: "Yes. Use 'mockforge org list' to see your organizations and 'mockforge org use <slug>' to switch context. The web UI also lets you switch organizations from the dropdown.".to_string(),
        },
        FAQItem {
            id: "20".to_string(),
            category: "Marketplace".to_string(),
            question: "How do I publish to the marketplace?".to_string(),
            answer: "Plugins can be published with `mockforge plugin publish <path>` from the CLI. Templates and scenarios are published from the web UI in their respective marketplaces. Submissions are reviewed before being made public, and verified content gets a badge.".to_string(),
        },
        FAQItem {
            id: "21".to_string(),
            category: "Security".to_string(),
            question: "How do I enable two-factor authentication?".to_string(),
            answer: "Open Account Settings → Security and click 'Enable 2FA'. Scan the QR code with any TOTP app (1Password, Authy, Google Authenticator), then enter the 6-digit code to confirm. You'll be issued recovery codes — store them somewhere safe. To disable later, re-enter your password.".to_string(),
        },
        FAQItem {
            id: "22".to_string(),
            category: "Organizations".to_string(),
            question: "How do I set up SAML SSO for my team?".to_string(),
            answer: "SSO is available on Team plans. Open Organization → SSO and configure your IdP's SSO URL, certificate, and entity ID. MockForge exposes SP metadata at /api/v1/sso/saml/metadata/<org-slug> and the ACS URL at /api/v1/sso/saml/acs/<org-slug>. Once configured, click 'Enable SSO' to require it for all org members.".to_string(),
        },
        FAQItem {
            id: "23".to_string(),
            category: "Features".to_string(),
            question: "Which protocols can I mock?".to_string(),
            answer: "MockForge supports HTTP/REST, gRPC, WebSocket, GraphQL, Kafka, MQTT, AMQP, SMTP, FTP, and raw TCP. Each protocol has its own server in the CLI (e.g. `--http-port`, `--grpc-port`, `--ws-port`) and can be driven from the same OpenAPI/proto/schema sources. Cloud currently hosts HTTP/WS/gRPC; the other protocols run locally or self-hosted.".to_string(),
        },
        FAQItem {
            id: "24".to_string(),
            category: "Features".to_string(),
            question: "What is Federation and when should I use it?".to_string(),
            answer: "Federation lets multiple MockForge instances cooperate so a single mock can be backed by responses from several upstreams or workspaces. Use it to compose mocks across teams, fan out requests to specialized instances, or front a hybrid of cloud-hosted and self-hosted mocks. Manage federations from the Federation page in the admin UI.".to_string(),
        },
        FAQItem {
            id: "25".to_string(),
            category: "Features".to_string(),
            question: "How do I record real traffic and replay it?".to_string(),
            answer: "Use the Recorder page (or `mockforge record` on the CLI) to capture live HTTP/gRPC/WS sessions to a fixture file. The captured fixture can be replayed deterministically as a mock or fed into the Behavioral Cloning workflow to generate variations. Combine with the Chaos page to inject latency, errors, or schema drift on top of the replay.".to_string(),
        },
        FAQItem {
            id: "26".to_string(),
            category: "Marketplace".to_string(),
            question: "How do I write my own plugin?".to_string(),
            answer: "Plugins are WebAssembly modules. Add the `mockforge-plugin-sdk` crate, implement the `Plugin` trait, and build with `cargo build --target wasm32-wasi --release`. Test locally with `mockforge plugin install ./target/wasm32-wasi/release/my_plugin.wasm`, then publish with `mockforge plugin publish`. See the response-graphql example under examples/plugins.".to_string(),
        },
        FAQItem {
            id: "27".to_string(),
            category: "Technical".to_string(),
            question: "When should I use JWT vs Personal Access Tokens?".to_string(),
            answer: "JWTs are short-lived (issued by web login) — use them for browser sessions and short scripts. Personal Access Tokens (PATs) are long-lived, scoped, and revocable — use them for CLI, CI/CD, and any automation. Create PATs in Settings → API Tokens. Both authenticate via `Authorization: Bearer <token>`.".to_string(),
        },
    ];

    Json(FAQResponse { faqs })
}
