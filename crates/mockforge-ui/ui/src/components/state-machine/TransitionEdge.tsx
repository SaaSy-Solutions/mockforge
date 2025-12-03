//! Transition Edge Component
//!
//! Custom React Flow edge component for representing state transitions.
//! Displays condition expressions and supports editing.

import React from 'react';
import { EdgeProps, getBezierPath } from '@xyflow/react';
import { Badge } from '../ui/Badge';
import { cn } from '@/utils/cn';

interface TransitionEdgeData {
  condition?: string;
  subScenarioRef?: string;
  probability?: number;
}

export function TransitionEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  markerEnd,
  data,
  selected,
}: EdgeProps<TransitionEdgeData>) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const condition = data?.condition || '';
  const hasCondition = condition.length > 0;

  return (
    <>
      <path
        id={id}
        style={style}
        className={cn(
          'fill-none stroke-2',
          selected
            ? 'stroke-blue-500 dark:stroke-blue-400'
            : hasCondition
            ? 'stroke-green-500 dark:stroke-green-400'
            : 'stroke-gray-400 dark:stroke-gray-500'
        )}
        d={edgePath}
        markerEnd={markerEnd}
      />
      {hasCondition && (
        <foreignObject
          width={200}
          height={40}
          x={labelX - 100}
          y={labelY - 20}
          className="pointer-events-none"
        >
          <div className="flex items-center justify-center h-full">
            <Badge
              variant="outline"
              className="bg-white dark:bg-gray-800 text-xs px-2 py-1 border-green-500 dark:border-green-400"
            >
              {condition.length > 30 ? `${condition.substring(0, 30)}...` : condition}
            </Badge>
          </div>
        </foreignObject>
      )}
    </>
  );
}
