//! Loop Node Component
//!
//! Custom React Flow node component for representing loop steps in a flow.

import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { Repeat } from 'lucide-react';
import { cn } from '@/utils/cn';

export interface LoopNodeData {
  id: string;
  name: string;
  iterations?: number;
  condition?: string;
}

export function LoopNode({ data, selected }: NodeProps<LoopNodeData>) {
  const iterations = data.iterations;
  const condition = data.condition;

  return (
    <div
      className={cn(
        'relative bg-white dark:bg-gray-800 border-2 rounded-lg shadow-lg min-w-[180px]',
        selected
          ? 'border-indigo-500 dark:border-indigo-400'
          : 'border-indigo-300 dark:border-indigo-600'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Repeat className="h-4 w-4 text-indigo-500" />
          <div className="text-sm font-medium text-gray-900 dark:text-gray-100 flex-1 truncate">
            {data.name}
          </div>
        </div>

        {/* Loop info */}
        {iterations && (
          <Badge variant="outline" className="text-xs w-fit">
            {iterations} iterations
          </Badge>
        )}
        {condition && (
          <div className="text-xs text-gray-600 dark:text-gray-400 font-mono bg-gray-50 dark:bg-gray-700 p-2 rounded truncate">
            {condition}
          </div>
        )}
      </div>

      {/* Output handles - loop nodes have two outputs (continue/exit) */}
      <Handle
        type="source"
        position={Position.Bottom}
        id="continue"
        className="w-3 h-3"
        style={{ left: '30%' }}
      >
        <div className="absolute -top-6 left-1/2 transform -translate-x-1/2 text-xs text-blue-600 font-medium">
          Continue
        </div>
      </Handle>
      <Handle
        type="source"
        position={Position.Bottom}
        id="exit"
        className="w-3 h-3"
        style={{ left: '70%' }}
      >
        <div className="absolute -top-6 left-1/2 transform -translate-x-1/2 text-xs text-gray-600 font-medium">
          Exit
        </div>
      </Handle>
    </div>
  );
}
