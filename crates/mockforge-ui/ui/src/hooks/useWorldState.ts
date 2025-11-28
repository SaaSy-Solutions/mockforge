/**
 * Hook for World State API queries
 *
 * Provides React hooks for querying and streaming world state data
 */

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from './useWebSocket';
import { authenticatedFetch } from '../utils/apiClient';
import { logger } from '@/utils/logger';

// Types for world state
export interface WorldStateNode {
  id: string;
  label: string;
  node_type: string;
  layer: string;
  state?: string;
  properties: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface WorldStateEdge {
  from: string;
  to: string;
  relationship_type: string;
  properties: Record<string, unknown>;
  created_at: string;
}

export interface WorldStateSnapshot {
  id: string;
  timestamp: string;
  nodes: WorldStateNode[];
  edges: WorldStateEdge[];
  layers: Record<string, boolean>;
  metadata: Record<string, unknown>;
}

export interface WorldStateSnapshotResponse {
  snapshot: WorldStateSnapshot;
  available_layers: string[];
}

export interface WorldStateGraphResponse {
  nodes: WorldStateNode[];
  edges: WorldStateEdge[];
  metadata: {
    node_count: number;
    edge_count: number;
    timestamp: string;
  };
}

export interface WorldStateQueryRequest {
  node_types?: string[];
  layers?: string[];
  node_ids?: string[];
  relationship_types?: string[];
  include_edges?: boolean;
  max_depth?: number;
}

const API_BASE = '/api/world-state';

/**
 * Fetch current world state snapshot
 */
async function fetchWorldStateSnapshot(): Promise<WorldStateSnapshotResponse> {
  const response = await authenticatedFetch(`${API_BASE}/snapshot`);
  if (!response.ok) {
    throw new Error(`Failed to fetch world state: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Fetch world state as graph
 */
async function fetchWorldStateGraph(layers?: string): Promise<WorldStateGraphResponse> {
  const url = layers ? `${API_BASE}/graph?layers=${layers}` : `${API_BASE}/graph`;
  const response = await authenticatedFetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch world state graph: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Query world state with filters
 */
async function queryWorldState(request: WorldStateQueryRequest): Promise<WorldStateSnapshot> {
  const response = await authenticatedFetch(`${API_BASE}/query`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error(`Failed to query world state: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Get available layers
 */
async function fetchLayers(): Promise<{ layers: Array<{ id: string; name: string }>; count: number }> {
  const response = await authenticatedFetch(`${API_BASE}/layers`);
  if (!response.ok) {
    throw new Error(`Failed to fetch layers: ${response.statusText}`);
  }
  return response.json();
}

/**
 * Hook to get current world state snapshot
 */
export function useWorldStateSnapshot() {
  return useQuery({
    queryKey: ['world-state', 'snapshot'],
    queryFn: fetchWorldStateSnapshot,
    refetchInterval: 5000, // Refetch every 5 seconds
    staleTime: 2000,
  });
}

/**
 * Hook to get world state as graph
 */
export function useWorldStateGraph(layers?: string) {
  return useQuery({
    queryKey: ['world-state', 'graph', layers],
    queryFn: () => fetchWorldStateGraph(layers),
    refetchInterval: 5000,
    staleTime: 2000,
  });
}

/**
 * Hook to query world state with filters
 */
export function useWorldStateQuery(request: WorldStateQueryRequest, enabled = true) {
  return useQuery({
    queryKey: ['world-state', 'query', request],
    queryFn: () => queryWorldState(request),
    enabled,
    staleTime: 2000,
  });
}

/**
 * Hook to get available layers
 */
export function useWorldStateLayers() {
  return useQuery({
    queryKey: ['world-state', 'layers'],
    queryFn: fetchLayers,
    staleTime: 60000, // Layers don't change often
  });
}

/**
 * Hook for WebSocket streaming of world state
 */
export function useWorldStateStream(enabled = true) {
  const wsUrl = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}${API_BASE}/stream`;

  const { lastMessage, connected, connect, disconnect } = useWebSocket(wsUrl, {
    autoConnect: enabled,
    reconnect: {
      enabled: true,
      maxAttempts: 5,
      delay: 2000,
    },
  });

  // Parse WebSocket messages
  let snapshot: WorldStateSnapshot | null = null;
  if (lastMessage?.data) {
    try {
      snapshot = JSON.parse(lastMessage.data) as WorldStateSnapshot;
    } catch (error) {
      logger.error('Failed to parse world state snapshot from WebSocket', error);
    }
  }

  return {
    snapshot,
    connected,
    connect,
    disconnect,
  };
}
