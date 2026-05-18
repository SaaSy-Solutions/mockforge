/**
 * Cloud World State view (#464 Phase 2).
 *
 * Rendered by `WorldStatePage` when `isCloudMode()`. Drives a per-deployment
 * world-state visualization against `cloudWorldStateApi` (the registry
 * proxy from Phase 1 — see `cloudWorldState.ts` and
 * `handlers::world_state`). Reuses the existing `StateLayerPanel`,
 * `WorldStateGraph`, and `StateNodeInspector` components so the graph
 * surface looks identical to local mode.
 *
 * Pattern mirrors `CloudTimeTravelView` (#466 Phase 2) and
 * `ResiliencePage`'s cloud branch: useState/useEffect with direct
 * `await cloudWorldStateApi.*` calls plus a deployment dropdown. The
 * WebSocket `/stream` endpoint that the local view consumes is not
 * proxied in Phase 1 — cloud mode polls every 5s instead, which matches
 * the `refetchInterval` defaults on the local TanStack Query hooks.
 */

import React, { useEffect, useState, useCallback, useMemo } from 'react';
import { PageHeader, Alert, Section } from '../components/ui/DesignSystem';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/button';
import { RefreshCw } from 'lucide-react';
import { WorldStateGraph } from '../components/world-state/WorldStateGraph';
import { StateLayerPanel, type StateLayer } from '../components/world-state/StateLayerPanel';
import { StateNodeInspector } from '../components/world-state/StateNodeInspector';
import {
  cloudWorldStateApi,
  type WorldStateRuntimeState,
} from '../services/api/cloudWorldState';
import type {
  WorldStateNode,
  WorldStateEdge,
  WorldStateGraphResponse,
} from '../hooks/useWorldState';

interface DeploymentSummary {
  id: string;
  name: string;
  status: string;
}

interface LayersResponse {
  layers: Array<{ id: string; name: string }>;
  count: number;
}

const POLL_MS = 5000;

export const CloudWorldStateView: React.FC = () => {
  const [deployments, setDeployments] = useState<DeploymentSummary[]>([]);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);

  const [runtimeState, setRuntimeState] = useState<WorldStateRuntimeState | null>(null);
  const [graphData, setGraphData] = useState<WorldStateGraphResponse | null>(null);
  const [layersData, setLayersData] = useState<LayersResponse | null>(null);

  const [enabledLayers, setEnabledLayers] = useState<Set<string>>(new Set());
  const [selectedNode, setSelectedNode] = useState<WorldStateNode | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<WorldStateEdge | null>(null);
  const [layout, setLayout] = useState<'force-directed' | 'hierarchical' | 'circular'>(
    'force-directed',
  );

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load deployments once + auto-select first active (same pattern as the
  // other cloud views).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const token = localStorage.getItem('auth_token');
        const resp = await fetch('/api/v1/hosted-mocks', {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
        if (!resp.ok) {
          if (!cancelled) {
            setError(`Failed to load deployments: ${resp.status}`);
            setLoading(false);
          }
          return;
        }
        const list = (await resp.json()) as DeploymentSummary[];
        if (cancelled) return;
        const items = Array.isArray(list) ? list : [];
        setDeployments(items);
        const active = items.find((d) => d.status === 'active') ?? items[0] ?? null;
        if (active) {
          setSelectedDeploymentId(active.id);
        } else {
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load deployments');
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Compute the layer filter string (comma-separated layer ids) from the
  // enabled set. When no layer is explicitly toggled OR all layers are
  // toggled, we omit the filter — Phase 1's proxy forwards the param
  // verbatim, so an omitted filter is the runtime's default ("all").
  const allLayerIds = useMemo(
    () => (Array.isArray(layersData?.layers) ? layersData!.layers.map((l) => l.id) : []),
    [layersData],
  );

  const layerFilter = useMemo(() => {
    if (enabledLayers.size === 0) return undefined;
    if (enabledLayers.size === allLayerIds.length) return undefined;
    return Array.from(enabledLayers).join(',');
  }, [enabledLayers, allLayerIds]);

  // Poll graph + layers when deployment changes (or filter changes for
  // graph). 5s cadence matches local mode's refetchInterval.
  useEffect(() => {
    if (!selectedDeploymentId) return;
    let cancelled = false;

    const fetchData = async () => {
      try {
        const [graphEnv, layersEnv] = await Promise.all([
          cloudWorldStateApi.getGraph<WorldStateGraphResponse>(
            selectedDeploymentId,
            layerFilter,
          ),
          cloudWorldStateApi.getLayers<LayersResponse>(selectedDeploymentId),
        ]);
        if (cancelled) return;

        // The proxy returns the same envelope for both calls; if either
        // is unreachable we mark the whole view unreachable so the user
        // gets one banner, not two.
        if (
          graphEnv.runtime_state === 'unreachable' ||
          layersEnv.runtime_state === 'unreachable'
        ) {
          setRuntimeState('unreachable');
        } else {
          setRuntimeState('live');
        }
        setGraphData(graphEnv.data);
        setLayersData(layersEnv.data);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setRuntimeState('unreachable');
        setError(err instanceof Error ? err.message : 'Failed to load world state');
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchData();
    const t = setInterval(fetchData, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [selectedDeploymentId, layerFilter]);

  // Layer panel state. Default: all layers enabled (so the view shows
  // everything until the user filters explicitly).
  const layers: StateLayer[] = useMemo(() => {
    if (!layersData || !Array.isArray(layersData.layers)) return [];
    return layersData.layers.map((layer) => ({
      id: layer.id,
      name: layer.name,
      enabled: enabledLayers.size === 0 ? true : enabledLayers.has(layer.id),
    }));
  }, [layersData, enabledLayers]);

  const displayNodes = useMemo(
    () => (Array.isArray(graphData?.nodes) ? graphData!.nodes : []),
    [graphData],
  );
  const displayEdges = useMemo(
    () => (Array.isArray(graphData?.edges) ? graphData!.edges : []),
    [graphData],
  );

  const filteredNodes = useMemo(() => {
    if (enabledLayers.size === 0) return displayNodes;
    return displayNodes.filter((node) => {
      const layerId = node.layer.toLowerCase().replace(/\s+/g, '_');
      return enabledLayers.has(layerId);
    });
  }, [displayNodes, enabledLayers]);

  const filteredEdges = useMemo(() => {
    const visibleNodeIds = new Set(filteredNodes.map((n) => n.id));
    return displayEdges.filter((e) => visibleNodeIds.has(e.from) && visibleNodeIds.has(e.to));
  }, [displayEdges, filteredNodes]);

  // Handlers — same shape as local WorldStatePage so the inspector
  // panel behaves identically.
  const handleLayerToggle = useCallback((layerId: string, enabled: boolean) => {
    setEnabledLayers((prev) => {
      const next = new Set(prev);
      if (enabled) next.add(layerId);
      else next.delete(layerId);
      return next;
    });
  }, []);

  const handleNodeClick = useCallback((node: WorldStateNode) => {
    setSelectedNode(node);
    setSelectedEdge(null);
  }, []);

  const handleEdgeClick = useCallback((edge: WorldStateEdge) => {
    setSelectedEdge(edge);
    setSelectedNode(null);
  }, []);

  const refresh = useCallback(async () => {
    if (!selectedDeploymentId) return;
    try {
      const [graphEnv, layersEnv] = await Promise.all([
        cloudWorldStateApi.getGraph<WorldStateGraphResponse>(
          selectedDeploymentId,
          layerFilter,
        ),
        cloudWorldStateApi.getLayers<LayersResponse>(selectedDeploymentId),
      ]);
      setRuntimeState(
        graphEnv.runtime_state === 'unreachable' || layersEnv.runtime_state === 'unreachable'
          ? 'unreachable'
          : 'live',
      );
      setGraphData(graphEnv.data);
      setLayersData(layersEnv.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to refresh');
    }
  }, [selectedDeploymentId, layerFilter]);

  // --- Render ------------------------------------------------------------

  if (deployments.length === 0 && !loading) {
    return (
      <div className="content-width space-y-8">
        <PageHeader title="World State" subtitle="Cloud world-state visualization" />
        <Alert
          variant="info"
          title="No deployments yet"
          description="Create a hosted mock first to visualize its world state from the cloud dashboard."
        />
      </div>
    );
  }

  const unreachable = runtimeState === 'unreachable';

  return (
    <div className="content-width space-y-6">
      <PageHeader
        title="World State"
        subtitle="Per-deployment unified state graph"
        className="space-section"
      />

      {/* Deployment selector */}
      {deployments.length > 1 ? (
        <Card className="p-4">
          <label className="block text-sm font-medium text-foreground mb-2">Deployment</label>
          <select
            className="w-full px-3 py-2 rounded-lg border border-border bg-background"
            value={selectedDeploymentId ?? ''}
            onChange={(e) => setSelectedDeploymentId(e.target.value)}
          >
            {deployments.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.status})
              </option>
            ))}
          </select>
        </Card>
      ) : (
        deployments[0] && (
          <p className="text-sm text-muted-foreground">
            Deployment:{' '}
            <span className="font-medium text-foreground">{deployments[0].name}</span>
            <Badge variant="outline" className="ml-2">
              {deployments[0].status}
            </Badge>
          </p>
        )
      )}

      {unreachable && (
        <Alert
          variant="warning"
          title="Deployment not reachable"
          description="The registry couldn't reach this deployment's runtime over Fly 6PN. Showing the last successful snapshot if any; polling will keep retrying. If this persists, check the deployment's status."
        />
      )}

      {error && !unreachable && <Alert variant="error" title="Error" description={error} />}

      <Section>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center space-x-4">
            {/* The local mode toggle is "Real-time updates"; cloud mode
                doesn't have a WS proxy yet (Phase 2 work), so we show a
                static label noting the polling cadence instead. */}
            <span className="text-xs text-muted-foreground">
              Polling every {POLL_MS / 1000}s — WebSocket stream not yet proxied in cloud mode.
            </span>
            <Button variant="outline" size="sm" onClick={refresh} disabled={!selectedDeploymentId}>
              <RefreshCw className="h-4 w-4 mr-1" />
              Refresh
            </Button>
          </div>
          <select
            value={layout}
            onChange={(e) =>
              setLayout(e.target.value as 'force-directed' | 'hierarchical' | 'circular')
            }
            className="text-sm border rounded px-3 py-1"
          >
            <option value="force-directed">Force Directed</option>
            <option value="hierarchical">Hierarchical</option>
            <option value="circular">Circular</option>
          </select>
        </div>
      </Section>

      <div className="grid grid-cols-12 gap-6">
        <div className="col-span-2">
          <StateLayerPanel layers={layers} onLayerToggle={handleLayerToggle} />
        </div>
        <div className="col-span-7">
          <div className="h-[600px] border rounded-lg overflow-hidden">
            <WorldStateGraph
              nodes={filteredNodes}
              edges={filteredEdges}
              onNodeClick={handleNodeClick}
              onEdgeClick={handleEdgeClick}
              selectedNodeId={selectedNode?.id}
              selectedEdgeId={
                selectedEdge
                  ? `${selectedEdge.from}-${selectedEdge.to}-${selectedEdge.relationship_type}`
                  : undefined
              }
              layout={layout}
            />
          </div>
        </div>
        <div className="col-span-3">
          <StateNodeInspector node={selectedNode} edges={filteredEdges} />
        </div>
      </div>

      <Section>
        <div className="grid grid-cols-4 gap-4">
          <div className="text-center">
            <div className="text-2xl font-bold">{filteredNodes.length}</div>
            <div className="text-sm text-muted-foreground">Nodes</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">{filteredEdges.length}</div>
            <div className="text-sm text-muted-foreground">Edges</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">{layers.length}</div>
            <div className="text-sm text-muted-foreground">Layers</div>
          </div>
          <div className="text-center">
            <div className="text-2xl font-bold">{layers.filter((l) => l.enabled).length}</div>
            <div className="text-sm text-muted-foreground">Active Layers</div>
          </div>
        </div>
      </Section>
    </div>
  );
};
