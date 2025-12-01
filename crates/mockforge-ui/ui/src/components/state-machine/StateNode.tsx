//! State Node Component
//!
//! Custom React Flow node component for representing states in a state machine.
//! Supports editing state labels and marking initial/final states.

import React, { useState } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { Input } from '../ui/input';
import { Circle, CheckCircle2 } from 'lucide-react';
import { cn } from '@/utils/cn';

interface StateNodeData {
  label: string;
  state: string;
  isInitial?: boolean;
  isFinal?: boolean;
}

export function StateNode({ data, selected }: NodeProps<StateNodeData>) {
  const [isEditing, setIsEditing] = useState(false);
  const [label, setLabel] = useState(data.label || data.state);

  const handleLabelChange = (newLabel: string) => {
    setLabel(newLabel);
    // Update node data (this would typically be handled by the parent)
    data.label = newLabel;
  };

  const handleBlur = () => {
    setIsEditing(false);
  };

  const handleDoubleClick = () => {
    setIsEditing(true);
  };

  const isInitial = data.isInitial || false;
  const isFinal = data.isFinal || false;

  return (
    <div
      className={cn(
        'relative bg-white dark:bg-gray-800 border-2 rounded-lg shadow-lg min-w-[120px] min-h-[60px]',
        selected
          ? 'border-blue-500 dark:border-blue-400'
          : 'border-gray-300 dark:border-gray-600',
        isInitial && 'border-green-500 dark:border-green-400',
        isFinal && 'border-purple-500 dark:border-purple-400'
      )}
    >
      {/* Input handles */}
      <Handle type="target" position={Position.Top} className="w-3 h-3" />
      <Handle type="target" position={Position.Left} className="w-3 h-3" />

      {/* Node content */}
      <div className="p-3 flex flex-col items-center justify-center gap-1">
        {isInitial && (
          <Badge variant="success" className="text-xs mb-1">
            <Circle className="h-3 w-3 mr-1" />
            Initial
          </Badge>
        )}
        {isFinal && (
          <Badge variant="default" className="text-xs mb-1">
            <CheckCircle2 className="h-3 w-3 mr-1" />
            Final
          </Badge>
        )}

        {isEditing ? (
          <Input
            value={label}
            onChange={(e) => handleLabelChange(e.target.value)}
            onBlur={handleBlur}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                handleBlur();
              }
            }}
            className="text-sm text-center h-6 px-2"
            autoFocus
          />
        ) : (
          <div
            onDoubleClick={handleDoubleClick}
            className="text-sm font-medium text-gray-900 dark:text-gray-100 cursor-text select-none"
          >
            {label}
          </div>
        )}

        <div className="text-xs text-gray-500 dark:text-gray-400">{data.state}</div>
      </div>

      {/* Output handles */}
      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
      <Handle type="source" position={Position.Right} className="w-3 h-3" />
    </div>
  );
}
