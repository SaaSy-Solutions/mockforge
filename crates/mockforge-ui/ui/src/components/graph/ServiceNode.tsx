import React from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
import { Server, Package } from 'lucide-react';

interface ServiceNodeData {
  label: string;
  nodeType: string;
  metadata: Record<string, unknown>;
}

export function ServiceNode({ data }: NodeProps<ServiceNodeData>) {
  const endpointCount = (data.metadata?.endpointCount as number) || 0;
  const serviceId = data.metadata?.serviceId as string | undefined;

  return (
    <div className="px-6 py-4 shadow-xl rounded-xl border-2 border-info-300 dark:border-info-600 bg-gradient-to-br from-blue-50 to-blue-100 dark:from-blue-900 dark:to-blue-800 min-w-[250px]">
      <Handle type="target" position={Position.Top} className="w-4 h-4 bg-info-500" />

      <div className="flex items-center gap-3 mb-3">
        <div className="p-2 rounded-lg bg-info-500 text-white">
          <Server className="h-5 w-5" />
        </div>
        <div className="flex-1">
          <div className="font-bold text-base text-foreground">
            {data.label}
          </div>
          {serviceId && (
            <div className="text-xs text-muted-foreground font-mono">
              {serviceId}
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 text-sm text-foreground">
        <Package className="h-4 w-4" />
        <span>{endpointCount} endpoint{endpointCount !== 1 ? 's' : ''}</span>
      </div>

      <Handle type="source" position={Position.Bottom} className="w-4 h-4 bg-info-500" />
    </div>
  );
}
