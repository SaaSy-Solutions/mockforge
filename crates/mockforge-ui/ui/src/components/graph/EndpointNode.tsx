import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { Server, Zap, Globe, MessageSquare, Database, Mail, Radio } from 'lucide-react';
import type { GraphNode } from '../../types/graph';

interface EndpointNodeData {
  label: string;
  nodeType: string;
  protocol?: string;
  currentState?: string;
  metadata: Record<string, unknown>;
}

const protocolIcons: Record<string, React.ReactNode> = {
  http: <Globe className="h-4 w-4" />,
  grpc: <Zap className="h-4 w-4" />,
  websocket: <MessageSquare className="h-4 w-4" />,
  graphql: <Database className="h-4 w-4" />,
  mqtt: <Radio className="h-4 w-4" />,
  smtp: <Mail className="h-4 w-4" />,
  kafka: <Database className="h-4 w-4" />,
  amqp: <MessageSquare className="h-4 w-4" />,
  ftp: <Server className="h-4 w-4" />,
};

const protocolColors: Record<string, string> = {
  http: 'bg-blue-500',
  grpc: 'bg-green-500',
  websocket: 'bg-purple-500',
  graphql: 'bg-pink-500',
  mqtt: 'bg-orange-500',
  smtp: 'bg-yellow-500',
  kafka: 'bg-red-500',
  amqp: 'bg-indigo-500',
  ftp: 'bg-gray-500',
};

const stateColors: Record<string, string> = {
  pending: 'bg-yellow-500',
  active: 'bg-green-500',
  inactive: 'bg-gray-500',
  error: 'bg-red-500',
  processing: 'bg-blue-500',
};

export function EndpointNode({ data }: NodeProps<EndpointNodeData>) {
  const protocol = data.protocol?.toLowerCase() || 'http';
  const state = data.currentState?.toLowerCase();
  const method = data.metadata?.method as string | undefined;
  const path = data.metadata?.path as string | undefined;

  return (
    <div className="px-4 py-3 shadow-lg rounded-lg border-2 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 min-w-[200px]">
      <Handle type="target" position={Position.Top} className="w-3 h-3" />

      <div className="flex items-center gap-2 mb-2">
        <div className={`p-1.5 rounded ${protocolColors[protocol] || 'bg-gray-500'} text-white`}>
          {protocolIcons[protocol] || <Server className="h-4 w-4" />}
        </div>
        <div className="flex-1 min-w-0">
          <div className="font-semibold text-sm text-gray-900 dark:text-gray-100 truncate">
            {data.label}
          </div>
          {method && (
            <div className="text-xs text-gray-500 dark:text-gray-400 font-mono">
              {method} {path || ''}
            </div>
          )}
        </div>
      </div>

      {state && (
        <div className="flex items-center gap-2 mt-2">
          <div className={`w-2 h-2 rounded-full ${stateColors[state] || 'bg-gray-400'} animate-pulse`} />
          <span className="text-xs text-gray-600 dark:text-gray-400 capitalize">{state}</span>
        </div>
      )}

      {protocol && (
        <Badge variant="outline" className="mt-2 text-xs">
          {protocol.toUpperCase()}
        </Badge>
      )}

      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
    </div>
  );
}
