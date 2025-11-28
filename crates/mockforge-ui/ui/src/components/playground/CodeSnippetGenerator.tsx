import { logger } from '@/utils/logger';
import React, { useState, useEffect } from 'react';
import { Copy, Check, Code2, Download } from 'lucide-react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/Tabs';
import { Badge } from '../ui/Badge';
import { usePlaygroundStore } from '../../stores/usePlaygroundStore';
import { apiService } from '../../services/api';
import { toast } from 'sonner';

/**
 * Code Snippet Generator Component
 *
 * Generates code snippets in multiple languages:
 * - curl
 * - JavaScript (fetch)
 * - Python (requests)
 * - Go
 * - Rust
 *
 * Supports both REST and GraphQL requests
 */
export function CodeSnippetGenerator() {
  const { protocol, restRequest, graphQLRequest } = usePlaygroundStore();
  const [snippets, setSnippets] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [copiedLanguage, setCopiedLanguage] = useState<string | null>(null);
  const [selectedLanguage, setSelectedLanguage] = useState<string>('curl');

  // Generate snippets when request changes
  useEffect(() => {
    generateSnippets();
  }, [protocol, restRequest, graphQLRequest]);

  // Generate code snippets
  const generateSnippets = async () => {
    setLoading(true);
    try {
      // Determine base URL
      const baseUrl = protocol === 'rest'
        ? restRequest.base_url || 'http://localhost:3000'
        : graphQLRequest.base_url || 'http://localhost:4000';

      const request = {
        protocol,
        method: protocol === 'rest' ? restRequest.method : undefined,
        path: protocol === 'rest' ? restRequest.path : '/graphql',
        headers: protocol === 'rest' ? restRequest.headers : undefined,
        body: protocol === 'rest' && restRequest.body
          ? (() => {
              try {
                return JSON.parse(restRequest.body);
              } catch {
                return restRequest.body;
              }
            })()
          : undefined,
        graphql_query: protocol === 'graphql' ? graphQLRequest.query : undefined,
        graphql_variables:
          protocol === 'graphql' && Object.keys(graphQLRequest.variables).length > 0
            ? graphQLRequest.variables
            : undefined,
        base_url: baseUrl,
      };

      const response = await apiService.generateCodeSnippet(request);
      setSnippets(response.snippets);
    } catch (error) {
      logger.error('Failed to generate code snippets', error);
      toast.error('Failed to generate code snippets');
    } finally {
      setLoading(false);
    }
  };

  // Copy snippet to clipboard
  const handleCopy = async (language: string, code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopiedLanguage(language);
      toast.success(`Copied ${language} snippet`);
      setTimeout(() => setCopiedLanguage(null), 2000);
    } catch (error) {
      logger.error('Failed to copy to clipboard', error);
      toast.error('Failed to copy');
    }
  };

  // Download snippet
  const handleDownload = (language: string, code: string) => {
    const extension = getFileExtension(language);
    const blob = new Blob([code], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `snippet.${extension}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success(`Downloaded ${language} snippet`);
  };

  // Get file extension for language
  const getFileExtension = (language: string): string => {
    const extensions: Record<string, string> = {
      curl: 'sh',
      javascript: 'js',
      python: 'py',
      go: 'go',
      rust: 'rs',
    };
    return extensions[language] || 'txt';
  };

  // Get language label
  const getLanguageLabel = (language: string): string => {
    const labels: Record<string, string> = {
      curl: 'cURL',
      javascript: 'JavaScript',
      python: 'Python',
      go: 'Go',
      rust: 'Rust',
    };
    return labels[language] || language;
  };

  // Available languages
  const availableLanguages = Object.keys(snippets).filter((lang) => snippets[lang]);

  if (availableLanguages.length === 0 && !loading) {
    return (
      <Card>
        <CardContent className="p-6 text-center text-muted-foreground">
          Configure a request to generate code snippets
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <CardTitle className="text-lg font-semibold flex items-center gap-2">
          <Code2 className="h-5 w-5" />
          Code Snippets
        </CardTitle>
      </CardHeader>

      <CardContent className="flex-1 overflow-auto">
        {loading ? (
          <div className="flex items-center justify-center py-8">
            <div className="text-center space-y-2">
              <div className="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
              <p className="text-sm text-muted-foreground">Generating snippets...</p>
            </div>
          </div>
        ) : (
          <Tabs value={selectedLanguage} onValueChange={setSelectedLanguage}>
            <TabsList className="grid w-full grid-cols-5">
              {availableLanguages.map((lang) => (
                <TabsTrigger key={lang} value={lang} className="text-xs">
                  {getLanguageLabel(lang)}
                </TabsTrigger>
              ))}
            </TabsList>

            {availableLanguages.map((lang) => (
              <TabsContent key={lang} value={lang} className="mt-4">
                <div className="flex items-center justify-between mb-2">
                  <Badge variant="outline">{getLanguageLabel(lang)}</Badge>
                  <div className="flex gap-2">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleCopy(lang, snippets[lang])}
                    >
                      {copiedLanguage === lang ? (
                        <Check className="h-4 w-4 text-green-600" />
                      ) : (
                        <Copy className="h-4 w-4" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDownload(lang, snippets[lang])}
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
                <pre className="bg-muted/30 rounded-md p-4 font-mono text-sm overflow-auto max-h-[500px] whitespace-pre-wrap">
                  {snippets[lang]}
                </pre>
              </TabsContent>
            ))}
          </Tabs>
        )}
      </CardContent>
    </Card>
  );
}
