//! API Call Node Component
//!
//! Custom React Flow node component for representing API call steps in a flow.

import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
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
    GET: 'bg-blue-100 text-blue-800 border-blue-300',
    POST: 'bg-green-100 text-green-800 border-green-300',
    PUT: 'bg-yellow-100 text-yellow-800 border-yellow-300',
    PATCH: 'bg-orange-100 text-orange-800 border-orange-300',
    DELETE: 'bg-red-100 text-red-800 border-red-300',
  };

  const methodColor = methodColors[method.toUpperCase()] || 'bg-gray-100 text-gray-800 border-gray-300';

  return (
    <div
      className={cn(
        'relative bg-white dark:bg-gray-800 border-2 rounded-lg shadow-lg min-w-[200px]',
        selected
          ? 'border-blue-500 dark:border-blue-400'
          : 'border-gray-300 dark:border-gray-600'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Globe className="h-4 w-4 text-gray-500" />
          <div className="text-sm font-medium text-gray-900 dark:text-gray-100 flex-1 truncate">
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
        <div className="text-xs text-gray-600 dark:text-gray-400 font-mono truncate">
          {endpoint}
        </div>
      </div>

      {/* Output handles */}
      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
      <Handle type="source" position={Position.Right} className="w-3 h-3" />
    </div>
  );
}
