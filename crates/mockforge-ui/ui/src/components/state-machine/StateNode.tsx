//! State Node Component
//!
//! Custom React Flow node component for representing states in a state machine.
//! Supports editing state labels and marking initial/final states.

import React, { useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
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
        'relative bg-card border-2 rounded-lg shadow-lg min-w-[120px] min-h-[60px]',
        selected
          ? 'border-info dark:border-info-400'
          : 'border-border',
        isInitial && 'border-success dark:border-success-400',
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
            className="text-sm font-medium text-foreground cursor-text select-none"
          >
            {label}
          </div>
        )}

        <div className="text-xs text-muted-foreground">{data.state}</div>
      </div>

      {/* Output handles */}
      <Handle type="source" position={Position.Bottom} className="w-3 h-3" />
      <Handle type="source" position={Position.Right} className="w-3 h-3" />
    </div>
  );
}
