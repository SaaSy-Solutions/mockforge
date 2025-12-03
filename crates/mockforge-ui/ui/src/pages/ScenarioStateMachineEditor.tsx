//! Scenario State Machine Editor
//!
//! Visual flow editor for creating and editing scenario state machines with React Flow.
//! Supports nested sub-scenarios, conditional transitions, and real-time state preview.

import { logger } from '@/utils/logger';
import React, { useEffect, useState, useCallback, useRef, useMemo } from 'react';
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
  MarkerType,
} from '@xyflow/react';
import { Loader2, Save, Download, Upload, Undo2, Redo2, Play, Square, Plus, Trash2, Database, Layers } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/button';
import { apiService } from '../services/api';
import { StateNode } from '../components/state-machine/StateNode';
import { TransitionEdge } from '../components/state-machine/TransitionEdge';
import { ConditionBuilder } from '../components/state-machine/ConditionBuilder';
import { StatePreviewPanel } from '../components/state-machine/StatePreviewPanel';
import { VbrEntitySelector } from '../components/state-machine/VbrEntitySelector';
import { SubScenarioEditor } from '../components/state-machine/SubScenarioEditor';
import { useWebSocket } from '../hooks/useWebSocket';
import { useHistory } from '../hooks/useHistory';

// Types for state machine data structures
interface StateMachineDefinition {
  resource_type: string;
  states: string[];
  initial_state: string;
  transitions: Array<{
    from_state: string;
    to_state: string;
    condition_expression?: string;
    sub_scenario_ref?: string;
    probability?: number;
  }>;
  sub_scenarios?: Array<{
    id: string;
    name: string;
    description?: string;
    state_machine: StateMachineDefinition;
  }>;
  visual_layout?: {
    nodes: Array<{
      id: string;
      type: string;
      position_x: number;
      position_y: number;
      width?: number;
      height?: number;
      label: string;
    }>;
    edges: Array<{
      id: string;
      source: string;
      target: string;
      label?: string;
      type?: string;
      animated?: boolean;
    }>;
  };
}

interface StateMachineEditorProps {
  resourceType?: string;
  className?: string;
}

const nodeTypes: NodeTypes = {
  state: StateNode,
  initial: StateNode,
  final: StateNode,
};

const edgeTypes = {
  default: TransitionEdge,
};

export const ScenarioStateMachineEditor: React.FC<StateMachineEditorProps> = ({
  resourceType,
  className = '',
}) => {
  // State management
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [stateMachine, setStateMachine] = useState<StateMachineDefinition | null>(null);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<Edge | null>(null);
  const [editingCondition, setEditingCondition] = useState<Edge | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [showVbrSelector, setShowVbrSelector] = useState(false);
  const [showSubScenarioEditor, setShowSubScenarioEditor] = useState(false);
  const [editingSubScenario, setEditingSubScenario] = useState<string | undefined>(undefined);
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

  // Load state machine on mount or when resourceType changes
  useEffect(() => {
    if (resourceType) {
      loadStateMachine(resourceType);
    } else {
      // Create new state machine
      initializeNewStateMachine();
    }
  }, [resourceType]);

  // Handle WebSocket messages for real-time updates
  useEffect(() => {
    if (lastMessage) {
      try {
        const event = JSON.parse(lastMessage.data);
        if (event.type === 'state_transitioned' && event.resource_type === resourceType) {
          // Update preview if showing
          if (showPreview) {
            // StatePreviewPanel will handle the update
          }
        } else if (event.type === 'state_machine_updated' && event.resource_type === resourceType) {
          // Reload state machine if it was updated externally
          loadStateMachine(resourceType);
        }
      } catch (err) {
        logger.error('Failed to parse WebSocket message', err);
      }
    }
  }, [lastMessage, resourceType, showPreview]);

  // Save current state to history when nodes or edges change
  useEffect(() => {
    push({ nodes, edges });
  }, [nodes, edges, push]);

  // Initialize a new empty state machine
  const initializeNewStateMachine = useCallback(() => {
    const newStateMachine: StateMachineDefinition = {
      resource_type: 'new-state-machine',
      states: ['initial'],
      initial_state: 'initial',
      transitions: [],
    };

    setStateMachine(newStateMachine);

    // Create initial node
    const initialNode: Node = {
      id: 'initial',
      type: 'initial',
      position: { x: 250, y: 250 },
      data: {
        label: 'Initial',
        state: 'initial',
        isInitial: true,
      },
    };

    setNodes([initialNode]);
    setEdges([]);
    setLoading(false);
  }, [setNodes, setEdges]);

  // Load state machine from API
  const loadStateMachine = useCallback(async (rt: string) => {
    try {
      setLoading(true);
      setError(null);

      const response = await apiService.getStateMachine(rt);
      const sm = response.state_machine as StateMachineDefinition;
      const layout = response.visual_layout;

      setStateMachine(sm);

      // Convert state machine to React Flow nodes and edges
      const flowNodes: Node[] = sm.states.map((state, index) => {
        // Use layout if available, otherwise use default positioning
        const layoutNode = layout?.nodes?.find((n) => n.id === state);
        const position = layoutNode
          ? { x: layoutNode.position_x, y: layoutNode.position_y }
          : { x: (index % 5) * 200 + 100, y: Math.floor(index / 5) * 150 + 100 };

        return {
          id: state,
          type: state === sm.initial_state ? 'initial' : 'state',
          position,
          data: {
            label: state,
            state,
            isInitial: state === sm.initial_state,
          },
          style: {
            width: layoutNode?.width || 150,
            height: layoutNode?.height || 60,
          },
        };
      });

      const flowEdges: Edge[] = sm.transitions.map((transition, index) => {
        const layoutEdge = layout?.edges?.find(
          (e) => e.source === transition.from_state && e.target === transition.to_state
        );

        return {
          id: layoutEdge?.id || `edge-${index}`,
          source: transition.from_state,
          target: transition.to_state,
          label: transition.condition_expression || '',
          type: 'default',
          animated: layoutEdge?.animated || false,
          data: {
            condition: transition.condition_expression,
            subScenarioRef: transition.sub_scenario_ref,
            probability: transition.probability,
          },
          markerEnd: {
            type: MarkerType.ArrowClosed,
          },
        };
      });

      setNodes(flowNodes);
      setEdges(flowEdges);
      push({ nodes: flowNodes, edges: flowEdges });
    } catch (err) {
      logger.error('Failed to load state machine', err);
      setError(err instanceof Error ? err.message : 'Failed to load state machine');
    } finally {
      setLoading(false);
    }
  }, [setNodes, setEdges, push]);

  // Save state machine to API
  const saveStateMachine = useCallback(async () => {
    if (!stateMachine) return;

    try {
      setError(null);

      // Convert React Flow nodes/edges back to state machine format
      const states = nodes.map((n) => n.data.state || n.id);
      const initialState = nodes.find((n) => n.data.isInitial)?.data.state || nodes[0]?.data.state || 'initial';

      const transitions = edges.map((e) => ({
        from_state: e.source,
        to_state: e.target,
        condition_expression: e.data?.condition,
        sub_scenario_ref: e.data?.subScenarioRef,
        probability: e.data?.probability,
      }));

      const updatedStateMachine: StateMachineDefinition = {
        ...stateMachine,
        states,
        initial_state: initialState,
        transitions,
        visual_layout: {
          nodes: nodes.map((n) => ({
            id: n.id,
            type: n.type || 'state',
            position_x: n.position.x,
            position_y: n.position.y,
            width: n.style?.width as number,
            height: n.style?.height as number,
            label: n.data.label || n.id,
          })),
          edges: edges.map((e) => ({
            id: e.id,
            source: e.source,
            target: e.target,
            label: e.label as string,
            type: e.type,
            animated: e.animated,
          })),
        },
      };

      if (resourceType) {
        await apiService.updateStateMachine(
          resourceType,
          updatedStateMachine,
          updatedStateMachine.visual_layout
        );
      } else {
        const response = await apiService.createStateMachine(
          updatedStateMachine,
          updatedStateMachine.visual_layout
        );
        // Update resource type from response
        if (response.state_machine) {
          const sm = response.state_machine as StateMachineDefinition;
          setStateMachine(sm);
        }
      }

      logger.info('State machine saved successfully');
    } catch (err) {
      logger.error('Failed to save state machine', err);
      setError(err instanceof Error ? err.message : 'Failed to save state machine');
    }
  }, [stateMachine, nodes, edges, resourceType]);

  // Handle node creation
  const handleAddNode = useCallback(() => {
    const newNodeId = `state-${Date.now()}`;
    const newNode: Node = {
      id: newNodeId,
      type: 'state',
      position: { x: Math.random() * 400 + 100, y: Math.random() * 400 + 100 },
      data: {
        label: 'New State',
        state: newNodeId,
        isInitial: false,
      },
    };

    setNodes((nds) => [...nds, newNode]);
  }, [setNodes]);

  // Handle node deletion
  const handleDeleteNode = useCallback(() => {
    if (!selectedNode) return;

    setNodes((nds) => nds.filter((n) => n.id !== selectedNode.id));
    setEdges((eds) => eds.filter((e) => e.source !== selectedNode.id && e.target !== selectedNode.id));
    setSelectedNode(null);
  }, [selectedNode, setNodes, setEdges]);

  // Handle edge connection
  const onConnect = useCallback(
    (params: Connection) => {
      const newEdge: Edge = {
        ...addEdge(params, []),
        type: 'default',
        data: {
          condition: '',
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
        },
      };
      setEdges((eds) => [...eds, newEdge]);
    },
    [setEdges]
  );

  // Handle node click
  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setSelectedEdge(null);
    setEditingCondition(null);
  }, []);

  // Handle edge click
  const onEdgeClick = useCallback((_event: React.MouseEvent, edge: Edge) => {
    setSelectedEdge(edge);
    setSelectedNode(null);
    setEditingCondition(edge);
  }, []);

  // Handle pane click (deselect)
  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
    setEditingCondition(null);
  }, []);

  // Handle condition update
  const handleConditionUpdate = useCallback(
    (edgeId: string, condition: string) => {
      setEdges((eds) =>
        eds.map((e) =>
          e.id === edgeId
            ? {
                ...e,
                label: condition,
                data: {
                  ...e.data,
                  condition,
                },
              }
            : e
        )
      );
      setEditingCondition(null);
    },
    [setEdges]
  );

  // Handle export
  const handleExport = useCallback(async () => {
    try {
      const data = await apiService.exportStateMachines();
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `state-machines-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      logger.error('Failed to export state machines', err);
      setError(err instanceof Error ? err.message : 'Failed to export');
    }
  }, []);

  // Handle import
  const handleImport = useCallback(async (file: File) => {
    try {
      const text = await file.text();
      const data = JSON.parse(text);
      await apiService.importStateMachines(data);
      if (resourceType) {
        await loadStateMachine(resourceType);
      }
    } catch (err) {
      logger.error('Failed to import state machines', err);
      setError(err instanceof Error ? err.message : 'Failed to import');
    }
  }, [resourceType, loadStateMachine]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + Z for undo
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
      // Cmd/Ctrl + Shift + Z or Cmd/Ctrl + Y for redo
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
      // Cmd/Ctrl + S for save
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        saveStateMachine();
      }
      // Delete key for deleting selected node
      if (e.key === 'Delete' && selectedNode) {
        handleDeleteNode();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [canUndo, canRedo, undo, redo, saveStateMachine, selectedNode, handleDeleteNode, setNodes, setEdges]);

  if (loading && !stateMachine) {
    return (
      <div className={`p-6 ${className}`}>
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <span className="ml-2 text-lg">Loading state machine...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col h-[calc(100vh-120px)] ${className}`}>
      {/* Toolbar */}
      <div className="flex items-center justify-between p-4 border-b bg-white dark:bg-gray-800">
        <div className="flex items-center gap-2">
          <Button onClick={handleAddNode} size="sm" variant="outline">
            <Plus className="h-4 w-4 mr-2" />
            Add State
          </Button>
          <Button
            onClick={handleDeleteNode}
            size="sm"
            variant="outline"
            disabled={!selectedNode}
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </Button>
          <div className="w-px h-6 bg-gray-300 dark:bg-gray-600 mx-2" />
          <Button
            onClick={undo}
            size="sm"
            variant="outline"
            disabled={!canUndo}
            title="Undo (Cmd/Ctrl+Z)"
          >
            <Undo2 className="h-4 w-4" />
          </Button>
          <Button
            onClick={redo}
            size="sm"
            variant="outline"
            disabled={!canRedo}
            title="Redo (Cmd/Ctrl+Shift+Z)"
          >
            <Redo2 className="h-4 w-4" />
          </Button>
          <div className="w-px h-6 bg-gray-300 dark:bg-gray-600 mx-2" />
          <Button onClick={saveStateMachine} size="sm" variant="default">
            <Save className="h-4 w-4 mr-2" />
            Save
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={handleExport} size="sm" variant="outline">
            <Download className="h-4 w-4 mr-2" />
            Export
          </Button>
          <label>
            <input
              type="file"
              accept=".json"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) handleImport(file);
              }}
              className="hidden"
            />
            <Button as="span" size="sm" variant="outline">
              <Upload className="h-4 w-4 mr-2" />
              Import
            </Button>
          </label>
          <Button
            onClick={() => setShowPreview(!showPreview)}
            size="sm"
            variant={showPreview ? 'default' : 'outline'}
          >
            {showPreview ? <Square className="h-4 w-4 mr-2" /> : <Play className="h-4 w-4 mr-2" />}
            {showPreview ? 'Hide Preview' : 'Show Preview'}
          </Button>
          <Button
            onClick={() => setShowVbrSelector(true)}
            size="sm"
            variant="outline"
            title="Select VBR Entity"
          >
            <Database className="h-4 w-4 mr-2" />
            VBR Entity
          </Button>
          <Button
            onClick={() => {
              setEditingSubScenario(undefined);
              setShowSubScenarioEditor(true);
            }}
            size="sm"
            variant="outline"
            title="Add Sub-Scenario"
          >
            <Layers className="h-4 w-4 mr-2" />
            Sub-Scenario
          </Button>
        </div>
      </div>

      {/* Main editor area */}
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
              edgeTypes={edgeTypes}
              fitView
              attributionPosition="bottom-left"
              className="bg-gray-50 dark:bg-gray-900"
            >
              <Background />
              <Controls />
              <MiniMap
                nodeColor={(node) => {
                  if (node.data?.isInitial) return '#10b981';
                  return '#3b82f6';
                }}
                maskColor="rgba(0, 0, 0, 0.1)"
              />
            </ReactFlow>
          </CardContent>
        </Card>

        {/* Condition Builder Dialog */}
        {editingCondition && (
          <div className="absolute top-4 right-4 z-10 w-96">
            <Card>
              <CardHeader>
                <CardTitle>Edit Transition Condition</CardTitle>
              </CardHeader>
              <CardContent>
                <ConditionBuilder
                  condition={editingCondition.data?.condition || ''}
                  onUpdate={(condition) => handleConditionUpdate(editingCondition.id, condition)}
                  onCancel={() => setEditingCondition(null)}
                />
              </CardContent>
            </Card>
          </div>
        )}

        {/* State Preview Panel */}
        {showPreview && stateMachine && (
          <div className="absolute bottom-4 right-4 z-10 w-96">
            <StatePreviewPanel
              resourceType={stateMachine.resource_type}
              onClose={() => setShowPreview(false)}
            />
          </div>
        )}

        {/* VBR Entity Selector */}
        {showVbrSelector && (
          <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-20">
            <VbrEntitySelector
              selectedEntity={stateMachine?.resource_type}
              onSelect={(entityName) => {
                if (stateMachine) {
                  setStateMachine({
                    ...stateMachine,
                    resource_type: entityName,
                  });
                }
                setShowVbrSelector(false);
              }}
              onClose={() => setShowVbrSelector(false)}
            />
          </div>
        )}

        {/* Sub-Scenario Editor */}
        {showSubScenarioEditor && (
          <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-20">
            <SubScenarioEditor
              subScenarioId={editingSubScenario}
              onSave={(config) => {
                // Add sub-scenario to state machine
                if (stateMachine) {
                  const updatedSubScenarios = [
                    ...(stateMachine.sub_scenarios || []),
                    {
                      id: config.id,
                      name: config.name,
                      description: config.description,
                      state_machine: {
                        resource_type: config.state_machine_resource_type,
                        states: [],
                        initial_state: '',
                        transitions: [],
                      },
                    },
                  ];
                  setStateMachine({
                    ...stateMachine,
                    sub_scenarios: updatedSubScenarios,
                  });
                }
                setShowSubScenarioEditor(false);
                setEditingSubScenario(undefined);
              }}
              onCancel={() => {
                setShowSubScenarioEditor(false);
                setEditingSubScenario(undefined);
              }}
            />
          </div>
        )}
      </div>

      {/* Error display */}
      {error && (
        <div className="mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md mx-4 mb-4">
          <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
        </div>
      )}
    </div>
  );
};

export default ScenarioStateMachineEditor;
