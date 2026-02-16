/**
 * World State Graph Visualization Component
 *
 * Displays the unified world state as an interactive graph using React Flow
 */

import React, { useMemo, useCallback } from 'react';
import {
  ReactFlow,
  Node,
  Edge,
  Background,
  Controls,
  MiniMap,
  NodeTypes,
  Connection,
  addEdge,
  useNodesState,
  useEdgesState,
} from '@xyflow/react';
import type { WorldStateNode, WorldStateEdge } from '../../hooks/useWorldState';
import { WorldStateNodeComponent } from './WorldStateNode';
import { applyLayout } from '../../utils/graphLayouts';

interface WorldStateGraphProps {
  nodes: WorldStateNode[];
  edges: WorldStateEdge[];
  onNodeClick?: (node: WorldStateNode) => void;
  onEdgeClick?: (edge: WorldStateEdge) => void;
  selectedNodeId?: string;
  selectedEdgeId?: string;
  layout?: 'force-directed' | 'hierarchical' | 'circular';
}

const nodeTypes: NodeTypes = {
  worldStateNode: WorldStateNodeComponent,
  default: WorldStateNodeComponent,
};

/**
 * Transform world state nodes to React Flow format
 */
function transformNodesToReactFlow(
  nodes: WorldStateNode[],
  selectedNodeId?: string
): Node[] {
  return nodes.map((node) => ({
    id: node.id,
    type: 'worldStateNode',
    position: { x: 0, y: 0 }, // Will be set by layout
    data: {
      label: node.label,
      nodeType: node.node_type,
      layer: node.layer,
      state: node.state,
      properties: node.properties,
      selected: node.id === selectedNodeId,
    },
    style: {
      border: node.id === selectedNodeId ? '2px solid #3b82f6' : '1px solid #e5e7eb',
    },
  }));
}

/**
 * Transform world state edges to React Flow format
 */
function transformEdgesToReactFlow(
  edges: WorldStateEdge[],
  selectedEdgeId?: string
): Edge[] {
  return edges.map((edge) => {
    const edgeId = `${edge.from}-${edge.to}-${edge.relationship_type}`;
    return {
      id: edgeId,
      source: edge.from,
      target: edge.to,
      label: edge.relationship_type,
      type: 'smoothstep',
      animated: edgeId === selectedEdgeId,
      style: {
        stroke: edgeId === selectedEdgeId ? '#3b82f6' : '#9ca3af',
        strokeWidth: edgeId === selectedEdgeId ? 2 : 1,
      },
    };
  });
}

export const WorldStateGraph: React.FC<WorldStateGraphProps> = ({
  nodes,
  edges,
  onNodeClick,
  onEdgeClick,
  selectedNodeId,
  selectedEdgeId,
  layout = 'force-directed',
}) => {
  // Transform data to React Flow format
  const reactFlowNodes = useMemo(
    () => transformNodesToReactFlow(nodes, selectedNodeId),
    [nodes, selectedNodeId]
  );

  const reactFlowEdges = useMemo(
    () => transformEdgesToReactFlow(edges, selectedEdgeId),
    [edges, selectedEdgeId]
  );

  // Apply layout
  const laidOutNodes = useMemo(
    () => applyLayout(layout, reactFlowNodes, reactFlowEdges),
    [layout, reactFlowNodes, reactFlowEdges]
  );

  const [flowNodes, setNodes, onNodesChange] = useNodesState(laidOutNodes);
  const [flowEdges, setEdges, onEdgesChange] = useEdgesState(reactFlowEdges);

  // Update nodes and edges when data changes
  React.useEffect(() => {
    setNodes(laidOutNodes);
  }, [laidOutNodes, setNodes]);

  React.useEffect(() => {
    setEdges(reactFlowEdges);
  }, [reactFlowEdges, setEdges]);

  // Handle node click
  const onNodeClickHandler = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const worldStateNode = nodes.find((n) => n.id === node.id);
      if (worldStateNode && onNodeClick) {
        onNodeClick(worldStateNode);
      }
    },
    [nodes, onNodeClick]
  );

  // Handle edge click
  const onEdgeClickHandler = useCallback(
    (_event: React.MouseEvent, edge: Edge) => {
      const worldStateEdge = edges.find(
        (e) => `${e.from}-${e.to}-${e.relationship_type}` === edge.id
      );
      if (worldStateEdge && onEdgeClick) {
        onEdgeClick(worldStateEdge);
      }
    },
    [edges, onEdgeClick]
  );

  // Handle connections (for future interactive editing)
  const onConnect = useCallback(
    (params: Connection) => {
      setEdges((eds) => addEdge(params, eds));
    },
    [setEdges]
  );

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClickHandler}
        onEdgeClick={onEdgeClickHandler}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-left"
      >
        <Background />
        <Controls />
        <MiniMap />
      </ReactFlow>
    </div>
  );
};
