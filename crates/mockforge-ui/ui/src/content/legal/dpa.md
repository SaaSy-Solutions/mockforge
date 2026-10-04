This Data Processing Agreement ("DPA") forms part of the [Terms of Service](/legal/terms) between the customer ("Customer", controller) and **SaaSy Solutions LLC** ("Processor", "we") and applies whenever we process Customer Personal Data in providing MockForge Cloud. If this DPA conflicts with the Terms, this DPA controls for data-protection matters.

## 1. Definitions

"Data Protection Laws" means the laws that apply to the processing, including the EU and UK GDPR and US state privacy laws such as the CCPA/CPRA. "Customer Personal Data" means personal data in Customer Content or account data that we process on Customer's behalf. "Sub-processor" means a third party we engage to process Customer Personal Data. Other capitalized terms have the meanings given in the Data Protection Laws.

## 2. Processing details

- **Subject matter and duration:** providing the Service for the term of the Terms, plus the deletion period in Section 10.
- **Nature and purpose:** hosting, storage, transmission, display and support of Customer Content; authentication; billing; security monitoring.
- **Data subjects:** Customer's users and organization members, and any individuals whose data Customer places in Customer Content.
- **Categories of data:** contact and account details, authentication identifiers, usage and log data, and any personal data Customer includes in specifications, fixtures or recorded traffic. Customer should not upload special-category data to mock fixtures.

## 3. Customer instructions

We process Customer Personal Data only on Customer's documented instructions, which are the Terms, this DPA and Customer's configuration of the Service, unless the law requires otherwise; in that case we will inform Customer unless the law prohibits it. We will tell Customer if we believe an instruction infringes Data Protection Laws.

## 4. Confidentiality

Personnel with access to Customer Personal Data are bound by confidentiality obligations and access it only as needed to provide, secure and support the Service.

## 5. Security

We maintain technical and organizational measures appropriate to the risk, including: TLS 1.2+ for data in transit, including between Cloudflare's edge and our origin; a primary database on a dedicated server with restricted administrative access and storage encrypted at rest (LUKS2, AES-XTS); database backups encrypted before they leave that server; storage of backups and uploaded files in Cloudflare R2, which encrypts stored data at rest; hashed credentials; role-based access control; per-organization tenant isolation, enforced in the application and, as defense in depth, by database row-level security on organization-keyed tables; audit logging; vulnerability scanning and dependency auditing; and an incident-response process.

## 6. Personal data breaches

We will notify Customer without undue delay, and in any event within 72 hours, after becoming aware of a breach affecting Customer Personal Data. The notice will describe what we know about the breach, its likely consequences and the measures taken, and we will update Customer as more becomes available.

## 7. Sub-processors

Customer authorizes the Sub-processors below. We impose data-protection obligations on each Sub-processor that are no less protective than this DPA and remain responsible for their performance. We will announce new Sub-processors on this page at least 30 days before they begin processing; Customer may object on reasonable data-protection grounds, and if we cannot address the objection, Customer may terminate the affected Service and receive a pro-rata refund of prepaid fees.

- **Hetzner Online GmbH:** servers in Ashburn, Virginia that run the MockForge Cloud application and its PostgreSQL database, which we operate ourselves (United States).
- **Cloudflare:** object storage (R2) for customer uploads and artifacts, plus DNS, CDN, TLS termination, tunneling and DDoS/WAF protection (United States and global edge).
- **Fly.io:** compute for customer-deployed hosted mocks served at `*.mocks.mockforge.dev` (United States).
- **Stripe:** payment processing and subscription billing (United States).
- **Brevo:** transactional email delivery (European Union).

## 8. Assistance and data subject requests

Taking into account the nature of the processing, we will help Customer respond to data subject requests, carry out data-protection impact assessments and consult supervisory authorities. If we receive a request directly from a data subject about Customer Personal Data, we will refer it to Customer.

## 9. International transfers

We process Customer Personal Data in the United States. For transfers of personal data from the EEA, UK or Switzerland, the parties agree to the EU Standard Contractual Clauses (Module 2, controller to processor; Module 3 where Customer is itself a processor) and the UK Addendum, which are incorporated by reference, with Customer as data exporter and SaaSy Solutions LLC as data importer.

## 10. Return and deletion

Customer can export Customer Content during the term and for 30 days after termination. We then delete Customer Personal Data from active systems within 30 days, and from backups within 90 days, unless the law requires us to keep it. Security and audit logs that we keep to secure the Service (event type, timestamp, IP address, user agent and pseudonymous identifiers, but no email addresses or usernames) are retained for 400 days from the event and then deleted, including after termination, as described in Section 5 of our [Privacy Policy](/legal/privacy).

## 11. Audits

We will make available information reasonably necessary to demonstrate compliance with this DPA, including answers to security questionnaires. Where that is not sufficient, Customer may conduct an audit once per year, on 30 days' notice, during business hours, at its own cost and subject to confidentiality obligations.

## 12. Liability

Each party's liability under this DPA is subject to the limitations in the Terms, except where Data Protection Laws do not permit such limitation.

## 13. Contact

Data-protection questions and signed-copy requests: privacy@mockforge.dev
