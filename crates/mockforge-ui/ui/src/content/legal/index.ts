/**
 * Legal documents for MockForge Cloud — the single source of truth.
 *
 * The copy lives in the sibling Markdown files and is bundled into the app,
 * so /legal/* renders for anonymous visitors without an API round-trip.
 * To change a document: edit its .md file and bump `version` /
 * `lastUpdated` below.
 *
 * Status: approved for launch (September 2026); counsel should review after
 * launch, notably governing law / venue (Terms §15, which names the state of
 * organization of SaaSy Solutions LLC generically).
 *
 * The sub-processor list (DPA §7) reflects production as verified on the
 * Ashburn host. Keep it in sync with infrastructure: adding a provider
 * requires 30 days' notice to customers (DPA §7).
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
    version: '2.1',
    lastUpdated: 'September 30, 2026',
    markdown: privacyMarkdown,
  },
  dpa: {
    id: 'dpa',
    title: 'Data Processing Agreement',
    path: '/legal/dpa',
    version: '2.1',
    lastUpdated: 'September 30, 2026',
    markdown: dpaMarkdown,
  },
};
