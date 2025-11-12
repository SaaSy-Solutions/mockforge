import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader2 } from 'lucide-react';
import ReactMarkdown from 'react-markdown';

interface LegalDocument {
  version: string;
  last_updated: string;
  content: string;
}

async function fetchDPA(): Promise<LegalDocument> {
  const response = await fetch('/api/v1/legal/dpa');
  if (!response.ok) {
    throw new Error('Failed to fetch Data Processing Agreement');
  }
  return response.json();
}

export function DPAPage() {
  const [dpa, setDpa] = useState<LegalDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchDPA()
      .then((data) => {
        setDpa(data);
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

  if (error || !dpa) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Error loading Data Processing Agreement: {error || 'Unknown error'}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <Card>
        <CardHeader>
          <CardTitle className="text-3xl">Data Processing Agreement (DPA)</CardTitle>
          <p className="text-sm text-muted-foreground">
            Version {dpa.version} • Last updated: {dpa.last_updated}
          </p>
        </CardHeader>
        <CardContent>
          <div className="prose prose-slate dark:prose-invert max-w-none">
            <ReactMarkdown>{dpa.content}</ReactMarkdown>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
