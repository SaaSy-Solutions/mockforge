//! Parallel Node Component
//!
//! Custom React Flow node component for representing parallel execution steps in a flow.

import React from 'react';
import { Handle, Position, NodeProps } from 'react-flow-renderer';
import { Badge } from '../ui/Badge';
import { Layers } from 'lucide-react';
import { cn } from '@/utils/cn';

export interface ParallelNodeData {
  id: string;
  name: string;
  branches?: number;
}

export function ParallelNode({ data, selected }: NodeProps<ParallelNodeData>) {
  const branches = data.branches || 2;

  return (
    <div
      className={cn(
        'relative bg-white dark:bg-gray-800 border-2 rounded-lg shadow-lg min-w-[180px]',
        selected
          ? 'border-teal-500 dark:border-teal-400'
          : 'border-teal-300 dark:border-teal-600'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Layers className="h-4 w-4 text-teal-500" />
          <div className="text-sm font-medium text-gray-900 dark:text-gray-100 flex-1 truncate">
            {data.name}
          </div>
        </div>

        {/* Branch count */}
        <Badge variant="outline" className="text-xs w-fit">
          {branches} branches
        </Badge>
      </div>

      {/* Output handles - parallel nodes have multiple outputs for each branch */}
      {Array.from({ length: branches }).map((_, index) => (
        <Handle
          key={`branch-${index}`}
          type="source"
          position={Position.Bottom}
          id={`branch-${index}`}
          className="w-3 h-3"
          style={{ left: `${((index + 1) * 100) / (branches + 1)}%` }}
        >
          <div className="absolute -top-6 left-1/2 transform -translate-x-1/2 text-xs text-teal-600 font-medium">
            Branch {index + 1}
          </div>
        </Handle>
      ))}
    </div>
  );
}

