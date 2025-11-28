import { Node, Edge } from 'react-flow-renderer';
import type { GraphCluster } from '../types/graph';

/**
 * Group nodes by cluster and apply cluster-based positioning
 */
export function applyClusterLayout(
  nodes: Node[],
  edges: Edge[],
  clusters: GraphCluster[]
): { nodes: Node[]; clusterGroups: Map<string, Node[]> } {
  const clusterGroups = new Map<string, Node[]>();
  const nodeToCluster = new Map<string, string>();

  // Build cluster groups
  clusters.forEach((cluster) => {
    const clusterNodes: Node[] = [];
    cluster.nodeIds.forEach((nodeId) => {
      const node = nodes.find((n) => n.id === nodeId);
      if (node) {
        clusterNodes.push(node);
        nodeToCluster.set(nodeId, cluster.id);
      }
    });
    if (clusterNodes.length > 0) {
      clusterGroups.set(cluster.id, clusterNodes);
    }
  });

  // Position nodes within clusters
  const CLUSTER_SPACING = 400;
  const NODE_SPACING = 150;
  let clusterX = 0;
  let clusterY = 0;
  const maxNodesPerRow = 5;

  clusterGroups.forEach((clusterNodes, clusterId) => {
    // Position nodes in a grid within the cluster
    clusterNodes.forEach((node, index) => {
      const row = Math.floor(index / maxNodesPerRow);
      const col = index % maxNodesPerRow;
      node.position = {
        x: clusterX + col * NODE_SPACING,
        y: clusterY + row * NODE_SPACING,
      };
    });

    // Move to next cluster position
    const rows = Math.ceil(clusterNodes.length / maxNodesPerRow);
    clusterY += rows * NODE_SPACING + CLUSTER_SPACING;
    if (clusterY > 2000) {
      clusterY = 0;
      clusterX += CLUSTER_SPACING * 2;
    }
  });

  // Position unclustered nodes
  const unclusteredNodes = nodes.filter((node) => !nodeToCluster.has(node.id));
  unclusteredNodes.forEach((node, index) => {
    node.position = {
      x: clusterX + (index % maxNodesPerRow) * NODE_SPACING,
      y: clusterY + Math.floor(index / maxNodesPerRow) * NODE_SPACING,
    };
  });

  return { nodes, clusterGroups };
}

/**
 * Get cluster information for a node
 */
export function getNodeCluster(
  nodeId: string,
  clusters: GraphCluster[]
): GraphCluster | null {
  return clusters.find((cluster) => cluster.nodeIds.includes(nodeId)) || null;
}

/**
 * Get all nodes in a cluster
 */
export function getClusterNodes(
  clusterId: string,
  nodes: Node[],
  clusters: GraphCluster[]
): Node[] {
  const cluster = clusters.find((c) => c.id === clusterId);
  if (!cluster) return [];

  return nodes.filter((node) => cluster.nodeIds.includes(node.id));
}
