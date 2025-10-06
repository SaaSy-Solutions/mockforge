import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';
import type { RequestLog } from '../../types';

interface LogEntryProps {
  log: RequestLog;
  isSelected?: boolean;
  onSelect?: (log: RequestLog) => void;
}

export function LogEntry({ log, isSelected = false, onSelect }: LogEntryProps) {
  const getStatusColor = (statusCode: number) => {
    if (statusCode >= 200 && statusCode < 300) return 'text-green-600 bg-green-50';
    if (statusCode >= 300 && statusCode < 400) return 'text-blue-600 bg-blue-50';
    if (statusCode >= 400 && statusCode < 500) return 'text-yellow-600 bg-yellow-50';
    if (statusCode >= 500) return 'text-red-600 bg-red-50';
    return 'text-gray-600 bg-gray-50';
  };

  const getMethodColor = (method: string) => {
    switch (method.toLowerCase()) {
      case 'get': return 'text-green-700 bg-green-100';
      case 'post': return 'text-blue-700 bg-blue-100';
      case 'put': return 'text-yellow-700 bg-yellow-100';
      case 'patch': return 'text-purple-700 bg-purple-100';
      case 'delete': return 'text-red-700 bg-red-100';
      default: return 'text-gray-700 bg-gray-100';
    }
  };

  const formatTime = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  return (
    <div
      className={cn(
        "flex items-center space-x-4 p-3 border-b hover:bg-accent/50 cursor-pointer font-mono text-sm",
        isSelected && "bg-accent border-accent-foreground/20"
      )}
      onClick={() => onSelect?.(log)}
    >
      {/* Timestamp */}
      <div className="w-20 text-xs text-muted-foreground">
        {formatTime(log.timestamp)}
      </div>

      {/* Method */}
      <div className={cn("px-2 py-1 rounded text-xs font-semibold min-w-[60px] text-center", getMethodColor(log.method))}>
        {log.method}
      </div>

      {/* Status Code */}
      <div className={cn("px-2 py-1 rounded text-xs font-semibold min-w-[50px] text-center", getStatusColor(log.status_code))}>
        {log.status_code}
      </div>

      {/* Path */}
      <div className="flex-1 truncate text-foreground">
        {log.path}
      </div>

      {/* Response Time */}
      <div className={cn(
        "text-xs px-2 py-1 rounded min-w-[60px] text-center",
        log.response_time_ms > 1000 ? "text-red-600 bg-red-50" :
        log.response_time_ms > 500 ? "text-yellow-600 bg-yellow-50" :
        "text-green-600 bg-green-50"
      )}>
        {formatDuration(log.response_time_ms)}
      </div>

      {/* Response Size */}
      <div className="text-xs text-muted-foreground min-w-[50px] text-right">
        {formatSize(log.response_size_bytes)}
      </div>

      {/* Client IP */}
      {log.client_ip && (
        <div className="text-xs text-muted-foreground min-w-[100px] text-right">
          {log.client_ip}
        </div>
      )}

      {/* Error indicator */}
      {log.error_message && (
        <div className="text-red-500 text-xs">
          ⚠️
        </div>
      )}
    </div>
  );
}