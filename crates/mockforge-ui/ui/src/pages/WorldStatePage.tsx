/**
 * World State Page
 *
 * Interactive visualization of the unified MockForge world state
 */

import React, { useState, useCallback, useMemo } from 'react';
import {
  PageHeader,
  Alert,
  Section,
} from '../components/ui/DesignSystem';
import { WorldStateGraph } from '../components/world-state/WorldStateGraph';
import { StateLayerPanel, type StateLayer } from '../components/world-state/StateLayerPanel';
import { StateNodeInspector } from '../components/world-state/StateNodeInspector';
import {
  useWorldStateGraph,
  useWorldStateLayers,
  useWorldStateStream,
  type WorldStateNode,
  type WorldStateEdge,
} from '../hooks/useWorldState';
import { DashboardLoading, ErrorState } from '../components/ui/LoadingStates';

export const WorldStatePage: React.FC = () => {
  const [selectedNode, setSelectedNode] = useState<WorldStateNode | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<WorldStateEdge | null>(null);
  const [enabledLayers, setEnabledLayers] = useState<Set<string>>(new Set());
  const [useRealtime, setUseRealtime] = useState(true);
  const [layout, setLayout] = useState<'force-directed' | 'hierarchical' | 'circular'>('force-directed');

  // Fetch layers
  const { data: layersData, isLoading: layersLoading } = useWorldStateLayers();

  // Build layer state
  const layers: StateLayer[] = useMemo(() => {
    if (!layersData) return [];
    return layersData.layers.map((layer) => ({
      id: layer.id,
      name: layer.name,
      enabled: enabledLayers.has(layer.id) || enabledLayers.size === 0,
    }));
  }, [layersData, enabledLayers]);

  // Build layer filter string
  const layerFilter = useMemo(() => {
    const activeLayers = layers.filter((l) => l.enabled).map((l) => l.id);
    return activeLayers.length > 0 && activeLayers.length < layers.length
      ? activeLayers.join(',')
      : undefined;
  }, [layers]);

  // Fetch graph data
  const { data: graphData, isLoading: graphLoading, error: graphError } = useWorldStateGraph(layerFilter);

  // WebSocket stream (optional, for real-time updates)
  const { snapshot: streamSnapshot, connected: streamConnected } = useWorldStateStream(useRealtime);

  // Use stream data if available, otherwise use REST data
  const displayNodes = useMemo(() => {
    if (streamSnapshot && useRealtime) {
      return streamSnapshot.nodes;
    }
    return graphData?.nodes || [];
  }, [streamSnapshot, graphData, useRealtime]);

  const displayEdges = useMemo(() => {
    if (streamSnapshot && useRealtime) {
      return streamSnapshot.edges;
    }
    return graphData?.edges || [];
  }, [streamSnapshot, graphData, useRealtime]);

  // Filter nodes and edges by enabled layers
  const filteredNodes = useMemo(() => {
    if (enabledLayers.size === 0) return displayNodes;
    return displayNodes.filter((node) => {
      const layerId = node.layer.toLowerCase().replace(/\s+/g, '_');
      return enabledLayers.has(layerId) || layers.find((l) => l.id === layerId)?.enabled;
    });
  }, [displayNodes, enabledLayers, layers]);

  const filteredEdges = useMemo(() => {
    const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
    return displayEdges.filter(
      (e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to)
    );
  }, [displayEdges, filteredNodes]);

  // Handle layer toggle
  const handleLayerToggle = useCallback((layerId: string, enabled: boolean) => {
    setEnabledLayers((prev) => {
      const next = new Set(prev);
      if (enabled) {
        next.add(layerId);
      } else {
        next.delete(layerId);
      }
      return next;
    });
  }, []);

  // Handle node click
  const handleNodeClick = useCallback((node: WorldStateNode) => {
    setSelectedNode(node);
    setSelectedEdge(null);
  }, []);

  // Handle edge click
  const handleEdgeClick = useCallback((edge: WorldStateEdge) => {
    setSelectedEdge(edge);
    setSelectedNode(null);
  }, []);

  const isLoading = layersLoading || graphLoading;
  const error = graphError;

  if (isLoading) {
    return (
      <div className="content-width space-y-8">
        <PageHeader
          title="World State"
          subtitle="Unified visualization of all MockForge state systems"
          className="space-section"
        />
        <DashboardLoading />
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-width space-y-8">
        <PageHeader
          title="World State"
          subtitle="Unified visualization of all MockForge state systems"
          className="space-section"
        />
        <ErrorState
          title="Failed to load world state"
          description="Unable to retrieve world state data. Please try refreshing the page."
          error={error}
          retry={() => window.location.reload()}
        />
      </div>
    );
  }

  return (
    <div className="content-width space-y-6">
      <PageHeader
        title="World State"
        subtitle="Unified visualization of all MockForge state systems - like a miniature game engine for your backend"
        className="space-section"
      />

      {/* Controls */}
      <Section>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-4">
            <label className="flex items-center space-x-2">
              <input
                type="checkbox"
                checked={useRealtime}
                onChange={(e) => setUseRealtime(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">Real-time updates</span>
            </label>
            {streamConnected && (
              <span className="text-xs text-green-600">● Connected</span>
            )}
          </div>
          <select
            value={layout}
            onChange={(e) =>
              setLayout(
                e.target.value as 'force-directed' | 'hierarchical' | 'circular'
              )
            }
            className="text-sm border rounded px-3 py-1"
          >
            <option value="force-directed">Force Directed</option>
            <option value="hierarchical">Hierarchical</option>
            <option value="circular">Circular</option>
          </select>
        </div>
      </Section>

      {/* Main content */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left sidebar - Layer panel */}
        <div className="col-span-2">
          <StateLayerPanel layers={layers} onLayerToggle={handleLayerToggle} />
        </div>

        {/* Center - Graph visualization */}
        <div className="col-span-7">
          <div className="h-[600px] border rounded-lg overflow-hidden">
            <WorldStateGraph
              nodes={filteredNodes}
              edges={filteredEdges}
              onNodeClick={handleNodeClick}
              onEdgeClick={handleEdgeClick}
              selectedNodeId={selectedNode?.id}
              selectedEdgeId={selectedEdge ? `${selectedEdge.from}-${selectedEdge.to}-${selectedEdge.relationship_type}` : undefined}
              layout={layout}
            />
          </div>
        </div>

        {/* Right sidebar - Node inspector */}
        <div className="col-span-3">
          <StateNodeInspector
            node={selectedNode}
            edges={filteredEdges}
          />
        </div>
      </div>

      {/* Stats */}
      <Section>
        <div className="grid grid-cols-4 gap-4">
          <div className="text-center">
            <div className="text-2xl font-bold">{filteredNodes.length}</div>
            <div className="text-sm text-gray-500">Nodes</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">{filteredEdges.length}</div>
            <div className="text-sm text-gray-500">Edges</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">{layers.length}</div>
            <div className="text-sm text-gray-500">Layers</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">
              {layers.filter((l) => l.enabled).length}
            </div>
            <div className="text-sm text-gray-500">Active Layers</div>
          </div>
        </div>
      </Section>
    </div>
  );
};
