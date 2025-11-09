import { logger } from '@/utils/logger';
import React, { useState, useMemo } from 'react';
import { Copy, Download, Check, Eye, Code, AlertCircle, Clock, FileJson, Sparkles } from 'lucide-react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/Tabs';
import { Badge } from '../ui/Badge';
import { Switch } from '../ui/switch';
import { Label } from '../ui/label';
import { usePlaygroundStore } from '../../stores/usePlaygroundStore';
import { toast } from 'sonner';

/**
 * Response Panel Component
 *
 * Displays the response from executed requests with:
 * - JSON tree view (collapsible)
 * - Raw JSON view
 * - Headers display
 * - Status code and timing
 * - Error display
 * - MockAI preview toggle (placeholder for future implementation)
 */
export function ResponsePanel() {
  const { currentResponse, mockAIResponse, responseError, responseLoading, protocol, executeRestRequest } = usePlaygroundStore();
  const [viewMode, setViewMode] = useState<'tree' | 'raw'>('tree');
  const [copied, setCopied] = useState(false);
  const [mockAIPreview, setMockAIPreview] = useState(false);
  const [mockAILoading, setMockAILoading] = useState(false);

  // Use MockAI response when preview is enabled, otherwise use standard response
  const displayResponse = mockAIPreview && mockAIResponse ? mockAIResponse : currentResponse;

  // Format response body as JSON string
  const responseBodyString = useMemo(() => {
    if (!displayResponse) return '';
    try {
      return JSON.stringify(displayResponse.body, null, 2);
    } catch {
      return String(displayResponse.body);
    }
  }, [displayResponse]);

  // Copy to clipboard
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(responseBodyString);
      setCopied(true);
      toast.success('Copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      logger.error('Failed to copy to clipboard', error);
      toast.error('Failed to copy');
    }
  };

  // Download response
  const handleDownload = () => {
    if (!currentResponse) return;

    const blob = new Blob([responseBodyString], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `response-${Date.now()}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success('Response downloaded');
  };

  // Get status color
  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'bg-green-500';
    if (status >= 300 && status < 400) return 'bg-blue-500';
    if (status >= 400 && status < 500) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  // Simple JSON tree renderer
  const renderJsonTree = (obj: unknown, depth = 0): React.ReactNode => {
    if (obj === null) {
      return <span className="text-gray-500">null</span>;
    }

    if (typeof obj === 'string') {
      return <span className="text-green-600">"{obj}"</span>;
    }

    if (typeof obj === 'number') {
      return <span className="text-blue-600">{obj}</span>;
    }

    if (typeof obj === 'boolean') {
      return <span className="text-purple-600">{String(obj)}</span>;
    }

    if (Array.isArray(obj)) {
      if (obj.length === 0) {
        return <span className="text-gray-400">[]</span>;
      }
      return (
        <div className="ml-4">
          {obj.map((item, index) => (
            <div key={index} className="flex">
              <span className="text-gray-400 mr-2">[{index}]:</span>
              <div className="flex-1">{renderJsonTree(item, depth + 1)}</div>
            </div>
          ))}
        </div>
      );
    }

    if (typeof obj === 'object') {
      const entries = Object.entries(obj);
      if (entries.length === 0) {
        return <span className="text-gray-400">{'{}'}</span>;
      }
      return (
        <div className="ml-4">
          {entries.map(([key, value]) => (
            <div key={key} className="flex">
              <span className="text-blue-400 mr-2">"{key}":</span>
              <div className="flex-1">{renderJsonTree(value, depth + 1)}</div>
            </div>
          ))}
        </div>
      );
    }

    return <span>{String(obj)}</span>;
  };

  if (responseLoading) {
    return (
      <Card className="h-full flex flex-col">
        <CardContent className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-4">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            <p className="text-muted-foreground">Executing request...</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (responseError && !displayResponse) {
    return (
      <Card className="h-full flex flex-col">
        <CardHeader>
          <CardTitle className="text-lg font-semibold flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-destructive" />
            Error
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive">{responseError}</div>
        </CardContent>
      </Card>
    );
  }

  if (!displayResponse) {
    return (
      <Card className="h-full flex flex-col">
        <CardContent className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-4">
            <FileJson className="h-12 w-12 mx-auto text-muted-foreground" />
            <div>
              <h3 className="text-lg font-semibold">No Response</h3>
              <p className="text-muted-foreground">Execute a request to see the response here</p>
            </div>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg font-semibold">Response</CardTitle>
          <div className="flex items-center gap-2">
            <Badge className={getStatusColor(displayResponse.status_code)}>
              {displayResponse.status_code}
            </Badge>
            <div className="flex items-center gap-1 text-sm text-muted-foreground">
              <Clock className="h-4 w-4" />
              {displayResponse.response_time_ms}ms
            </div>
            <Button variant="ghost" size="icon" onClick={handleCopy}>
              {copied ? (
                <Check className="h-4 w-4 text-green-600" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
            <Button variant="ghost" size="icon" onClick={handleDownload}>
              <Download className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-auto">
        <Tabs value={viewMode} onValueChange={(value) => setViewMode(value as 'tree' | 'raw')}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="tree" className="flex items-center gap-2">
              <Eye className="h-4 w-4" />
              Tree
            </TabsTrigger>
            <TabsTrigger value="raw" className="flex items-center gap-2">
              <Code className="h-4 w-4" />
              Raw
            </TabsTrigger>
          </TabsList>

          <TabsContent value="tree" className="mt-4">
            <div className="bg-muted/30 rounded-md p-4 font-mono text-sm overflow-auto max-h-[600px]">
              {renderJsonTree(displayResponse.body)}
            </div>
          </TabsContent>

          <TabsContent value="raw" className="mt-4">
            <pre className="bg-muted/30 rounded-md p-4 font-mono text-sm overflow-auto max-h-[600px] whitespace-pre-wrap">
              {responseBodyString}
            </pre>
          </TabsContent>
        </Tabs>

        {/* Headers */}
        {Object.keys(displayResponse.headers).length > 0 && (
          <div className="mt-4">
            <h4 className="text-sm font-semibold mb-2">Headers</h4>
            <div className="bg-muted/30 rounded-md p-4 space-y-1">
              {Object.entries(displayResponse.headers).map(([key, value]) => (
                <div key={key} className="flex text-sm">
                  <span className="font-semibold text-muted-foreground w-1/3">{key}:</span>
                  <span className="flex-1 font-mono">{value}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Error message */}
        {displayResponse.error && (
          <div className="mt-4 p-4 bg-destructive/10 border border-destructive rounded-md">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-4 w-4" />
              <span className="font-semibold">Error</span>
            </div>
            <p className="mt-2 text-sm">{displayResponse.error}</p>
          </div>
        )}

        {/* MockAI Preview Toggle */}
        {protocol === 'rest' && displayResponse && (
          <div className="mt-4 p-4 bg-muted/30 rounded-md border">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-purple-600" />
                <Label htmlFor="mockai-preview" className="text-sm font-semibold">
                  MockAI Preview
                </Label>
              </div>
              <Switch
                id="mockai-preview"
                checked={mockAIPreview}
                onCheckedChange={async (checked) => {
                  setMockAIPreview(checked);
                  if (checked && !mockAIResponse) {
                    // Generate MockAI preview
                    setMockAILoading(true);
                    try {
                      await executeRestRequest(true);
                      toast.success('MockAI preview generated');
                    } catch (error) {
                      logger.error('Failed to generate MockAI preview', error);
                      toast.error('Failed to generate MockAI preview');
                      setMockAIPreview(false); // Revert toggle on error
                    } finally {
                      setMockAILoading(false);
                    }
                  }
                }}
              />
            </div>
            {mockAIPreview && (
              <div className="mt-2 text-xs text-muted-foreground">
                {mockAILoading ? (
                  <div className="flex items-center gap-2">
                    <div className="inline-block animate-spin rounded-full h-3 w-3 border-b-2 border-purple-600"></div>
                    <span>Generating AI response...</span>
                  </div>
                ) : (
                  <span>Showing AI-generated response preview</span>
                )}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
