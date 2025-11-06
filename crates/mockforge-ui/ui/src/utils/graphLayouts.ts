import { Node, Edge } from 'react-flow-renderer';
import type { GraphData } from '../types/graph';

export type LayoutType = 'hierarchical' | 'force-directed' | 'grid' | 'circular';

/**
 * Apply hierarchical layout (top-to-bottom tree structure)
 */
export function applyHierarchicalLayout(
  nodes: Node[],
  edges: Edge[],
  direction: 'TB' | 'LR' = 'TB'
): Node[] {
  const nodeMap = new Map(nodes.map((n) => [n.id, n]));
  const childrenMap = new Map<string, string[]>();
  const parentMap = new Map<string, string>();
  const levels = new Map<string, number>();

  // Build parent-child relationships
  edges.forEach((edge) => {
    if (!childrenMap.has(edge.source)) {
      childrenMap.set(edge.source, []);
    }
    childrenMap.get(edge.source)!.push(edge.target);
    parentMap.set(edge.target, edge.source);
  });

  // Find root nodes (nodes with no incoming edges)
  const rootNodes = nodes.filter((node) => !parentMap.has(node.id));

  // Calculate levels using BFS
  const queue: Array<{ id: string; level: number }> = rootNodes.map((n) => ({
    id: n.id,
    level: 0,
  }));

  while (queue.length > 0) {
    const { id, level } = queue.shift()!;
    levels.set(id, level);

    const children = childrenMap.get(id) || [];
    children.forEach((childId) => {
      if (!levels.has(childId)) {
        queue.push({ id: childId, level: level + 1 });
      }
    });
  }

  // Group nodes by level
  const nodesByLevel = new Map<number, Node[]>();
  nodes.forEach((node) => {
    const level = levels.get(node.id) || 0;
    if (!nodesByLevel.has(level)) {
      nodesByLevel.set(level, []);
    }
    nodesByLevel.get(level)!.push(node);
  });

  // Position nodes
  const HORIZONTAL_SPACING = 200;
  const VERTICAL_SPACING = 150;
  const maxLevel = Math.max(...Array.from(nodesByLevel.keys()));

  return nodes.map((node) => {
    const level = levels.get(node.id) || 0;
    const levelNodes = nodesByLevel.get(level) || [];
    const indexInLevel = levelNodes.findIndex((n) => n.id === node.id);
    const nodesInLevel = levelNodes.length;

    if (direction === 'TB') {
      return {
        ...node,
        position: {
          x: (indexInLevel - nodesInLevel / 2) * HORIZONTAL_SPACING,
          y: level * VERTICAL_SPACING,
        },
      };
    } else {
      return {
        ...node,
        position: {
          x: level * VERTICAL_SPACING,
          y: (indexInLevel - nodesInLevel / 2) * HORIZONTAL_SPACING,
        },
      };
    }
  });
}

/**
 * Apply force-directed layout (simplified spring-based)
 */
export function applyForceDirectedLayout(
  nodes: Node[],
  edges: Edge[],
  iterations: number = 50
): Node[] {
  // Simple force-directed algorithm
  const nodePositions = new Map(
    nodes.map((node) => [
      node.id,
      { x: node.position.x || Math.random() * 500, y: node.position.y || Math.random() * 500 },
    ])
  );

  const k = Math.sqrt((800 * 600) / nodes.length); // Optimal distance
  const repulsionStrength = 1000;
  const attractionStrength = 0.01;

  for (let iter = 0; iter < iterations; iter++) {
    const forces = new Map(
      nodes.map((node) => [node.id, { x: 0, y: 0 }])
    );

    // Repulsion forces between all nodes
    nodes.forEach((node1) => {
      nodes.forEach((node2) => {
        if (node1.id === node2.id) return;

        const pos1 = nodePositions.get(node1.id)!;
        const pos2 = nodePositions.get(node2.id)!;
        const dx = pos2.x - pos1.x;
        const dy = pos2.y - pos1.y;
        const distance = Math.sqrt(dx * dx + dy * dy) || 0.1;

        const force = repulsionStrength / (distance * distance);
        const fx = (dx / distance) * force;
        const fy = (dy / distance) * force;

        const f1 = forces.get(node1.id)!;
        f1.x -= fx;
        f1.y -= fy;
      });
    });

    // Attraction forces along edges
    edges.forEach((edge) => {
      const pos1 = nodePositions.get(edge.source)!;
      const pos2 = nodePositions.get(edge.target)!;
      const dx = pos2.x - pos1.x;
      const dy = pos2.y - pos1.y;
      const distance = Math.sqrt(dx * dx + dy * dy) || 0.1;

      const force = attractionStrength * (distance - k);
      const fx = (dx / distance) * force;
      const fy = (dy / distance) * force;

      const f1 = forces.get(edge.source)!;
      const f2 = forces.get(edge.target)!;
      f1.x += fx;
      f1.y += fy;
      f2.x -= fx;
      f2.y -= fy;
    });

    // Update positions
    nodes.forEach((node) => {
      const force = forces.get(node.id)!;
      const pos = nodePositions.get(node.id)!;
      pos.x += force.x * 0.1;
      pos.y += force.y * 0.1;
    });
  }

  return nodes.map((node) => ({
    ...node,
    position: nodePositions.get(node.id)!,
  }));
}

/**
 * Apply grid layout
 */
export function applyGridLayout(nodes: Node[]): Node[] {
  const cols = Math.ceil(Math.sqrt(nodes.length));
  const spacing = 200;

  return nodes.map((node, index) => ({
    ...node,
    position: {
      x: (index % cols) * spacing,
      y: Math.floor(index / cols) * spacing,
    },
  }));
}

/**
 * Apply circular layout
 */
export function applyCircularLayout(nodes: Node[]): Node[] {
  const centerX = 400;
  const centerY = 300;
  const radius = Math.max(200, nodes.length * 10);
  const angleStep = (2 * Math.PI) / nodes.length;

  return nodes.map((node, index) => {
    const angle = index * angleStep;
    return {
      ...node,
      position: {
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      },
    };
  });
}

/**
 * Apply layout based on type
 */
export function applyLayout(
  layoutType: LayoutType,
  nodes: Node[],
  edges: Edge[]
): Node[] {
  switch (layoutType) {
    case 'hierarchical':
      return applyHierarchicalLayout(nodes, edges);
    case 'force-directed':
      return applyForceDirectedLayout(nodes, edges);
    case 'grid':
      return applyGridLayout(nodes);
    case 'circular':
      return applyCircularLayout(nodes);
    default:
      return nodes;
  }
}
