//! Scenario Studio step types
//!
//! Single source of truth for step type names. These strings are the React
//! Flow node `type`, the persisted `step_type`, and the backend's serialized
//! `StepType` (`mockforge-intelligence` `#[serde(rename_all = "snake_case")]`),
//! so they must stay snake_case.

import type { NodeTypes } from '@xyflow/react';
import { ApiCallNode } from './ApiCallNode';
import { ConditionNode } from './ConditionNode';
import { DelayNode } from './DelayNode';
import { LoopNode } from './LoopNode';
import { ParallelNode } from './ParallelNode';

export const STEP_TYPES = ['api_call', 'condition', 'delay', 'loop', 'parallel'] as const;

export type StepType = (typeof STEP_TYPES)[number];

export const scenarioNodeTypes = {
  api_call: ApiCallNode,
  condition: ConditionNode,
  delay: DelayNode,
  loop: LoopNode,
  parallel: ParallelNode,
} satisfies Record<StepType, NodeTypes[string]>;

export const STEP_TYPE_COLORS: Record<StepType, string> = {
  api_call: '#3b82f6',
  condition: '#a855f7',
  delay: '#eab308',
  loop: '#6366f1',
  parallel: '#14b8a6',
};

/** Map a stored or node type to its canonical name, accepting the legacy camelCase `apiCall`. */
export function normalizeStepType(type: string | undefined): StepType {
  if (type === 'apiCall') return 'api_call';
  return (STEP_TYPES as readonly string[]).includes(type ?? '') ? (type as StepType) : 'api_call';
}
