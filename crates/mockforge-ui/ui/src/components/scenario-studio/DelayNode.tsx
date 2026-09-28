//! Delay Node Component
//!
//! Custom React Flow node component for representing delay steps in a flow.

import { Handle, Position } from '@xyflow/react';
import type { Node, NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { Clock } from 'lucide-react';
import { cn } from '@/utils/cn';

export type DelayNodeData = {
  id: string;
  name: string;
  delayMs?: number;
};

export type DelayNodeType = Node<DelayNodeData, 'delay'>;

export function DelayNode({ data, selected }: NodeProps<DelayNodeType>) {
  const delayMs = data.delayMs || 0;
  const delaySeconds = (delayMs / 1000).toFixed(1);

  return (
    <div
      className={cn(
        'relative bg-card border-2 rounded-lg shadow-lg min-w-[150px]',
        selected
          ? 'border-warning dark:border-warning-400'
          : 'border-warning-300 dark:border-warning-600'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2 items-center">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Clock className="h-4 w-4 text-warning-500" />
          <div className="text-sm font-medium text-foreground">
            {data.name}
          </div>
        </div>

        {/* Delay duration */}
        <Badge variant="outline" className="text-xs">
          {delayMs > 0 ? `${delaySeconds}s` : 'No delay'}
        </Badge>
      </div>

      {/* Output handles */}
      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
      <Handle type="source" position={Position.Right} className="w-3 h-3" />
    </div>
  );
}
