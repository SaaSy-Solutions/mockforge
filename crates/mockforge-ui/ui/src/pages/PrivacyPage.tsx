import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader2 } from 'lucide-react';
import ReactMarkdown from 'react-markdown';

interface LegalDocument {
  version: string;
  last_updated: string;
  content: string;
}

async function fetchPrivacy(): Promise<LegalDocument> {
  const response = await fetch('/api/v1/legal/privacy');
  if (!response.ok) {
    throw new Error('Failed to fetch Privacy Policy');
  }
  return response.json();
}

export function PrivacyPage() {
  const [privacy, setPrivacy] = useState<LegalDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchPrivacy()
      .then((data) => {
        setPrivacy(data);
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

  if (error || !privacy) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Error loading Privacy Policy: {error || 'Unknown error'}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <Card>
        <CardHeader>
          <CardTitle className="text-3xl">Privacy Policy</CardTitle>
          <p className="text-sm text-muted-foreground">
            Version {privacy.version} • Last updated: {privacy.last_updated}
          </p>
        </CardHeader>
        <CardContent>
          <div className="prose prose-slate dark:prose-invert max-w-none">
            <ReactMarkdown>{privacy.content}</ReactMarkdown>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
