/**
 * Custom Node Component for World State Graph
 *
 * Displays world state nodes with layer-specific styling
 */

import React from 'react';
import { Handle, Position, NodeProps } from 'react-flow-renderer';

interface WorldStateNodeData {
  label: string;
  nodeType: string;
  layer: string;
  state?: string;
  properties: Record<string, unknown>;
  selected?: boolean;
}

/**
 * Get color for layer
 */
function getLayerColor(layer: string): string {
  const colors: Record<string, string> = {
    personas: '#3b82f6',
    lifecycle: '#10b981',
    reality: '#f59e0b',
    time: '#8b5cf6',
    protocols: '#ef4444',
    behavior: '#ec4899',
    schemas: '#06b6d4',
    recorded: '#6366f1',
    ai_modifiers: '#14b8a6',
    system: '#64748b',
  };
  return colors[layer.toLowerCase()] || '#6b7280';
}

export const WorldStateNodeComponent: React.FC<NodeProps<WorldStateNodeData>> = ({
  data,
  selected,
}) => {
  const layerColor = getLayerColor(data?.layer || 'system');
  const isSelected = selected || data?.selected;

  return (
    <div
      style={{
        padding: '10px',
        borderRadius: '8px',
        background: isSelected ? '#eff6ff' : '#ffffff',
        border: `2px solid ${isSelected ? layerColor : '#e5e7eb'}`,
        minWidth: '150px',
        boxShadow: isSelected
          ? `0 4px 6px -1px ${layerColor}40`
          : '0 1px 3px 0 rgba(0, 0, 0, 0.1)',
      }}
    >
      <Handle type="target" position={Position.Top} />

      <div style={{ marginBottom: '8px' }}>
        <div
          style={{
            fontSize: '12px',
            fontWeight: 600,
            color: layerColor,
            textTransform: 'uppercase',
            marginBottom: '4px',
          }}
        >
          {data?.layer || 'unknown'}
        </div>
        <div style={{ fontSize: '14px', fontWeight: 500, color: '#111827' }}>
          {data?.label || 'Unknown Node'}
        </div>
        {data?.state && (
          <div
            style={{
              fontSize: '11px',
              color: '#6b7280',
              marginTop: '4px',
              fontStyle: 'italic',
            }}
          >
            {data.state}
          </div>
        )}
      </div>

      <div
        style={{
          fontSize: '10px',
          color: '#9ca3af',
          borderTop: '1px solid #e5e7eb',
          paddingTop: '4px',
          marginTop: '4px',
        }}
      >
        {data?.nodeType || 'unknown'}
      </div>

      <Handle type="source" position={Position.Bottom} />
    </div>
  );
};
