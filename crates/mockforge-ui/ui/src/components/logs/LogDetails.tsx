import React from 'react';
import { Button } from '../ui/button';
import type { RequestLog } from '../../types';

interface LogDetailsProps {
  log: RequestLog;
  onClose: () => void;
}

export function LogDetails({ log, onClose }: LogDetailsProps) {
  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms} ms`;
    return `${(ms / 1000).toFixed(2)} s`;
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getStatusColor = (statusCode: number) => {
    if (statusCode >= 200 && statusCode < 300) return 'text-green-600';
    if (statusCode >= 300 && statusCode < 400) return 'text-blue-600';
    if (statusCode >= 400 && statusCode < 500) return 'text-yellow-600';
    if (statusCode >= 500) return 'text-red-600';
    return 'text-gray-600';
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg border shadow-lg w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="p-4 border-b flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Request Details</h2>
            <p className="text-sm text-muted-foreground">
              {log.method} {log.path} • {formatTimestamp(log.timestamp)}
            </p>
          </div>
          <Button variant="outline" onClick={onClose} size="sm">
            Close
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4 space-y-6">
          {/* Request Overview */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="space-y-1">
              <div className="text-xs text-muted-foreground">Status Code</div>
              <div className={`text-lg font-semibold ${getStatusColor(log.status_code)}`}>
                {log.status_code}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-xs text-muted-foreground">Response Time</div>
              <div className="text-lg font-semibold">
                {formatDuration(log.response_time_ms)}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-xs text-muted-foreground">Response Size</div>
              <div className="text-lg font-semibold">
                {formatSize(log.response_size_bytes)}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-xs text-muted-foreground">Request ID</div>
              <div className="text-sm font-mono">
                {log.id}
              </div>
            </div>
          </div>

          {/* Client Information */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {log.client_ip && (
              <div className="space-y-2">
                <h3 className="font-semibold text-sm">Client IP</h3>
                <div className="p-3 bg-muted/50 rounded border font-mono text-sm">
                  {log.client_ip}
                </div>
              </div>
            )}
            {log.user_agent && (
              <div className="space-y-2">
                <h3 className="font-semibold text-sm">User Agent</h3>
                <div className="p-3 bg-muted/50 rounded border font-mono text-sm break-all">
                  {log.user_agent}
                </div>
              </div>
            )}
          </div>

          {/* Request Headers */}
          {Object.keys(log.headers).length > 0 && (
            <div className="space-y-2">
              <h3 className="font-semibold text-sm">Request Headers</h3>
              <div className="border rounded">
                <div className="p-3 bg-muted/50 border-b">
                  <div className="grid grid-cols-2 gap-4 text-xs font-semibold text-muted-foreground">
                    <div>Header</div>
                    <div>Value</div>
                  </div>
                </div>
                <div className="max-h-64 overflow-auto">
                  {Object.entries(log.headers).map(([key, value]) => (
                    <div key={key} className="p-3 border-b last:border-b-0 grid grid-cols-2 gap-4 text-sm font-mono">
                      <div className="font-semibold">{key}</div>
                      <div className="break-all text-muted-foreground">{value}</div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Error Message */}
          {log.error_message && (
            <div className="space-y-2">
              <h3 className="font-semibold text-sm text-destructive">Error Message</h3>
              <div className="p-3 bg-destructive/10 border border-destructive/20 rounded">
                <pre className="text-sm text-destructive whitespace-pre-wrap font-mono">
                  {log.error_message}
                </pre>
              </div>
            </div>
          )}

          {/* Timing Breakdown (if available) */}
          <div className="space-y-2">
            <h3 className="font-semibold text-sm">Timing Information</h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 p-3 bg-muted/50 rounded border">
              <div className="text-center">
                <div className="text-xs text-muted-foreground">Total Time</div>
                <div className="font-semibold">{formatDuration(log.response_time_ms)}</div>
              </div>
              <div className="text-center">
                <div className="text-xs text-muted-foreground">Timestamp</div>
                <div className="font-mono text-sm">{formatTimestamp(log.timestamp)}</div>
              </div>
              <div className="text-center">
                <div className="text-xs text-muted-foreground">Request ID</div>
                <div className="font-mono text-xs">{log.id}</div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 border-t bg-muted/50">
          <div className="flex justify-between items-center text-xs text-muted-foreground">
            <div>Raw log entry for request {log.id}</div>
            <div className="flex space-x-4">
              <button className="hover:text-foreground">Copy as JSON</button>
              <button className="hover:text-foreground">Copy as cURL</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}