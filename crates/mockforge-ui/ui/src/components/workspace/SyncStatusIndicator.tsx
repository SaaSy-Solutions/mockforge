import { logger } from '@/utils/logger';
import React from 'react';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/button';
import { Tooltip } from '../ui/Tooltip';
import { RefreshCw, AlertCircle, CheckCircle, Clock, Play, Square } from 'lucide-react';
import type { SyncStatus } from '../../types';

interface SyncStatusIndicatorProps {
  status: SyncStatus;
  onSyncNow?: () => void;
  onStopSync?: () => void;
  loading?: boolean;
}

export function SyncStatusIndicator({
  status,
  onSyncNow,
  onStopSync,
  loading = false
}: SyncStatusIndicatorProps) {
  const getStatusIcon = () => {
    switch (status.status.toLowerCase()) {
      case 'syncing':
        return <RefreshCw className="h-4 w-4 animate-spin text-blue-500" />;
      case 'success':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'error':
        return <AlertCircle className="h-4 w-4 text-red-500" />;
      case 'idle':
        return <Clock className="h-4 w-4 text-gray-500" />;
      default:
        return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  const getStatusBadgeVariant = () => {
    switch (status.status.toLowerCase()) {
      case 'syncing':
        return 'info';
      case 'success':
        return 'success';
      case 'error':
        return 'danger';
      case 'idle':
        return 'secondary';
      default:
        return 'secondary';
    }
  };

  const getStatusText = () => {
    switch (status.status.toLowerCase()) {
      case 'syncing':
        return 'Syncing...';
      case 'success':
        return 'Synced';
      case 'error':
        return 'Sync Error';
      case 'idle':
        return 'Idle';
      default:
        return 'Unknown';
    }
  };

  const formatLastSync = (timestamp?: string) => {
    if (!timestamp) return 'Never';

    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
  };

  return (
    <div className="flex items-center gap-3 p-3 bg-bg-secondary rounded-lg">
      <div className="flex items-center gap-2">
        {getStatusIcon()}
        <div className="flex flex-col">
          <div className="flex items-center gap-2">
            <Badge variant={getStatusBadgeVariant()}>
              {getStatusText()}
            </Badge>
            {status.target_directory && (
              <Tooltip content={`Syncing to: ${status.target_directory}`}>
                <code className="text-xs text-gray-600 dark:text-gray-400 truncate max-w-32">
                  {status.target_directory.split('/').pop()}
                </code>
              </Tooltip>
            )}
          </div>
          {status.last_sync && (
            <span className="text-xs text-gray-600 dark:text-gray-400">
              Last sync: {formatLastSync(status.last_sync)}
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-1 ml-auto">
        {status.enabled && status.sync_direction === 'Bidirectional' && status.realtime_monitoring && (
          <Tooltip content="Real-time monitoring active">
            <div className="flex items-center gap-1 text-xs text-green-600">
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
              Live
            </div>
          </Tooltip>
        )}

        {onSyncNow && status.sync_direction !== 'Bidirectional' && (
          <Tooltip content="Sync now">
            <Button
              variant="outline"
              size="sm"
              onClick={onSyncNow}
              disabled={loading || status.status === 'syncing'}
              className="h-8 w-8 p-0"
            >
              <Play className="h-3 w-3" />
            </Button>
          </Tooltip>
        )}

        {onStopSync && status.status === 'syncing' && (
          <Tooltip content="Stop sync">
            <Button
              variant="outline"
              size="sm"
              onClick={onStopSync}
              disabled={loading}
              className="h-8 w-8 p-0"
            >
              <Square className="h-3 w-3" />
            </Button>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
