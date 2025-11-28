import { logger } from '@/utils/logger';
import React, { useEffect, useState, useCallback, useRef } from 'react';
import ReactFlow, {
  Node,
  Edge,
  Background,
  Controls,
  MiniMap,
  Connection,
  addEdge,
  useNodesState,
  useEdgesState,
  NodeTypes,
  ReactFlowInstance,
} from 'react-flow-renderer';
import { Loader2 } from 'lucide-react';
import { Card, CardContent } from '../components/ui/Card';
import { apiService } from '../services/api';
import type { GraphData, GraphNode, GraphEdge } from '../types/graph';
import { EndpointNode } from '../components/graph/EndpointNode';
import { ServiceNode } from '../components/graph/ServiceNode';
import { GraphControls, LayoutType, FilterType, ProtocolFilter } from '../components/graph/GraphControls';
import { GraphDetailsPanel } from '../components/graph/GraphDetailsPanel';
import { applyLayout } from '../utils/graphLayouts';
import { applyClusterLayout } from '../utils/graphClustering';
import { useSSE } from '../hooks/useSSE';

interface GraphPageProps {
  className?: string;
}

const nodeTypes: NodeTypes = {
  endpoint: EndpointNode,
  service: ServiceNode,
  default: EndpointNode,
};

export const GraphPage: React.FC<GraphPageProps> = ({ className }) => {
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<GraphEdge | null>(null);
  const [layout, setLayout] = useState<LayoutType>('force-directed');
  const [nodeFilter, setNodeFilter] = useState<FilterType>('all');
  const [protocolFilter, setProtocolFilter] = useState<ProtocolFilter>('all');
  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);
  const refreshIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const [useRealtime, setUseRealtime] = useState(true);

  // SSE for real-time updates
  const { data: sseData, isConnected: sseConnected } = useSSE<GraphData>(
    '/__mockforge/graph/sse',
    {
      autoConnect: useRealtime,
      retry: {
        enabled: true,
        maxAttempts: 5,
        delay: 2000,
      },
    }
  );

  const fetchGraphData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await apiService.getGraph();
      setGraphData(response);

      // Apply filters
      let filteredNodes = response.nodes;
      let filteredEdges = response.edges;

      if (nodeFilter !== 'all') {
        filteredNodes = filteredNodes.filter((n) => n.nodeType === nodeFilter);
        // Filter edges to only include connections between visible nodes
        const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
        filteredEdges = filteredEdges.filter(
          (e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to)
        );
      }

      if (protocolFilter !== 'all') {
        filteredNodes = filteredNodes.filter((n) => n.protocol === protocolFilter);
        const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
        filteredEdges = filteredEdges.filter(
          (e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to)
        );
      }

      // Transform to ReactFlow format
      const reactFlowNodes = transformNodesToReactFlow(filteredNodes);
      const reactFlowEdges = transformEdgesToReactFlow(filteredEdges);

      // Apply clustering if clusters exist
      let laidOutNodes = reactFlowNodes;
      if (response.clusters && response.clusters.length > 0) {
        const { nodes: clusteredNodes } = applyClusterLayout(
          reactFlowNodes,
          reactFlowEdges,
          response.clusters
        );
        laidOutNodes = clusteredNodes;
      } else {
        // Apply layout if no clusters
        laidOutNodes = applyLayout(layout, reactFlowNodes, reactFlowEdges);
      }

      setNodes(laidOutNodes);
      setEdges(reactFlowEdges);
    } catch (err) {
      logger.error('Failed to fetch graph data', err);
      const errorMessage = err instanceof Error
        ? err.message
        : 'Failed to load graph data';
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [setNodes, setEdges, layout, nodeFilter, protocolFilter]);

  useEffect(() => {
    fetchGraphData();
  }, [fetchGraphData]);

  // Real-time updates via SSE (with polling fallback)
  useEffect(() => {
    if (sseData && sseConnected) {
      // Update graph from SSE data
      setGraphData(sseData);

      // Apply filters and transform
      let filteredNodes = sseData.nodes;
      let filteredEdges = sseData.edges;

      if (nodeFilter !== 'all') {
        filteredNodes = filteredNodes.filter((n) => n.nodeType === nodeFilter);
        const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
        filteredEdges = filteredEdges.filter(
          (e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to)
        );
      }

      if (protocolFilter !== 'all') {
        filteredNodes = filteredNodes.filter((n) => n.protocol === protocolFilter);
        const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
        filteredEdges = filteredEdges.filter(
          (e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to)
        );
      }

      const reactFlowNodes = transformNodesToReactFlow(filteredNodes);
      const reactFlowEdges = transformEdgesToReactFlow(filteredEdges);

      let laidOutNodes = reactFlowNodes;
      if (sseData.clusters && sseData.clusters.length > 0) {
        const { nodes: clusteredNodes } = applyClusterLayout(
          reactFlowNodes,
          reactFlowEdges,
          sseData.clusters
        );
        laidOutNodes = clusteredNodes;
      } else {
        laidOutNodes = applyLayout(layout, reactFlowNodes, reactFlowEdges);
      }

      setNodes(laidOutNodes);
      setEdges(reactFlowEdges);
    }
  }, [sseData, sseConnected, nodeFilter, protocolFilter, layout, setNodes, setEdges]);

  // Fallback to polling if SSE is not available
  useEffect(() => {
    if (!useRealtime || !sseConnected) {
      refreshIntervalRef.current = setInterval(() => {
        fetchGraphData();
      }, 30000); // Refresh every 30 seconds

      return () => {
        if (refreshIntervalRef.current) {
          clearInterval(refreshIntervalRef.current);
        }
      };
    }
  }, [fetchGraphData, useRealtime, sseConnected]);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    const graphNode = graphData?.nodes.find((n) => n.id === node.id);
    setSelectedNode(graphNode || null);
    setSelectedEdge(null);
  }, [graphData]);

  const onEdgeClick = useCallback((_event: React.MouseEvent, edge: Edge) => {
    const graphEdge = graphData?.edges.find(
      (e) => e.from === edge.source && e.to === edge.target
    );
    setSelectedEdge(graphEdge || null);
    setSelectedNode(null);
  }, [graphData]);

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, []);

  const handleLayoutChange = useCallback((newLayout: LayoutType) => {
    setLayout(newLayout);
    // Layout will be reapplied in fetchGraphData
  }, []);

  const handleExport = useCallback(async (format: 'png' | 'svg' | 'json') => {
    if (!reactFlowInstance) return;

    if (format === 'json') {
      const data = {
        nodes: nodes.map((n) => ({
          id: n.id,
          label: n.data.label,
          position: n.position,
        })),
        edges: edges.map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          label: e.label,
        })),
      };
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'graph.json';
      a.click();
      URL.revokeObjectURL(url);
    } else if (format === 'png') {
      // Use html2canvas to capture the ReactFlow viewport
      try {
        const html2canvas = (await import('html2canvas')).default;
        const reactFlowElement = document.querySelector('.react-flow') as HTMLElement;
        if (!reactFlowElement) {
          logger.error('ReactFlow element not found');
          return;
        }

        const canvas = await html2canvas(reactFlowElement, {
          backgroundColor: '#f9fafb', // Light gray background
          useCORS: true,
          scale: 2, // Higher quality
        });

        canvas.toBlob((blob) => {
          if (blob) {
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'graph.png';
            a.click();
            URL.revokeObjectURL(url);
          }
        }, 'image/png');
      } catch (err) {
        logger.error('Failed to export PNG', err);
      }
    } else if (format === 'svg') {
      // Get SVG from ReactFlow viewport
      try {
        const reactFlowElement = document.querySelector('.react-flow') as HTMLElement;
        if (!reactFlowElement) {
          logger.error('ReactFlow element not found');
          return;
        }

        // Get the SVG element from ReactFlow
        const svgElement = reactFlowElement.querySelector('svg');
        if (!svgElement) {
          logger.error('SVG element not found in ReactFlow');
          return;
        }

        // Clone the SVG to avoid modifying the original
        const clonedSvg = svgElement.cloneNode(true) as SVGElement;

        // Get viewport bounds
        const bounds = reactFlowElement.getBoundingClientRect();
        clonedSvg.setAttribute('width', bounds.width.toString());
        clonedSvg.setAttribute('height', bounds.height.toString());
        clonedSvg.setAttribute('viewBox', `0 0 ${bounds.width} ${bounds.height}`);

        // Serialize to string
        const serializer = new XMLSerializer();
        const svgString = serializer.serializeToString(clonedSvg);

        // Add XML declaration and create blob
        const svgBlob = new Blob(
          ['<?xml version="1.0" encoding="UTF-8"?>\n', svgString],
          { type: 'image/svg+xml;charset=utf-8' }
        );

        const url = URL.createObjectURL(svgBlob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'graph.svg';
        a.click();
        URL.revokeObjectURL(url);
      } catch (err) {
        logger.error('Failed to export SVG', err);
      }
    }
  }, [reactFlowInstance, nodes, edges]);

  if (loading && !graphData) {
    return (
      <div className={`p-6 ${className}`}>
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <span className="ml-2 text-lg">Loading graph...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col h-[calc(100vh-120px)] ${className}`}>
      <GraphControls
        layout={layout}
        onLayoutChange={handleLayoutChange}
        nodeFilter={nodeFilter}
        onNodeFilterChange={setNodeFilter}
        protocolFilter={protocolFilter}
        onProtocolFilterChange={setProtocolFilter}
        onRefresh={fetchGraphData}
        onExport={handleExport}
        nodeCount={nodes.length}
        edgeCount={edges.length}
      />

      <div className="flex-1 relative">
        <Card className="h-full">
          <CardContent className="h-full p-0">
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onConnect={onConnect}
              onNodeClick={onNodeClick}
              onEdgeClick={onEdgeClick}
              onPaneClick={onPaneClick}
              onInit={setReactFlowInstance}
              nodeTypes={nodeTypes}
              fitView
              attributionPosition="bottom-left"
              className="bg-gray-50 dark:bg-gray-900"
            >
              <Background />
              <Controls />
              <MiniMap
                nodeColor={(node) => {
                  const protocol = node.data?.protocol;
                  return protocolColors[protocol as string] || '#94a3b8';
                }}
                maskColor="rgba(0, 0, 0, 0.1)"
              />
            </ReactFlow>
          </CardContent>
        </Card>

        {(selectedNode || selectedEdge) && (
          <GraphDetailsPanel
            selectedNode={selectedNode}
            selectedEdge={selectedEdge}
            onClose={() => {
              setSelectedNode(null);
              setSelectedEdge(null);
            }}
          />
        )}
      </div>

      {error && (
        <div className="mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
        </div>
      )}
    </div>
  );
};

// Transform GraphNode to ReactFlow Node
function transformNodesToReactFlow(graphNodes: GraphNode[]): Node[] {
  return graphNodes.map((node, index) => {
    // Determine node type based on nodeType
    let nodeType = 'default';
    if (node.nodeType === 'endpoint') {
      nodeType = 'endpoint';
    } else if (node.nodeType === 'service') {
      nodeType = 'service';
    }

    return {
      id: node.id,
      type: nodeType,
      position: {
        x: (index % 10) * 150,
        y: Math.floor(index / 10) * 150,
      },
      data: {
        label: node.label,
        nodeType: node.nodeType,
        protocol: node.protocol,
        currentState: node.currentState,
        metadata: node.metadata,
      },
    };
  });
}

// Transform GraphEdge to ReactFlow Edge
function transformEdgesToReactFlow(graphEdges: GraphEdge[]): Edge[] {
  return graphEdges.map((edge, index) => ({
    id: `${edge.from}-${edge.to}-${index}`,
    source: edge.from,
    target: edge.to,
    label: edge.label,
    type: 'default',
    animated: edge.edgeType === 'statetransition',
    style: {
      stroke: getEdgeColor(edge.edgeType),
      strokeWidth: 2,
    },
    labelStyle: {
      fill: '#374151',
      fontWeight: 500,
    },
  }));
}

function getEdgeColor(edgeType: string): string {
  const colors: Record<string, string> = {
    dependency: '#3b82f6', // blue
    statetransition: '#10b981', // green
    servicecall: '#8b5cf6', // purple
    dataflow: '#f59e0b', // amber
    contains: '#6b7280', // gray
  };
  return colors[edgeType.toLowerCase()] || '#6b7280';
}

const protocolColors: Record<string, string> = {
  http: '#3b82f6',
  grpc: '#10b981',
  websocket: '#8b5cf6',
  graphql: '#ec4899',
  mqtt: '#f59e0b',
  smtp: '#eab308',
  kafka: '#ef4444',
  amqp: '#6366f1',
  ftp: '#6b7280',
};

export default GraphPage;
