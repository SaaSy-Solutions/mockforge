import { logger } from '@/utils/logger';
import React, { useState, useEffect, useCallback } from 'react';
import { Play, Loader2, Plus, Trash2, Code2 } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { usePlaygroundStore } from '../../stores/usePlaygroundStore';
import { toast } from 'sonner';

/**
 * Request Panel Component
 *
 * Provides an interactive interface for building and executing REST and GraphQL requests.
 * Features:
 * - REST form builder with method, path, headers, and body
 * - GraphQL query editor with variables support
 * - Endpoint autocomplete
 * - Request execution
 */
export function RequestPanel() {
  const {
    protocol,
    setProtocol,
    restRequest,
    setRestRequest,
    graphQLRequest,
    setGraphQLRequest,
    executeRestRequest,
    executeGraphQLRequest,
    responseLoading,
    endpoints,
    loadEndpoints,
  } = usePlaygroundStore();

  const [headerKeys, setHeaderKeys] = useState<string[]>([]);
  const [newHeaderKey, setNewHeaderKey] = useState('');
  const [newHeaderValue, setNewHeaderValue] = useState('');

  // Load endpoints on mount
  useEffect(() => {
    loadEndpoints();
  }, [loadEndpoints]);

  // Initialize header keys from restRequest.headers
  useEffect(() => {
    if (protocol === 'rest') {
      setHeaderKeys(Object.keys(restRequest.headers));
    }
  }, [protocol, restRequest.headers]);

  // Handle REST request execution
  const handleExecuteRest = useCallback(async () => {
    if (!restRequest.path) {
      toast.error('Please enter a request path');
      return;
    }

    try {
      await executeRestRequest();
      toast.success('Request executed successfully');
    } catch (error) {
      logger.error('Failed to execute REST request', error);
      toast.error('Failed to execute request');
    }
  }, [restRequest.path, executeRestRequest]);

  // Handle GraphQL request execution
  const handleExecuteGraphQL = useCallback(async () => {
    if (!graphQLRequest.query.trim()) {
      toast.error('Please enter a GraphQL query');
      return;
    }

    try {
      await executeGraphQLRequest();
      toast.success('Query executed successfully');
    } catch (error) {
      logger.error('Failed to execute GraphQL query', error);
      toast.error('Failed to execute query');
    }
  }, [graphQLRequest.query, executeGraphQLRequest]);

  // Add header
  const handleAddHeader = () => {
    if (newHeaderKey && newHeaderValue) {
      const updatedHeaders = { ...restRequest.headers, [newHeaderKey]: newHeaderValue };
      setRestRequest({ headers: updatedHeaders });
      setHeaderKeys([...headerKeys, newHeaderKey]);
      setNewHeaderKey('');
      setNewHeaderValue('');
    }
  };

  // Remove header
  const handleRemoveHeader = (key: string) => {
    const updatedHeaders = { ...restRequest.headers };
    delete updatedHeaders[key];
    setRestRequest({ headers: updatedHeaders });
    setHeaderKeys(headerKeys.filter((k) => k !== key));
  };

  // Update header value
  const handleUpdateHeader = (key: string, value: string) => {
    const updatedHeaders = { ...restRequest.headers, [key]: value };
    setRestRequest({ headers: updatedHeaders });
  };

  // Get autocomplete suggestions for path
  const getPathSuggestions = () => {
    if (protocol === 'rest') {
      return endpoints
        .filter((e) => e.protocol === 'rest')
        .map((e) => e.path)
        .filter((path, index, self) => self.indexOf(path) === index);
    }
    return [];
  };

  // Handle path autocomplete
  const handlePathChange = (value: string) => {
    setRestRequest({ path: value });

    // Try to find matching endpoint and set method
    const matchingEndpoint = endpoints.find(
      (e) => e.protocol === 'rest' && e.path === value
    );
    if (matchingEndpoint) {
      setRestRequest({ method: matchingEndpoint.method.toUpperCase() });
    }
  };

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg font-semibold">Request</CardTitle>
          <div className="flex items-center gap-2">
            <Select value={protocol} onValueChange={(value: 'rest' | 'graphql') => setProtocol(value)}>
              <SelectTrigger className="w-32">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="rest">REST</SelectItem>
                <SelectItem value="graphql">GraphQL</SelectItem>
              </SelectContent>
            </Select>
            <Button
              onClick={protocol === 'rest' ? handleExecuteRest : handleExecuteGraphQL}
              disabled={responseLoading}
              size="sm"
              className="gap-2"
            >
              {responseLoading ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Executing...
                </>
              ) : (
                <>
                  <Play className="h-4 w-4" />
                  Execute
                </>
              )}
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-auto space-y-4">
        {protocol === 'rest' ? (
          // REST Request Form
          <div className="space-y-4">
            {/* Method and Path */}
            <div className="grid grid-cols-12 gap-2">
              <div className="col-span-3">
                <Label htmlFor="method">Method</Label>
                <Select
                  value={restRequest.method}
                  onValueChange={(value) => setRestRequest({ method: value })}
                >
                  <SelectTrigger id="method">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="GET">GET</SelectItem>
                    <SelectItem value="POST">POST</SelectItem>
                    <SelectItem value="PUT">PUT</SelectItem>
                    <SelectItem value="DELETE">DELETE</SelectItem>
                    <SelectItem value="PATCH">PATCH</SelectItem>
                    <SelectItem value="HEAD">HEAD</SelectItem>
                    <SelectItem value="OPTIONS">OPTIONS</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="col-span-9">
                <Label htmlFor="path">Path</Label>
                <Input
                  id="path"
                  value={restRequest.path}
                  onChange={(e) => handlePathChange(e.target.value)}
                  placeholder="/api/users"
                  list="path-suggestions"
                />
                <datalist id="path-suggestions">
                  {getPathSuggestions().map((path) => (
                    <option key={path} value={path} />
                  ))}
                </datalist>
              </div>
            </div>

            {/* Base URL (optional) */}
            <div>
              <Label htmlFor="base_url">Base URL (optional)</Label>
              <Input
                id="base_url"
                value={restRequest.base_url || ''}
                onChange={(e) => setRestRequest({ base_url: e.target.value || undefined })}
                placeholder="http://localhost:3000"
              />
            </div>

            {/* Headers */}
            <div>
              <Label>Headers</Label>
              <div className="space-y-2 mt-2">
                {headerKeys.map((key) => (
                  <div key={key} className="flex gap-2">
                    <Input
                      value={key}
                      readOnly
                      className="flex-1 font-mono text-sm"
                    />
                    <Input
                      value={restRequest.headers[key] || ''}
                      onChange={(e) => handleUpdateHeader(key, e.target.value)}
                      placeholder="Value"
                      className="flex-2"
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveHeader(key)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
                <div className="flex gap-2">
                  <Input
                    value={newHeaderKey}
                    onChange={(e) => setNewHeaderKey(e.target.value)}
                    placeholder="Header name"
                    className="flex-1"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        handleAddHeader();
                      }
                    }}
                  />
                  <Input
                    value={newHeaderValue}
                    onChange={(e) => setNewHeaderValue(e.target.value)}
                    placeholder="Header value"
                    className="flex-2"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        handleAddHeader();
                      }
                    }}
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={handleAddHeader}
                    disabled={!newHeaderKey || !newHeaderValue}
                  >
                    <Plus className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </div>

            {/* Body */}
            {(restRequest.method === 'POST' ||
              restRequest.method === 'PUT' ||
              restRequest.method === 'PATCH') && (
              <div>
                <Label htmlFor="body">Body (JSON)</Label>
                <Textarea
                  id="body"
                  value={restRequest.body}
                  onChange={(e) => setRestRequest({ body: e.target.value })}
                  placeholder='{"key": "value"}'
                  className="font-mono text-sm min-h-[200px]"
                />
              </div>
            )}
          </div>
        ) : (
          // GraphQL Request Form
          <div className="space-y-4">
            {/* Query Editor */}
            <div>
              <Label htmlFor="graphql-query">Query</Label>
              <Textarea
                id="graphql-query"
                value={graphQLRequest.query}
                onChange={(e) => setGraphQLRequest({ query: e.target.value })}
                placeholder={`query {
  user(id: "1") {
    id
    name
    email
  }
}`}
                className="font-mono text-sm min-h-[300px]"
              />
            </div>

            {/* Variables Editor */}
            <div>
              <Label htmlFor="graphql-variables">Variables (JSON)</Label>
              <Textarea
                id="graphql-variables"
                value={JSON.stringify(graphQLRequest.variables, null, 2)}
                onChange={(e) => {
                  try {
                    const vars = JSON.parse(e.target.value || '{}');
                    setGraphQLRequest({ variables: vars });
                  } catch {
                    // Invalid JSON, ignore for now
                  }
                }}
                placeholder='{"id": "1"}'
                className="font-mono text-sm min-h-[150px]"
              />
            </div>

            {/* Operation Name */}
            <div>
              <Label htmlFor="operation-name">Operation Name (optional)</Label>
              <Input
                id="operation-name"
                value={graphQLRequest.operation_name || ''}
                onChange={(e) =>
                  setGraphQLRequest({ operation_name: e.target.value || undefined })
                }
                placeholder="GetUser"
              />
            </div>

            {/* Base URL (optional) */}
            <div>
              <Label htmlFor="graphql-base_url">Base URL (optional)</Label>
              <Input
                id="graphql-base_url"
                value={graphQLRequest.base_url || ''}
                onChange={(e) => setGraphQLRequest({ base_url: e.target.value || undefined })}
                placeholder="http://localhost:4000"
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
