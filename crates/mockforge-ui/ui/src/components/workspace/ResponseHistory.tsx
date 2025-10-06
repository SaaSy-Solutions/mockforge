import { logger } from '@/utils/logger';
import React, { useState, useEffect } from 'react';
import { apiService } from '../../services/api';
import type { ResponseHistoryEntry } from '../../types';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../components/ui/Card';
import { Button } from '../../components/ui/button';
import { Badge } from '../../components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../components/ui/Tabs';
import { Clock, Play, AlertTriangle, CheckCircle, XCircle } from 'lucide-react';
import { toast } from 'sonner';

interface ResponseHistoryProps {
  workspaceId: string;
  requestId: string;
  requestName: string;
  onExecuteRequest?: () => void;
}

const ResponseHistory: React.FC<ResponseHistoryProps> = ({
  workspaceId,
  requestId,
  requestName,
  onExecuteRequest
}) => {
  const [history, setHistory] = useState<ResponseHistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadHistory = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await apiService.getRequestHistory(workspaceId, requestId);
      setHistory(response.history);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load history');
      toast.error('Failed to load request history');
    } finally {
      setLoading(false);
    }
  };

  const executeRequest = async () => {
    try {
      setExecuting(true);
      const response = await apiService.executeRequest(workspaceId, requestId);
      setHistory(prev => [response.execution, ...prev]);
      toast.success('Request executed successfully');
      onExecuteRequest?.();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to execute request');
    } finally {
      setExecuting(false);
    }
  };

  useEffect(() => {
    loadHistory();
  }, [workspaceId, requestId]);

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  };

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'bg-green-500';
    if (status >= 300 && status < 400) return 'bg-blue-500';
    if (status >= 400 && status < 500) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  const getStatusIcon = (status: number) => {
    if (status >= 200 && status < 300) return <CheckCircle className="w-4 h-4 text-green-500" />;
    if (status >= 300 && status < 400) return <AlertTriangle className="w-4 h-4 text-blue-500" />;
    if (status >= 400 && status < 500) return <AlertTriangle className="w-4 h-4 text-yellow-500" />;
    return <XCircle className="w-4 h-4 text-red-500" />;
  };

  if (loading && history.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="w-5 h-5" />
            Response History
          </CardTitle>
          <CardDescription>
            Execution history for {requestName}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error && history.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="w-5 h-5" />
            Response History
          </CardTitle>
          <CardDescription>
            Execution history for {requestName}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8">
            <AlertTriangle className="w-8 h-8 text-red-500 mx-auto mb-2" />
            <p className="text-red-600">{error}</p>
            <Button onClick={loadHistory} className="mt-4">
              Retry
            </Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Clock className="w-5 h-5" />
              Response History
            </CardTitle>
            <CardDescription>
              {history.length} execution{history.length !== 1 ? 's' : ''} for {requestName}
            </CardDescription>
          </div>
          <Button
            onClick={executeRequest}
            disabled={executing}
            className="flex items-center gap-2"
          >
            <Play className="w-4 h-4" />
            {executing ? 'Executing...' : 'Execute Request'}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {history.length === 0 ? (
          <div className="text-center py-8">
            <Clock className="w-8 h-8 text-gray-400 mx-auto mb-2" />
            <p className="text-gray-600">No executions yet</p>
            <p className="text-sm text-gray-500">Execute the request to see history</p>
          </div>
        ) : (
          <div className="space-y-4">
            {history.map((entry: ResponseHistoryEntry) => (
              <Card key={entry.executed_at} className="border-l-4 border-l-blue-500">
                <CardContent className="pt-4">
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-3">
                      {getStatusIcon(entry.response_status_code)}
                      <div>
                        <div className="flex items-center gap-2">
                          <Badge variant="default" className={getStatusColor(entry.response_status_code)}>
                            {entry.response_status_code}
                          </Badge>
                          <span className="font-medium">{entry.request_method}</span>
                          <span className="text-gray-600">{entry.request_path}</span>
                        </div>
                        <div className="text-sm text-gray-500 mt-1">
                          {new Date(entry.executed_at).toLocaleString()}
                        </div>
                      </div>
                    </div>
                    <div className="text-right text-sm text-gray-500">
                      <div>{formatDuration(entry.response_time_ms)}</div>
                      <div>{formatSize(entry.response_size_bytes)}</div>
                    </div>
                  </div>

                  <Tabs defaultValue="response" className="w-full">
                    <TabsList className="grid w-full grid-cols-3">
                      <TabsTrigger value="response">Response</TabsTrigger>
                      <TabsTrigger value="request">Request</TabsTrigger>
                      <TabsTrigger value="headers">Headers</TabsTrigger>
                    </TabsList>

                    <TabsContent value="response" className="mt-3">
                      {entry.error_message ? (
                        <div className="bg-red-50 border border-red-200 rounded p-3">
                          <div className="flex items-center gap-2 text-red-700 font-medium mb-2">
                            <XCircle className="w-4 h-4" />
                            Error
                          </div>
                          <pre className="text-red-600 text-sm whitespace-pre-wrap">
                            {entry.error_message}
                          </pre>
                        </div>
                      ) : (
                        <div className="bg-gray-50 border rounded p-3">
                          <pre className="text-sm whitespace-pre-wrap overflow-x-auto">
                            {entry.response_body || '(empty response)'}
                          </pre>
                        </div>
                      )}
                    </TabsContent>

                    <TabsContent value="request" className="mt-3">
                      <div className="space-y-3">
                        {entry.request_body && (
                          <div>
                            <h4 className="font-medium text-sm mb-2">Request Body</h4>
                            <div className="bg-gray-50 border rounded p-3">
                              <pre className="text-sm whitespace-pre-wrap overflow-x-auto">
                                {entry.request_body}
                              </pre>
                            </div>
                          </div>
                        )}
                        {Object.keys(entry.request_headers).length > 0 && (
                          <div>
                            <h4 className="font-medium text-sm mb-2">Request Headers</h4>
                            <div className="bg-gray-50 border rounded p-3">
                              {Object.entries(entry.request_headers as Record<string, unknown>).map(([key, value]) => (
                                <div key={key} className="text-sm">
                                  <span className="font-medium">{key}:</span> {String(value)}
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    </TabsContent>

                    <TabsContent value="headers" className="mt-3">
                      <div className="space-y-3">
                        <div>
                          <h4 className="font-medium text-sm mb-2">Response Headers</h4>
                          <div className="bg-gray-50 border rounded p-3">
                            {Object.keys(entry.response_headers as Record<string, unknown>).length > 0 ? (
                              Object.entries(entry.response_headers as Record<string, unknown>).map(([key, value]) => (
                                <div key={key} className="text-sm">
                                  <span className="font-medium">{key}:</span> {String(value)}
                                </div>
                              ))
                            ) : (
                              <span className="text-gray-500 text-sm">(no headers)</span>
                            )}
                          </div>
                        </div>
                      </div>
                    </TabsContent>
                  </Tabs>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
};

export default ResponseHistory;
