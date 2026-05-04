/**
 * State Node Inspector Component
 *
 * Displays detailed information about a selected node
 */

import React from 'react';
import { Card } from '../ui/Card';
import type { WorldStateNode, WorldStateEdge } from '../../hooks/useWorldState';

interface StateNodeInspectorProps {
  node: WorldStateNode | null;
  edges?: WorldStateEdge[];
}

export const StateNodeInspector: React.FC<StateNodeInspectorProps> = ({
  node,
  edges = [],
}) => {
  if (!node) {
    return (
      <Card className="p-4">
        <p className="text-sm text-muted-foreground">Select a node to view details</p>
      </Card>
    );
  }

  // Find connected edges
  const connectedEdges = edges.filter(
    (e) => e.from === node.id || e.to === node.id
  );

  return (
    <Card className="p-4">
      <h3 className="text-lg font-semibold mb-4">Node Details</h3>

      <div className="space-y-4">
        <div>
          <label className="text-xs font-semibold text-muted-foreground uppercase">
            Label
          </label>
          <p className="text-sm font-medium">{node.label}</p>
        </div>

        <div>
          <label className="text-xs font-semibold text-muted-foreground uppercase">
            Type
          </label>
          <p className="text-sm">{node.node_type}</p>
        </div>

        <div>
          <label className="text-xs font-semibold text-muted-foreground uppercase">
            Layer
          </label>
          <p className="text-sm">{node.layer}</p>
        </div>

        {node.state && (
          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              State
            </label>
            <p className="text-sm">{node.state}</p>
          </div>
        )}

        {Object.keys(node.properties).length > 0 && (
          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Properties
            </label>
            <pre className="text-xs bg-muted p-2 rounded overflow-auto max-h-40">
              {JSON.stringify(node.properties, null, 2)}
            </pre>
          </div>
        )}

        {connectedEdges.length > 0 && (
          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Connections ({connectedEdges.length})
            </label>
            <div className="space-y-1 mt-2">
              {connectedEdges.map((edge, idx) => (
                <div
                  key={idx}
                  className="text-xs bg-muted p-2 rounded"
                >
                  <span className="font-medium">
                    {edge.from === node.id ? '→' : '←'} {edge.relationship_type}
                  </span>
                  <span className="text-muted-foreground ml-2">
                    {edge.from === node.id ? edge.to : edge.from}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="pt-2 border-t">
          <div className="text-xs text-muted-foreground">
            <div>Created: {new Date(node.created_at).toLocaleString()}</div>
            <div>Updated: {new Date(node.updated_at).toLocaleString()}</div>
          </div>
        </div>
      </div>
    </Card>
  );
};
