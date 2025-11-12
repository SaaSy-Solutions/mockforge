import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader2 } from 'lucide-react';
import ReactMarkdown from 'react-markdown';

interface LegalDocument {
  version: string;
  last_updated: string;
  content: string;
}

async function fetchTerms(): Promise<LegalDocument> {
  const response = await fetch('/api/v1/legal/terms');
  if (!response.ok) {
    throw new Error('Failed to fetch Terms of Service');
  }
  return response.json();
}

export function TermsPage() {
  const [terms, setTerms] = useState<LegalDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchTerms()
      .then((data) => {
        setTerms(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err.message);
        setLoading(false);
      });
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  if (error || !terms) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Error loading Terms of Service: {error || 'Unknown error'}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <Card>
        <CardHeader>
          <CardTitle className="text-3xl">Terms of Service</CardTitle>
          <p className="text-sm text-muted-foreground">
            Version {terms.version} • Last updated: {terms.last_updated}
          </p>
        </CardHeader>
        <CardContent>
          <div className="prose prose-slate dark:prose-invert max-w-none">
            <ReactMarkdown>{terms.content}</ReactMarkdown>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
