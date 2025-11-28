//! State Preview Panel Component
//!
//! Real-time state visualization panel that shows active state instances
//! and their current states. Updates via WebSocket for live preview.

import React, { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import { X, RefreshCw } from 'lucide-react';
import { apiService } from '../../services/api';
import { useWebSocket } from '../../hooks/useWebSocket';
import { logger } from '@/utils/logger';

interface StatePreviewPanelProps {
  resourceType: string;
  onClose: () => void;
}

interface StateInstance {
  resource_id: string;
  current_state: string;
  resource_type: string;
  history_count: number;
  state_data: Record<string, unknown>;
}

export function StatePreviewPanel({ resourceType, onClose }: StatePreviewPanelProps) {
  const [instances, setInstances] = useState<StateInstance[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // WebSocket for real-time updates
  const { lastMessage, connected } = useWebSocket('/__mockforge/ws', {
    autoConnect: true,
  });

  // Load instances
  const loadInstances = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await apiService.getStateInstances();
      const filtered = response.instances.filter(
        (inst) => inst.resource_type === resourceType
      );
      setInstances(filtered);
    } catch (err) {
      logger.error('Failed to load state instances', err);
      setError(err instanceof Error ? err.message : 'Failed to load instances');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadInstances();
  }, [resourceType]);

  // Handle WebSocket updates
  useEffect(() => {
    if (lastMessage) {
      try {
        const event = JSON.parse(lastMessage.data);
        if (
          event.type === 'state_transitioned' &&
          event.resource_type === resourceType
        ) {
          // Update the instance in the list
          setInstances((insts) =>
            insts.map((inst) =>
              inst.resource_id === event.resource_id
                ? {
                    ...inst,
                    current_state: event.to_state,
                    state_data: event.state_data,
                    history_count: inst.history_count + 1,
                  }
                : inst
            )
          );
        } else if (
          event.type === 'state_instance_created' &&
          event.resource_type === resourceType
        ) {
          // Add new instance
          setInstances((insts) => [
            ...insts,
            {
              resource_id: event.resource_id,
              current_state: event.initial_state,
              resource_type: event.resource_type,
              history_count: 0,
              state_data: {},
            },
          ]);
        }
      } catch (err) {
        logger.error('Failed to parse WebSocket message', err);
      }
    }
  }, [lastMessage, resourceType]);

  return (
    <Card className="w-full max-h-[600px] overflow-hidden flex flex-col">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">State Preview</CardTitle>
        <div className="flex items-center gap-2">
          <Badge variant={connected ? 'success' : 'outline'} className="text-xs">
            {connected ? 'Live' : 'Offline'}
          </Badge>
          <Button
            onClick={loadInstances}
            size="sm"
            variant="ghost"
            className="h-6 w-6 p-0"
            title="Refresh"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button
            onClick={onClose}
            size="sm"
            variant="ghost"
            className="h-6 w-6 p-0"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="text-center py-4 text-sm text-gray-500">Loading...</div>
        ) : error ? (
          <div className="text-center py-4 text-sm text-red-500">{error}</div>
        ) : instances.length === 0 ? (
          <div className="text-center py-4 text-sm text-gray-500">
            No active instances
          </div>
        ) : (
          <div className="space-y-2">
            {instances.map((instance) => (
              <div
                key={instance.resource_id}
                className="p-3 border rounded-lg bg-gray-50 dark:bg-gray-800"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="font-medium text-sm">{instance.resource_id}</div>
                  <Badge variant="outline" className="text-xs">
                    {instance.current_state}
                  </Badge>
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400">
                  History: {instance.history_count} transitions
                </div>
                {Object.keys(instance.state_data).length > 0 && (
                  <div className="mt-2 text-xs">
                    <div className="font-medium mb-1">State Data:</div>
                    <pre className="bg-gray-100 dark:bg-gray-900 p-2 rounded text-xs overflow-x-auto">
                      {JSON.stringify(instance.state_data, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
