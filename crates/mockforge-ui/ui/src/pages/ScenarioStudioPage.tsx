//! Scenario Studio Page
//!
//! Visual flow editor for co-editing business flows (happy path, SLA violation, regression)
//! with drag-and-drop React Flow integration.

import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  ReactFlow,
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
  MarkerType,
} from '@xyflow/react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import {
  Plus,
  Play,
  Save,
  Trash2,
  Loader2,
  Globe,
  GitBranch,
  Clock,
  Repeat,
  Layers,
  Settings,
} from 'lucide-react';
import { ApiCallNode, ApiCallNodeData } from '@/components/scenario-studio/ApiCallNode';
import { ConditionNode, ConditionNodeData } from '@/components/scenario-studio/ConditionNode';
import { DelayNode, DelayNodeData } from '@/components/scenario-studio/DelayNode';
import { LoopNode, LoopNodeData } from '@/components/scenario-studio/LoopNode';
import { ParallelNode, ParallelNodeData } from '@/components/scenario-studio/ParallelNode';
import { FlowPropertiesPanel } from '@/components/scenario-studio/FlowPropertiesPanel';
import { FlowExecutor } from '@/components/scenario-studio/FlowExecutor';
import { useHistory } from '@/hooks/useHistory';
import { useWebSocket } from '@/hooks/useWebSocket';

interface FlowDefinition {
  id: string;
  name: string;
  description?: string;
  flow_type: 'happy_path' | 'sla_violation' | 'regression' | 'custom';
  steps: FlowStep[];
  connections: FlowConnection[];
  tags: string[];
  created_at: string;
  updated_at: string;
}

interface FlowStep {
  id: string;
  name: string;
  step_type: 'api_call' | 'condition' | 'delay' | 'loop' | 'parallel';
  method?: string;
  endpoint?: string;
  delay_ms?: number;
  expected_status?: number;
  expression?: string;
  iterations?: number;
  condition?: string;
  branches?: number;
  position?: { x: number; y: number };
}

interface FlowConnection {
  from_step_id: string;
  to_step_id: string;
  label?: string;
}

// Node type mapping
const nodeTypes: NodeTypes = {
  apiCall: ApiCallNode,
  condition: ConditionNode,
  delay: DelayNode,
  loop: LoopNode,
  parallel: ParallelNode,
};

export function ScenarioStudioPage() {
  const [flows, setFlows] = useState<FlowDefinition[]>([]);
  const [selectedFlow, setSelectedFlow] = useState<FlowDefinition | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [newFlowName, setNewFlowName] = useState('');
  const [newFlowType, setNewFlowType] = useState<'happy_path' | 'sla_violation' | 'regression' | 'custom'>('happy_path');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [showProperties, setShowProperties] = useState(false);
  const [showExecutor, setShowExecutor] = useState(false);
  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // History for undo/redo
  const { history, push, undo, redo, canUndo, canRedo } = useHistory<{
    nodes: Node[];
    edges: Edge[];
  }>({ nodes: [], edges: [] }, 50);

  // WebSocket for real-time updates
  const { lastMessage, sendMessage, connected } = useWebSocket('/__mockforge/ws');

  // Load flows on mount
  useEffect(() => {
    loadFlows();
  }, []);

  // Load selected flow into React Flow
  useEffect(() => {
    if (selectedFlow) {
      loadFlowIntoEditor(selectedFlow);
    } else {
      setNodes([]);
      setEdges([]);
    }
  }, [selectedFlow]);

  // Handle WebSocket messages
  useEffect(() => {
    if (lastMessage && selectedFlow) {
      try {
        const event = JSON.parse(lastMessage.data);
        if (event.type === 'flow_updated' && event.flow_id === selectedFlow.id) {
          loadFlows();
        }
      } catch (err) {
        console.error('Failed to parse WebSocket message', err);
      }
    }
  }, [lastMessage, selectedFlow]);

  // Save to history when nodes/edges change
  useEffect(() => {
    if (nodes.length > 0 || edges.length > 0) {
      push({ nodes, edges });
    }
  }, [nodes, edges, push]);

  const loadFlows = async () => {
    try {
      setLoading(true);
      const response = await fetch('/api/v1/scenario-studio/flows');
      if (response.ok) {
        const data = await response.json();
        setFlows(data);
      }
    } catch (error) {
      console.error('Failed to load flows:', error);
      setError('Failed to load flows');
    } finally {
      setLoading(false);
    }
  };

  const loadFlowIntoEditor = (flow: FlowDefinition) => {
    // Convert flow steps to React Flow nodes
    const flowNodes: Node[] = flow.steps.map((step, index) => {
      const position = step.position || { x: (index % 5) * 250 + 100, y: Math.floor(index / 5) * 150 + 100 };

      let nodeData: any = {
        id: step.id,
        name: step.name,
      };

      // Add type-specific data
      switch (step.step_type) {
        case 'api_call':
          nodeData = {
            ...nodeData,
            method: step.method,
            endpoint: step.endpoint,
            expectedStatus: step.expected_status,
          } as ApiCallNodeData;
          break;
        case 'condition':
          nodeData = {
            ...nodeData,
            expression: step.expression,
          } as ConditionNodeData;
          break;
        case 'delay':
          nodeData = {
            ...nodeData,
            delayMs: step.delay_ms,
          } as DelayNodeData;
          break;
        case 'loop':
          nodeData = {
            ...nodeData,
            iterations: step.iterations,
            condition: step.condition,
          } as LoopNodeData;
          break;
        case 'parallel':
          nodeData = {
            ...nodeData,
            branches: step.branches,
          } as ParallelNodeData;
          break;
      }

      return {
        id: step.id,
        type: step.step_type,
        position,
        data: nodeData,
      };
    });

    // Convert flow connections to React Flow edges
    const flowEdges: Edge[] = flow.connections.map((conn, index) => ({
      id: `edge-${index}`,
      source: conn.from_step_id,
      target: conn.to_step_id,
      label: conn.label || '',
      markerEnd: {
        type: MarkerType.ArrowClosed,
      },
    }));

    setNodes(flowNodes);
    setEdges(flowEdges);
  };

  const createFlow = async () => {
    try {
      const response = await fetch('/api/v1/scenario-studio/flows', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newFlowName,
          flow_type: newFlowType,
        }),
      });
      if (response.ok) {
        const flow = await response.json();
        setFlows([...flows, flow]);
        setSelectedFlow(flow);
        setIsCreating(false);
        setNewFlowName('');
      }
    } catch (error) {
      console.error('Failed to create flow:', error);
      setError('Failed to create flow');
    }
  };

  const saveFlow = async () => {
    if (!selectedFlow) return;

    try {
      // Convert React Flow nodes/edges back to flow format
      const steps: FlowStep[] = nodes.map((node) => {
        const baseStep: FlowStep = {
          id: node.id,
          name: node.data.name,
          step_type: (node.type as any) || 'api_call',
          position: { x: node.position.x, y: node.position.y },
        };

        // Add type-specific fields
        switch (node.type) {
          case 'api_call':
            const apiData = node.data as ApiCallNodeData;
            return {
              ...baseStep,
              method: apiData.method,
              endpoint: apiData.endpoint,
              expected_status: apiData.expectedStatus,
            };
          case 'condition':
            const conditionData = node.data as ConditionNodeData;
            return {
              ...baseStep,
              expression: conditionData.expression,
            };
          case 'delay':
            const delayData = node.data as DelayNodeData;
            return {
              ...baseStep,
              delay_ms: delayData.delayMs,
            };
          case 'loop':
            const loopData = node.data as LoopNodeData;
            return {
              ...baseStep,
              iterations: loopData.iterations,
              condition: loopData.condition,
            };
          case 'parallel':
            const parallelData = node.data as ParallelNodeData;
            return {
              ...baseStep,
              branches: parallelData.branches,
            };
          default:
            return baseStep;
        }
      });

      const connections: FlowConnection[] = edges.map((edge) => ({
        from_step_id: edge.source,
        to_step_id: edge.target,
        label: edge.label as string,
      }));

      const response = await fetch(`/api/v1/scenario-studio/flows/${selectedFlow.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          steps,
          connections,
        }),
      });

      if (response.ok) {
        const updatedFlow = await response.json();
        setSelectedFlow(updatedFlow);
        setFlows(flows.map((f) => (f.id === updatedFlow.id ? updatedFlow : f)));

        // Broadcast update via WebSocket
        if (connected) {
          sendMessage({
            type: 'flow_updated',
            flow_id: updatedFlow.id,
          });
        }
      }
    } catch (error) {
      console.error('Failed to save flow:', error);
      setError('Failed to save flow');
    }
  };

  const deleteFlow = async (flowId: string) => {
    if (!confirm('Are you sure you want to delete this flow?')) return;
    try {
      const response = await fetch(`/api/v1/scenario-studio/flows/${flowId}`, {
        method: 'DELETE',
      });
      if (response.ok) {
        setFlows(flows.filter((f) => f.id !== flowId));
        if (selectedFlow?.id === flowId) {
          setSelectedFlow(null);
        }
      }
    } catch (error) {
      console.error('Failed to delete flow:', error);
      setError('Failed to delete flow');
    }
  };

  const handleAddNode = (stepType: FlowStep['step_type']) => {
    if (!selectedFlow) return;

    const nodeId = `step-${Date.now()}`;
    let nodeData: any = {
      id: nodeId,
      name: `New ${stepType.replace('_', ' ')}`,
    };

    // Add type-specific defaults
    switch (stepType) {
      case 'api_call':
        nodeData = { ...nodeData, method: 'GET', endpoint: '/api/endpoint' } as ApiCallNodeData;
        break;
      case 'condition':
        nodeData = { ...nodeData, expression: '{{condition}}' } as ConditionNodeData;
        break;
      case 'delay':
        nodeData = { ...nodeData, delayMs: 1000 } as DelayNodeData;
        break;
      case 'loop':
        nodeData = { ...nodeData, iterations: 5 } as LoopNodeData;
        break;
      case 'parallel':
        nodeData = { ...nodeData, branches: 2 } as ParallelNodeData;
        break;
    }

    const newNode: Node = {
      id: nodeId,
      type: stepType,
      position: reactFlowInstance
        ? reactFlowInstance.project({ x: 400, y: 300 })
        : { x: 400, y: 300 },
      data: nodeData,
    };

    setNodes((nds) => [...nds, newNode]);
  };

  const handleDeleteNode = useCallback(() => {
    if (!selectedNode) return;
    setNodes((nds) => nds.filter((n) => n.id !== selectedNode.id));
    setEdges((eds) => eds.filter((e) => e.source !== selectedNode.id && e.target !== selectedNode.id));
    setSelectedNode(null);
    setShowProperties(false);
  }, [selectedNode, setNodes, setEdges]);

  const onConnect = useCallback(
    (params: Connection) => {
      const newEdge: Edge = {
        ...addEdge(params, []),
        markerEnd: {
          type: MarkerType.ArrowClosed,
        },
      };
      setEdges((eds) => [...eds, newEdge]);
    },
    [setEdges]
  );

  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setShowProperties(true);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setShowProperties(false);
  }, []);

  const handleNodeUpdate = useCallback(
    (nodeId: string, data: any) => {
      setNodes((nds) =>
        nds.map((n) => (n.id === nodeId ? { ...n, data: { ...n.data, ...data } } : n))
      );
      setShowProperties(false);
    },
    [setNodes]
  );

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'z' && !e.shiftKey) {
        e.preventDefault();
        if (canUndo) {
          const prev = undo();
          if (prev) {
            setNodes(prev.nodes);
            setEdges(prev.edges);
          }
        }
      }
      if ((e.metaKey || e.ctrlKey) && (e.shiftKey ? e.key === 'z' : e.key === 'y')) {
        e.preventDefault();
        if (canRedo) {
          const next = redo();
          if (next) {
            setNodes(next.nodes);
            setEdges(next.edges);
          }
        }
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        saveFlow();
      }
      if (e.key === 'Delete' && selectedNode) {
        handleDeleteNode();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [canUndo, canRedo, undo, redo, saveFlow, selectedNode, handleDeleteNode, setNodes, setEdges]);

  return (
    <div className="flex h-[calc(100vh-120px)]">
      {/* Flow List Sidebar */}
      <div className="w-80 border-r bg-white dark:bg-gray-800 flex flex-col">
        <div className="p-4 border-b">
          <h2 className="text-lg font-semibold mb-4">Scenario Studio</h2>
          <Button
            size="sm"
            onClick={() => setIsCreating(true)}
            className="w-full"
          >
            <Plus className="h-4 w-4 mr-2" />
            New Flow
          </Button>
        </div>

        {isCreating && (
          <div className="p-4 border-b space-y-3">
            <div>
              <Label htmlFor="flow-name">Flow Name</Label>
              <Input
                id="flow-name"
                value={newFlowName}
                onChange={(e) => setNewFlowName(e.target.value)}
                placeholder="Enter flow name"
              />
            </div>
            <div>
              <Label htmlFor="flow-type">Flow Type</Label>
              <Select value={newFlowType} onValueChange={(v: any) => setNewFlowType(v)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="happy_path">Happy Path</SelectItem>
                  <SelectItem value="sla_violation">SLA Violation</SelectItem>
                  <SelectItem value="regression">Regression</SelectItem>
                  <SelectItem value="custom">Custom</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex gap-2">
              <Button size="sm" onClick={createFlow} className="flex-1">
                Create
              </Button>
              <Button size="sm" variant="outline" onClick={() => setIsCreating(false)} className="flex-1">
                Cancel
              </Button>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto p-2">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : (
            <div className="space-y-2">
              {flows.map((flow) => (
                <div
                  key={flow.id}
                  className={`p-3 border rounded-lg cursor-pointer hover:bg-accent ${
                    selectedFlow?.id === flow.id ? 'bg-accent border-primary' : ''
                  }`}
                  onClick={() => setSelectedFlow(flow)}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{flow.name}</div>
                      <div className="text-sm text-muted-foreground">
                        {flow.flow_type.replace('_', ' ')}
                      </div>
                    </div>
                    <div className="flex gap-1">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={(e) => {
                          e.stopPropagation();
                          setShowExecutor(true);
                        }}
                      >
                        <Play className="h-4 w-4" />
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteFlow(flow.id);
                        }}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Main Editor Area */}
      <div className="flex-1 flex flex-col">
        {selectedFlow ? (
          <>
            {/* Toolbar */}
            <div className="flex items-center justify-between p-4 border-b bg-white dark:bg-gray-800">
              <div className="flex items-center gap-2">
                <h3 className="font-semibold">{selectedFlow.name}</h3>
                <span className="text-sm text-muted-foreground">
                  ({selectedFlow.flow_type.replace('_', ' ')})
                </span>
              </div>
              <div className="flex items-center gap-2">
                <div className="flex gap-1 border-r pr-2">
                  <Button size="sm" variant="outline" onClick={() => handleAddNode('api_call')} title="Add API Call">
                    <Globe className="h-4 w-4" />
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => handleAddNode('condition')} title="Add Condition">
                    <GitBranch className="h-4 w-4" />
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => handleAddNode('delay')} title="Add Delay">
                    <Clock className="h-4 w-4" />
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => handleAddNode('loop')} title="Add Loop">
                    <Repeat className="h-4 w-4" />
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => handleAddNode('parallel')} title="Add Parallel">
                    <Layers className="h-4 w-4" />
                  </Button>
                </div>
                <Button size="sm" onClick={saveFlow}>
                  <Save className="h-4 w-4 mr-2" />
                  Save
                </Button>
                <Button size="sm" variant="outline" onClick={() => setShowExecutor(true)}>
                  <Play className="h-4 w-4 mr-2" />
                  Execute
                </Button>
              </div>
            </div>

            {/* React Flow Canvas */}
            <div className="flex-1 relative">
              <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onNodeClick={onNodeClick}
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
                    const colors: Record<string, string> = {
                      apiCall: '#3b82f6',
                      condition: '#a855f7',
                      delay: '#eab308',
                      loop: '#6366f1',
                      parallel: '#14b8a6',
                    };
                    return colors[node.type || 'apiCall'] || '#6b7280';
                  }}
                />
              </ReactFlow>

              {/* Properties Panel */}
              {showProperties && selectedNode && (
                <div className="absolute top-4 right-4 z-10">
                  <FlowPropertiesPanel
                    selectedNode={selectedNode}
                    onUpdate={handleNodeUpdate}
                    onClose={() => setShowProperties(false)}
                  />
                </div>
              )}

              {/* Executor Panel */}
              {showExecutor && selectedFlow && (
                <div className="absolute top-4 left-4 z-10">
                  <FlowExecutor
                    flowId={selectedFlow.id}
                    onClose={() => setShowExecutor(false)}
                  />
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center text-muted-foreground">
              <p className="text-lg mb-2">No flow selected</p>
              <p className="text-sm">Select a flow from the sidebar or create a new one</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default ScenarioStudioPage;
