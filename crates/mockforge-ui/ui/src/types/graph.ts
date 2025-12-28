// Graph visualization types matching the backend GraphData structure

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
  clusters: GraphCluster[];
}

export interface GraphNode {
  id: string;
  label: string;
  nodeType: 'endpoint' | 'service' | 'workspace';
  protocol?: 'http' | 'grpc' | 'websocket' | 'graphql' | 'mqtt' | 'smtp' | 'kafka' | 'amqp' | 'ftp' | 'tcp';
  currentState?: string;
  metadata: Record<string, unknown>;
}

export interface GraphEdge {
  from: string;
  to: string;
  edgeType: 'dependency' | 'statetransition' | 'servicecall' | 'dataflow' | 'contains';
  label?: string;
  metadata: Record<string, unknown>;
}

export interface GraphCluster {
  id: string;
  label: string;
  clusterType: 'workspace' | 'service' | 'chain';
  nodeIds: string[];
  metadata: Record<string, unknown>;
}
