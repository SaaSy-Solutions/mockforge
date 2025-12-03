//! Condition Node Component
//!
//! Custom React Flow node component for representing conditional branching steps in a flow.

import React from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { GitBranch } from 'lucide-react';
import { cn } from '@/utils/cn';

export interface ConditionNodeData {
  id: string;
  name: string;
  expression?: string;
}

export function ConditionNode({ data, selected }: NodeProps<ConditionNodeData>) {
  const expression = data.expression || 'condition';

  return (
    <div
      className={cn(
        'relative bg-white dark:bg-gray-800 border-2 rounded-lg shadow-lg min-w-[180px]',
        selected
          ? 'border-purple-500 dark:border-purple-400'
          : 'border-purple-300 dark:border-purple-600'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col gap-2">
        {/* Header */}
        <div className="flex items-center gap-2">
          <GitBranch className="h-4 w-4 text-purple-500" />
          <div className="text-sm font-medium text-gray-900 dark:text-gray-100 flex-1 truncate">
            {data.name}
          </div>
        </div>

        {/* Expression */}
        <div className="text-xs text-gray-600 dark:text-gray-400 font-mono bg-gray-50 dark:bg-gray-700 p-2 rounded truncate">
          {expression}
        </div>
      </div>

      {/* Output handles - condition nodes have two outputs (true/false) */}
      <Handle
        type="source"
        position={Position.Bottom}
        id="true"
        className="w-3 h-3"
        style={{ left: '30%' }}
      >
        <div className="absolute -top-6 left-1/2 transform -translate-x-1/2 text-xs text-green-600 font-medium">
          True
        </div>
      </Handle>
      <Handle
        type="source"
        position={Position.Bottom}
        id="false"
        className="w-3 h-3"
        style={{ left: '70%' }}
      >
        <div className="absolute -top-6 left-1/2 transform -translate-x-1/2 text-xs text-red-600 font-medium">
          False
        </div>
      </Handle>
    </div>
  );
}
