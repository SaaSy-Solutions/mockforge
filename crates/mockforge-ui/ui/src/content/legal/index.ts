/**
 * Legal documents for MockForge Cloud — the single source of truth.
 *
 * The copy lives in the sibling Markdown files and is bundled into the app,
 * so /legal/* renders for anonymous visitors without an API round-trip.
 * To change a document: edit its .md file and bump `version` /
 * `lastUpdated` below.
 *
 * REVIEW STATUS: DRAFT, pending legal review (September 2026). Items counsel
 * must confirm before these are treated as final:
 *   - Governing law / venue (Terms §15): the state of organization of
 *     SaaSy Solutions LLC is referenced generically.
 *   - Sub-processor list (DPA §7): compiled from the codebase and
 *     infrastructure docs. Confirm the Ashburn, Virginia hosting provider,
 *     the transactional email provider actually configured in production
 *     (code supports Postmark, Brevo, SMTP), whether Cloudflare R2 holds
 *     customer uploads, whether hosted AI features send customer content to
 *     OpenAI/Anthropic, and remove Fly.io once the legacy machines are retired.
 *   - Contact mailboxes (legal@, privacy@, security@, support@mockforge.dev)
 *     must exist and be monitored.
 */
import termsMarkdown from './terms.md?raw';
import privacyMarkdown from './privacy.md?raw';
import dpaMarkdown from './dpa.md?raw';

export type LegalDocumentId = 'terms' | 'privacy' | 'dpa';

export interface LegalDocument {
  id: LegalDocumentId;
  title: string;
  /** Canonical, publicly reachable path. */
  path: `/legal/${LegalDocumentId}`;
  version: string;
  lastUpdated: string;
  markdown: string;
}

export const LEGAL_DOCUMENTS: Record<LegalDocumentId, LegalDocument> = {
  terms: {
    id: 'terms',
    title: 'Terms of Service',
    path: '/legal/terms',
    version: '2.0',
    lastUpdated: 'September 27, 2026',
    markdown: termsMarkdown,
  },
  privacy: {
    id: 'privacy',
    title: 'Privacy Policy',
    path: '/legal/privacy',
    version: '2.0',
    lastUpdated: 'September 27, 2026',
    markdown: privacyMarkdown,
  },
  dpa: {
    id: 'dpa',
    title: 'Data Processing Agreement',
    path: '/legal/dpa',
    version: '2.0',
    lastUpdated: 'September 27, 2026',
    markdown: dpaMarkdown,
  },
};
