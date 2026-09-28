import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import ReactMarkdown, { type Components } from 'react-markdown';
import { LEGAL_DOCUMENTS, type LegalDocumentId } from '@/content/legal';
import { Logo } from '@/components/ui/Logo';

// Tailwind v4 preflight strips heading/list styling and the typography plugin
// is not registered, so style the Markdown elements explicitly.
const markdownComponents: Components = {
  h1: ({ children }) => <h2 className="text-2xl font-semibold mt-8 mb-3">{children}</h2>,
  h2: ({ children }) => <h2 className="text-xl font-semibold mt-8 mb-3">{children}</h2>,
  h3: ({ children }) => <h3 className="text-lg font-semibold mt-6 mb-2">{children}</h3>,
  p: ({ children }) => <p className="leading-7 my-3">{children}</p>,
  ul: ({ children }) => <ul className="list-disc pl-6 my-3 space-y-1">{children}</ul>,
  ol: ({ children }) => <ol className="list-decimal pl-6 my-3 space-y-1">{children}</ol>,
  li: ({ children }) => <li className="leading-7">{children}</li>,
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  a: ({ href, children }) =>
    href && href.startsWith('/') ? (
      <Link to={href} className="text-primary underline underline-offset-2">
        {children}
      </Link>
    ) : (
      <a href={href} className="text-primary underline underline-offset-2" rel="noopener noreferrer">
        {children}
      </a>
    ),
};

function LegalNav({ current }: { current: LegalDocumentId }): ReactNode {
  return (
    <nav aria-label="Legal documents" className="flex flex-wrap gap-4 text-sm">
      {Object.values(LEGAL_DOCUMENTS).map((doc) => (
        <Link
          key={doc.id}
          to={doc.path}
          aria-current={doc.id === current ? 'page' : undefined}
          className={
            doc.id === current
              ? 'font-medium text-foreground'
              : 'text-muted-foreground hover:text-foreground'
          }
        >
          {doc.title}
        </Link>
      ))}
    </nav>
  );
}

/**
 * Public legal document page. Rendered outside AuthGuard/AppShell so it is
 * reachable by anonymous visitors and does not depend on the API.
 */
export function LegalDocumentPage({ id }: { id: LegalDocumentId }) {
  const doc = LEGAL_DOCUMENTS[id];

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b">
        <div className="mx-auto max-w-3xl px-4 py-4 flex flex-wrap items-center justify-between gap-4">
          <Link to="/" aria-label="MockForge home">
            <Logo variant="full" size="md" />
          </Link>
          <LegalNav current={id} />
        </div>
      </header>
      <main className="mx-auto max-w-3xl px-4 py-10">
        <h1 className="text-3xl font-bold">{doc.title}</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Version {doc.version} · Last updated {doc.lastUpdated}
        </p>
        <article className="mt-6">
          <ReactMarkdown components={markdownComponents}>{doc.markdown}</ReactMarkdown>
        </article>
      </main>
    </div>
  );
}

export function TermsPage() {
  return <LegalDocumentPage id="terms" />;
}

export function PrivacyPage() {
  return <LegalDocumentPage id="privacy" />;
}

export function DPAPage() {
  return <LegalDocumentPage id="dpa" />;
}
