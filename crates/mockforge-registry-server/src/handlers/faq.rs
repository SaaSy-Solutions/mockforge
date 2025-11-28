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
            answer: "Free plan includes 10,000 requests/month, 1GB storage, and BYOK for AI features. Pro ($29/month) includes 100,000 requests, 10GB storage, hosted AI, and priority support. Team ($99/month) includes 1M requests, 100GB storage, SSO, and 24-hour support SLA.".to_string(),
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
            answer: "Rate limits vary by plan. Free: 60 requests/minute, Pro: 300 requests/minute, Team: 1000 requests/minute. Monthly request limits also apply. Check the X-RateLimit-* headers in API responses for current limits.".to_string(),
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
            answer: "Yes. You can export your data at any time via the API or by contacting support. You can also request data deletion, which we'll process within 30 days per GDPR requirements.".to_string(),
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
            answer: "Use the CLI commands (mockforge plugin publish, template publish, scenario publish) or the web UI. Your content will be reviewed before being made public. Verified content gets a badge.".to_string(),
        },
    ];

    Json(FAQResponse { faqs })
}
