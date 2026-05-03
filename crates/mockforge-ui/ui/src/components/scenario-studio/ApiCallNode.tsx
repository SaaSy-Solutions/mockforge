//! API Call Node Component
//!
//! Custom React Flow node component for representing API call steps in a flow.

import React from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { Globe } from 'lucide-react';
import { cn } from '@/utils/cn';

export interface ApiCallNodeData {
  id: string;
  name: string;
  method?: string;
  endpoint?: string;
  expectedStatus?: number;
}

export function ApiCallNode({ data, selected }: NodeProps<ApiCallNodeData>) {
  const method = data.method || 'GET';
  const endpoint = data.endpoint || '/api/endpoint';
  const status = data.expectedStatus;

  // Color coding by HTTP method
  const methodColors: Record<string, string> = {
    GET: 'bg-info-100 text-info-700 border-info-300',
    POST: 'bg-success-100 text-success-700 border-success-300',
    PUT: 'bg-warning-100 text-warning-700 border-warning-300',
    PATCH: 'bg-orange-100 text-orange-800 border-orange-300',
    DELETE: 'bg-danger-100 text-danger-700 border-danger-300',
  };

  const methodColor = methodColors[method.toUpperCase()] || 'bg-gray-100 text-gray-800 border-gray-300';

  return (
    <div
      className={cn(
        'relative bg-card border-2 rounded-lg shadow-lg min-w-[200px]',
        selected
          ? 'border-info dark:border-info-400'
          : 'border-border'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Globe className="h-4 w-4 text-muted-foreground" />
          <div className="text-sm font-medium text-foreground flex-1 truncate">
            {data.name}
          </div>
        </div>

        {/* Method badge */}
        <div className="flex items-center gap-2">
          <Badge className={cn('text-xs font-mono', methodColor)}>
            {method}
          </Badge>
          {status && (
            <Badge variant="outline" className="text-xs">
              {status}
            </Badge>
          )}
        </div>

        {/* Endpoint */}
        <div className="text-xs text-muted-foreground font-mono truncate">
          {endpoint}
        </div>
      </div>

      {/* Output handles */}
      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
      <Handle type="source" position={Position.Right} className="w-3 h-3" />
    </div>
  );
}
