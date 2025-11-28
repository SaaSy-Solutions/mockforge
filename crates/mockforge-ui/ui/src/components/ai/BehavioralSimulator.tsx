//! Behavioral Simulator Component
//!
//! Provides UI for AI-powered user behavior simulation. Models users as narrative agents
//! that react to app state, form intentions, respond to errors, and trigger multi-step interactions.
//!
//! Features:
//! - Create and manage narrative agents
//! - Attach to existing personas or generate new ones
//! - Configure behavior policies
//! - Live simulation dashboard
//! - Interaction timeline visualization
//! - Intention/state visualization

import React, { useState, useEffect } from 'react';
import {
  Users,
  Play,
  Pause,
  Square,
  UserPlus,
  Settings,
  Activity,
  TrendingUp,
  AlertCircle,
  CheckCircle2,
  XCircle,
  Loader2,
  Eye,
  EyeOff,
  RefreshCw,
  Zap,
} from 'lucide-react';
import { Card } from '../ui/Card';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';

interface NarrativeAgent {
  agent_id: string;
  persona_id: string;
  current_intention: string;
  session_history: Interaction[];
  behavioral_traits: BehavioralTraits;
  state_awareness: AppState;
  behavior_policy: BehaviorPolicy;
  created_at: string;
}

interface Interaction {
  timestamp: string;
  action: string;
  intention: string;
  request?: any;
  response?: any;
  result: string;
}

interface BehavioralTraits {
  patience: number;
  price_sensitivity: number;
  risk_tolerance: number;
  technical_proficiency: number;
  engagement_level: number;
}

interface AppState {
  current_page?: string;
  cart: CartState;
  authenticated: boolean;
  recent_errors: ErrorEncounter[];
  context: Record<string, any>;
}

interface CartState {
  is_empty: boolean;
  item_count: number;
  total_value: number;
  items: CartItem[];
}

interface CartItem {
  item_id: string;
  name: string;
  price: number;
  quantity: number;
}

interface ErrorEncounter {
  error_type: string;
  message: string;
  timestamp: string;
  count: number;
}

interface BehaviorPolicy {
  policy_type: string;
  description: string;
  rules: PolicyRule[];
}

interface PolicyRule {
  condition: string;
  action: string;
  priority: number;
}

interface SimulateBehaviorResponse {
  next_action: NextAction;
  intention: string;
  reasoning: string;
  agent?: NarrativeAgent;
  tokens_used?: number;
  cost_usd?: number;
}

interface NextAction {
  action_type: string;
  target: string;
  body?: any;
  query_params?: Record<string, string>;
  delay_ms?: number;
}

interface BehavioralSimulatorProps {
  onUsageUpdate?: () => void;
}

export function BehavioralSimulator({ onUsageUpdate }: BehavioralSimulatorProps) {
  const [agents, setAgents] = useState<NarrativeAgent[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const [isCreatingAgent, setIsCreatingAgent] = useState(false);
  const [isSimulating, setIsSimulating] = useState(false);
  const [simulationHistory, setSimulationHistory] = useState<SimulateBehaviorResponse[]>([]);

  // Create agent form state
  const [personaId, setPersonaId] = useState<string>('');
  const [behaviorPolicy, setBehaviorPolicy] = useState<string>('');
  const [generatePersona, setGeneratePersona] = useState(false);

  // Simulation state
  const [currentState, setCurrentState] = useState<AppState>({
    cart: { is_empty: true, item_count: 0, total_value: 0, items: [] },
    authenticated: false,
    recent_errors: [],
    context: {},
  });
  const [triggerEvent, setTriggerEvent] = useState<string>('');

  // Load agents on mount
  useEffect(() => {
    // In a full implementation, this would fetch from API
    // For now, we'll start with empty list
  }, []);

  const handleCreateAgent = async () => {
    if (!personaId && !generatePersona) {
      toast.error('Please select an existing persona or enable persona generation');
      return;
    }

    try {
      setIsCreatingAgent(true);

      const response = await fetch('/__mockforge/api/v1/ai-studio/simulate-behavior/create-agent', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          persona_id: personaId || null,
          behavior_policy: behaviorPolicy || null,
          generate_persona: generatePersona,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();
      setAgents((prev) => [...prev, result.agent]);
      setSelectedAgent(result.agent.agent_id);

      // Reset form
      setPersonaId('');
      setBehaviorPolicy('');
      setGeneratePersona(false);

      toast.success('Agent created successfully');
    } catch (error: any) {
      logger.error('Create agent failed', error);
      toast.error(`Failed to create agent: ${error.message}`);
    } finally {
      setIsCreatingAgent(false);
    }
  };

  const handleSimulate = async () => {
    if (!selectedAgent) {
      toast.error('Please select an agent first');
      return;
    }

    try {
      setIsSimulating(true);

      const response = await fetch('/__mockforge/api/v1/ai-studio/simulate-behavior', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          agent_id: selectedAgent,
          current_state: currentState,
          trigger_event: triggerEvent || null,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();
      setSimulationHistory((prev) => [result, ...prev]);

      // Update agent if returned
      if (result.agent) {
        setAgents((prev) =>
          prev.map((a) => (a.agent_id === result.agent.agent_id ? result.agent : a))
        );
        setCurrentState(result.agent.state_awareness);
      }

      toast.success(`Simulation completed: ${result.intention}`);

      if (onUsageUpdate) {
        onUsageUpdate();
      }
    } catch (error: any) {
      logger.error('Simulation failed', error);
      toast.error(`Simulation failed: ${error.message}`);
    } finally {
      setIsSimulating(false);
    }
  };

  const getIntentionColor = (intention: string) => {
    switch (intention.toLowerCase()) {
      case 'browse':
        return 'text-blue-600 dark:text-blue-400';
      case 'shop':
        return 'text-green-600 dark:text-green-400';
      case 'buy':
        return 'text-purple-600 dark:text-purple-400';
      case 'abandon':
        return 'text-red-600 dark:text-red-400';
      case 'retry':
        return 'text-yellow-600 dark:text-yellow-400';
      default:
        return 'text-gray-600 dark:text-gray-400';
    }
  };

  const getActionIcon = (actionType: string) => {
    switch (actionType.toUpperCase()) {
      case 'GET':
        return <Eye className="w-4 h-4" />;
      case 'POST':
        return <Zap className="w-4 h-4" />;
      case 'NAVIGATE':
        return <RefreshCw className="w-4 h-4" />;
      case 'ABANDON':
        return <XCircle className="w-4 h-4" />;
      default:
        return <Activity className="w-4 h-4" />;
    }
  };

  const selectedAgentData = agents.find((a) => a.agent_id === selectedAgent);

  return (
    <div className="space-y-6">
      {/* Agent Management */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Create Agent Card */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center">
            <UserPlus className="w-5 h-5 mr-2" />
            Create Agent
          </h3>
          <div className="space-y-4">
            <div>
              <Label htmlFor="persona-id">Persona ID (Optional)</Label>
              <Input
                id="persona-id"
                value={personaId}
                onChange={(e) => setPersonaId(e.target.value)}
                placeholder="existing-persona-123"
                className="mt-2"
              />
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Attach to existing persona (primary mode)
              </p>
            </div>

            <div>
              <Label htmlFor="behavior-policy">Behavior Policy (Optional)</Label>
              <Select value={behaviorPolicy} onValueChange={setBehaviorPolicy}>
                <SelectTrigger id="behavior-policy">
                  <SelectValue placeholder="Select policy type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="bargain-hunter">Bargain Hunter</SelectItem>
                  <SelectItem value="power-user">Power User</SelectItem>
                  <SelectItem value="churn-risk">Churn Risk</SelectItem>
                  <SelectItem value="default">Default</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="generate-persona"
                checked={generatePersona}
                onChange={(e) => setGeneratePersona(e.target.checked)}
                className="rounded"
              />
              <Label htmlFor="generate-persona" className="cursor-pointer">
                Generate new persona if needed (secondary mode)
              </Label>
            </div>

            <Button
              onClick={handleCreateAgent}
              disabled={isCreatingAgent || (!personaId && !generatePersona)}
              className="w-full"
            >
              {isCreatingAgent ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <UserPlus className="w-4 h-4 mr-2" />
                  Create Agent
                </>
              )}
            </Button>
          </div>
        </Card>

        {/* Agent List Card */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center">
            <Users className="w-5 h-5 mr-2" />
            Active Agents ({agents.length})
          </h3>
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {agents.length === 0 ? (
              <p className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
                No agents created yet
              </p>
            ) : (
              agents.map((agent) => (
                <div
                  key={agent.agent_id}
                  className={`p-3 rounded-lg border cursor-pointer transition-colors ${
                    selectedAgent === agent.agent_id
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'
                  }`}
                  onClick={() => setSelectedAgent(agent.agent_id)}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-medium text-sm">{agent.agent_id.substring(0, 16)}...</div>
                      <div className="text-xs text-gray-600 dark:text-gray-400">
                        Persona: {agent.persona_id.substring(0, 16)}...
                      </div>
                    </div>
                    <div className="text-right">
                      <div className={`text-xs font-medium ${getIntentionColor(agent.current_intention)}`}>
                        {agent.current_intention}
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">
                        {agent.session_history.length} interactions
                      </div>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </Card>
      </div>

      {/* Simulation Controls */}
      {selectedAgentData && (
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center">
            <Play className="w-5 h-5 mr-2" />
            Simulation Controls
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <Label htmlFor="trigger-event">Trigger Event (Optional)</Label>
              <Select value={triggerEvent} onValueChange={setTriggerEvent}>
                <SelectTrigger id="trigger-event">
                  <SelectValue placeholder="Select trigger event" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">None</SelectItem>
                  <SelectItem value="error_500">Error 500</SelectItem>
                  <SelectItem value="timeout">Timeout</SelectItem>
                  <SelectItem value="cart_empty">Cart Empty</SelectItem>
                  <SelectItem value="payment_failed">Payment Failed</SelectItem>
                  <SelectItem value="validation_error">Validation Error</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-end">
              <Button
                onClick={handleSimulate}
                disabled={isSimulating}
                className="w-full"
                size="lg"
              >
                {isSimulating ? (
                  <>
                    <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    Simulating...
                  </>
                ) : (
                  <>
                    <Play className="w-4 h-4 mr-2" />
                    Simulate Behavior
                  </>
                )}
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* Agent State Display */}
      {selectedAgentData && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Current State */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4">Current State</h3>
            <div className="space-y-3">
              <div>
                <div className="text-sm font-medium mb-1">Intention</div>
                <div className={`text-lg ${getIntentionColor(selectedAgentData.current_intention)}`}>
                  {selectedAgentData.current_intention}
                </div>
              </div>
              <div>
                <div className="text-sm font-medium mb-1">Cart</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  {selectedAgentData.state_awareness.cart.is_empty
                    ? 'Empty'
                    : `${selectedAgentData.state_awareness.cart.item_count} items, $${selectedAgentData.state_awareness.cart.total_value.toFixed(2)}`}
                </div>
              </div>
              <div>
                <div className="text-sm font-medium mb-1">Recent Errors</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  {selectedAgentData.state_awareness.recent_errors.length} errors
                </div>
              </div>
              <div>
                <div className="text-sm font-medium mb-1">Behavioral Traits</div>
                <div className="space-y-1 text-xs">
                  <div>Patience: {(selectedAgentData.behavioral_traits.patience * 100).toFixed(0)}%</div>
                  <div>Price Sensitivity: {(selectedAgentData.behavioral_traits.price_sensitivity * 100).toFixed(0)}%</div>
                  <div>Engagement: {(selectedAgentData.behavioral_traits.engagement_level * 100).toFixed(0)}%</div>
                </div>
              </div>
            </div>
          </Card>

          {/* Behavior Policy */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4">Behavior Policy</h3>
            <div className="space-y-3">
              <div>
                <div className="text-sm font-medium mb-1">Policy Type</div>
                <div className="text-sm text-gray-600 dark:text-gray-400 capitalize">
                  {selectedAgentData.behavior_policy.policy_type.replace('-', ' ')}
                </div>
              </div>
              <div>
                <div className="text-sm font-medium mb-1">Description</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  {selectedAgentData.behavior_policy.description}
                </div>
              </div>
              {selectedAgentData.behavior_policy.rules.length > 0 && (
                <div>
                  <div className="text-sm font-medium mb-2">Rules</div>
                  <div className="space-y-1">
                    {selectedAgentData.behavior_policy.rules.map((rule, idx) => (
                      <div key={idx} className="text-xs p-2 bg-gray-50 dark:bg-gray-800 rounded">
                        <div className="font-medium">{rule.condition}</div>
                        <div className="text-gray-600 dark:text-gray-400">→ {rule.action}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>
      )}

      {/* Simulation History */}
      {simulationHistory.length > 0 && (
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center">
            <Activity className="w-5 h-5 mr-2" />
            Simulation Timeline ({simulationHistory.length})
          </h3>
          <div className="space-y-4">
            {simulationHistory.map((sim, idx) => (
              <div
                key={idx}
                className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg"
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center space-x-3">
                    {getActionIcon(sim.next_action.action_type)}
                    <div>
                      <div className="font-medium">{sim.next_action.action_type} {sim.next_action.target}</div>
                      <div className="text-xs text-gray-600 dark:text-gray-400">
                        {new Date().toLocaleTimeString()}
                      </div>
                    </div>
                  </div>
                  <div className={`text-sm font-medium ${getIntentionColor(sim.intention)}`}>
                    {sim.intention}
                  </div>
                </div>
                <div className="text-sm text-gray-700 dark:text-gray-300 mb-2">
                  {sim.reasoning}
                </div>
                {sim.tokens_used && (
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    Tokens: {sim.tokens_used.toLocaleString()} • Cost: ${sim.cost_usd?.toFixed(4) || '0.0000'}
                  </div>
                )}
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
