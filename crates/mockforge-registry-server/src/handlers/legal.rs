//! Legal document handlers (Terms of Service, Privacy Policy, DPA)

use axum::Json;
use serde_json::{json, Value};

/// Get Terms of Service
/// Returns the current Terms of Service document
pub async fn get_terms() -> Json<Value> {
    Json(json!({
        "version": "1.0",
        "last_updated": "2025-01-27",
        "content": r#"
# Terms of Service

**Last Updated: January 27, 2025**

## 1. Acceptance of Terms

By accessing and using MockForge Cloud ("Service"), you accept and agree to be bound by the terms and provision of this agreement.

## 2. Description of Service

MockForge Cloud is a cloud-hosted version of MockForge, providing API mocking, testing, and development tools. The Service includes:

- Plugin Marketplace
- Template Marketplace
- Scenario Marketplace
- Hosted Mock Services
- Organization and team collaboration features

## 3. User Accounts

### 3.1 Account Creation
You must provide accurate, current, and complete information during registration and keep your account information updated.

### 3.2 Account Security
You are responsible for maintaining the confidentiality of your account credentials and for all activities that occur under your account.

### 3.3 Account Types
- **Free Tier**: Limited usage, BYOK (Bring Your Own Key) required for AI features
- **Pro Tier**: Increased limits, hosted AI features
- **Team Tier**: Advanced collaboration, SSO support

## 4. Acceptable Use

### 4.1 Prohibited Activities
You agree not to:
- Violate any applicable laws or regulations
- Infringe upon intellectual property rights
- Transmit malicious code or malware
- Attempt to gain unauthorized access to the Service
- Use the Service for illegal or fraudulent purposes
- Abuse or overload the Service infrastructure

### 4.2 Content Responsibility
You are solely responsible for all content you publish, upload, or share through the Service.

## 5. Subscription and Billing

### 5.1 Subscription Plans
Subscription plans are billed monthly or annually as selected. Prices are subject to change with 30 days notice.

### 5.2 Payment
Payments are processed through Stripe. You agree to provide valid payment information.

### 5.3 Refunds
Refunds are provided at our discretion. Contact support for refund requests.

### 5.4 Cancellation
You may cancel your subscription at any time. Cancellation takes effect at the end of the current billing period.

## 6. Usage Limits

Each subscription tier has specific usage limits (requests, storage, AI tokens, etc.). Exceeding limits may result in service throttling or suspension.

## 7. Intellectual Property

### 7.1 Our Rights
The Service, including all software, content, and materials, is owned by MockForge and protected by intellectual property laws.

### 7.2 Your Rights
You retain ownership of content you create. By using the Service, you grant us a license to host, store, and serve your content.

## 8. Data and Privacy

Your use of the Service is also governed by our Privacy Policy. Please review it to understand our data practices.

## 9. Service Availability

We strive to maintain high availability but do not guarantee uninterrupted service. We reserve the right to perform maintenance, updates, or modifications that may temporarily affect availability.

## 10. Termination

### 10.1 By You
You may terminate your account at any time through your account settings.

### 10.2 By Us
We may suspend or terminate your account if you violate these Terms or engage in harmful activities.

## 11. Limitation of Liability

TO THE MAXIMUM EXTENT PERMITTED BY LAW, MOCKFORGE SHALL NOT BE LIABLE FOR ANY INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, OR PUNITIVE DAMAGES.

## 12. Changes to Terms

We reserve the right to modify these Terms at any time. Material changes will be communicated via email or in-app notification. Continued use constitutes acceptance of modified Terms.

## 13. Contact

For questions about these Terms, contact us at:
- Email: legal@mockforge.dev
- Support: support@mockforge.dev

## 14. Governing Law

These Terms are governed by the laws of [Your Jurisdiction], without regard to conflict of law provisions.
"#
    }))
}

/// Get Privacy Policy
/// Returns the current Privacy Policy document
pub async fn get_privacy() -> Json<Value> {
    Json(json!({
        "version": "1.0",
        "last_updated": "2025-01-27",
        "content": r#"
# Privacy Policy

**Last Updated: January 27, 2025**

## 1. Introduction

MockForge ("we", "our", "us") is committed to protecting your privacy. This Privacy Policy explains how we collect, use, disclose, and safeguard your information when you use MockForge Cloud.

## 2. Information We Collect

### 2.1 Account Information
- Name, email address, username
- Organization information
- Billing address and payment information (processed by Stripe)

### 2.2 Usage Data
- API request logs
- Feature usage statistics
- Error logs and diagnostics
- Performance metrics

### 2.3 Content Data
- Plugins, templates, and scenarios you publish
- Mock configurations and fixtures
- Organization data and settings

### 2.4 Technical Data
- IP addresses
- Browser type and version
- Device information
- Cookies and similar tracking technologies

## 3. How We Use Your Information

We use collected information to:
- Provide and maintain the Service
- Process transactions and manage subscriptions
- Send important service notifications
- Improve and optimize the Service
- Detect and prevent fraud or abuse
- Comply with legal obligations

## 4. Data Sharing and Disclosure

### 4.1 Service Providers
We share data with trusted service providers:
- **Stripe**: Payment processing
- **Postmark/Brevo**: Email delivery
- **Neon**: Database hosting
- **Upstash**: Redis caching
- **Backblaze B2**: Object storage
- **Fly.io**: Application hosting

### 4.2 Legal Requirements
We may disclose information if required by law or to protect our rights and safety.

### 4.3 Business Transfers
In the event of a merger or acquisition, your data may be transferred to the new entity.

## 5. Data Security

We implement industry-standard security measures:
- Encryption in transit (TLS/SSL)
- Encryption at rest for sensitive data
- Regular security audits
- Access controls and authentication
- Secure password hashing

## 6. Data Retention

- **Account Data**: Retained while your account is active
- **Usage Data**: Retained for up to 90 days
- **Billing Records**: Retained as required by law (typically 7 years)
- **Deleted Accounts**: Data is deleted within 30 days of account deletion

## 7. Your Rights

### 7.1 Access and Correction
You can access and update your account information through your account settings.

### 7.2 Data Export
You can request a copy of your data by contacting support.

### 7.3 Data Deletion
You can delete your account and associated data through account settings or by contacting support.

### 7.4 Opt-Out
You can opt out of marketing emails while still receiving service-related communications.

## 8. Cookies and Tracking

We use cookies and similar technologies for:
- Authentication and session management
- Analytics and performance monitoring
- Service functionality

You can control cookies through your browser settings.

## 9. International Data Transfers

Your data may be transferred to and processed in countries other than your own. We ensure appropriate safeguards are in place.

## 10. Children's Privacy

Our Service is not intended for users under 13 years of age. We do not knowingly collect information from children.

## 11. California Privacy Rights

California residents have additional rights under CCPA:
- Right to know what personal information is collected
- Right to delete personal information
- Right to opt-out of sale of personal information (we do not sell your data)

## 12. GDPR Compliance

If you are in the European Economic Area (EEA), you have additional rights:
- Right to access your data
- Right to rectification
- Right to erasure ("right to be forgotten")
- Right to restrict processing
- Right to data portability
- Right to object to processing

## 13. Changes to Privacy Policy

We may update this Privacy Policy from time to time. Material changes will be communicated via email or in-app notification.

## 14. Contact Us

For privacy-related questions or requests:
- Email: privacy@mockforge.dev
- Support: support@mockforge.dev
- Data Protection Officer: dpo@mockforge.dev

## 15. Data Processing Agreement (DPA)

For enterprise customers, we offer a Data Processing Agreement. Contact sales@mockforge.dev for details.
"#
    }))
}

/// Get Data Processing Agreement (DPA)
/// Returns the current DPA document
pub async fn get_dpa() -> Json<Value> {
    Json(json!({
        "version": "1.0",
        "last_updated": "2025-01-27",
        "content": r#"
# Data Processing Agreement (DPA)

**Last Updated: January 27, 2025**

This Data Processing Agreement ("DPA") forms part of the Terms of Service and governs the processing of personal data by MockForge on behalf of our customers.

## 1. Definitions

- **Controller**: The customer who determines the purposes and means of processing personal data
- **Processor**: MockForge, who processes personal data on behalf of the Controller
- **Personal Data**: Any information relating to an identified or identifiable natural person
- **Processing**: Any operation performed on personal data

## 2. Scope and Purpose

This DPA applies when MockForge processes Personal Data on behalf of customers in connection with the Service.

## 3. Processing Details

### 3.1 Subject Matter
Processing of Personal Data necessary to provide the MockForge Cloud Service.

### 3.2 Duration
For the duration of the customer's use of the Service.

### 3.3 Nature and Purpose
- Hosting and storage of customer data
- API request processing
- User authentication and authorization
- Service delivery and support

### 3.4 Types of Personal Data
- User account information
- Organization member data
- Usage and analytics data
- Content created by users

## 4. Processor Obligations

### 4.1 Compliance
MockForge will:
- Process Personal Data only in accordance with customer instructions
- Implement appropriate technical and organizational measures
- Ensure personnel are bound by confidentiality obligations
- Assist customers in fulfilling their obligations under GDPR

### 4.2 Security Measures
- Encryption in transit and at rest
- Regular security assessments
- Access controls and authentication
- Incident response procedures

### 4.3 Sub-processors
MockForge may engage sub-processors (listed in Section 6) with customer consent. We will:
- Ensure sub-processors are bound by similar obligations
- Notify customers of new sub-processors
- Allow 30 days for objection

## 5. Customer Obligations

Customers are responsible for:
- Ensuring they have lawful basis for processing
- Obtaining necessary consents
- Providing accurate instructions
- Complying with applicable data protection laws

## 6. Sub-processors

Current sub-processors:
- **Stripe** (Payment processing) - United States
- **Postmark/Brevo** (Email delivery) - United States
- **Neon** (Database hosting) - United States/Europe
- **Upstash** (Redis caching) - United States/Europe
- **Backblaze B2** (Object storage) - United States
- **Fly.io** (Application hosting) - United States/Europe

## 7. Data Subject Rights

MockForge will assist customers in responding to data subject requests:
- Right of access
- Right to rectification
- Right to erasure
- Right to restrict processing
- Right to data portability
- Right to object

## 8. Data Breach Notification

In the event of a data breach, MockForge will:
- Notify the customer without undue delay (within 72 hours)
- Provide details of the breach
- Assist in breach notification to authorities if required

## 9. Data Transfers

Personal Data may be transferred outside the EEA. We ensure appropriate safeguards:
- Standard Contractual Clauses (SCCs)
- Adequacy decisions where applicable
- Binding Corporate Rules where applicable

## 10. Audit Rights

Customers may audit MockForge's compliance with this DPA, subject to:
- Reasonable notice
- Confidentiality obligations
- Limited frequency (annually, unless breach suspected)

## 11. Return or Deletion

Upon termination of the Service:
- Customer may export their data
- MockForge will delete Personal Data within 30 days
- Backup data will be deleted within 90 days

## 12. Liability

Each party's liability is limited as set forth in the Terms of Service.

## 13. Contact

For DPA-related inquiries:
- Email: dpo@mockforge.dev
- Legal: legal@mockforge.dev

## 14. Governing Law

This DPA is governed by the same law as the Terms of Service.
"#
    }))
}
