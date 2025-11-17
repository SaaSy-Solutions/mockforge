//! Voice + LLM Interface Page
//!
//! This page provides a conversational interface for creating mocks using
//! natural language voice commands powered by LLM.

import React, { useState } from 'react';
import { VoiceInput, VoiceCommandResult } from '../components/voice/VoiceInput';
import { NLHookEditor, HookResult } from '../components/hooks/NLHookEditor';
import {
  WorkspaceScenarioCreator,
  WorkspaceScenarioResult,
} from '../components/workspace/WorkspaceScenarioCreator';
import { Card } from '../components/ui/Card';
import { Mic, Sparkles, FileCode, Code2, Building2 } from 'lucide-react';

type TabType = 'api' | 'hooks' | 'scenarios';

export function VoicePage() {
  const [activeTab, setActiveTab] = useState<TabType>('api');
  const [history, setHistory] = useState<VoiceCommandResult[]>([]);
  const [hookHistory, setHookHistory] = useState<HookResult[]>([]);
  const [scenarioHistory, setScenarioHistory] = useState<WorkspaceScenarioResult[]>([]);

  const handleCommandProcessed = (result: VoiceCommandResult) => {
    setHistory(prev => [result, ...prev].slice(0, 10)); // Keep last 10 commands
  };

  const handleHookGenerated = (result: HookResult) => {
    setHookHistory(prev => [result, ...prev].slice(0, 10)); // Keep last 10 hooks
  };

  const handleScenarioCreated = (result: WorkspaceScenarioResult) => {
    setScenarioHistory(prev => [result, ...prev].slice(0, 10)); // Keep last 10 scenarios
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <Mic className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold">Voice + LLM Interface</h1>
        </div>
        <p className="text-muted-foreground">
          Build mocks conversationally using natural language commands powered by AI.
          Speak or type your requirements, and we'll generate an OpenAPI specification.
        </p>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          <button
            onClick={() => setActiveTab('api')}
            className={`
              py-4 px-1 border-b-2 font-medium text-sm
              ${
                activeTab === 'api'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }
            `}
          >
            <div className="flex items-center gap-2">
              <FileCode className="w-5 h-5" />
              API Generation
            </div>
          </button>
          <button
            onClick={() => setActiveTab('hooks')}
            className={`
              py-4 px-1 border-b-2 font-medium text-sm
              ${
                activeTab === 'hooks'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }
            `}
          >
            <div className="flex items-center gap-2">
              <Code2 className="w-5 h-5" />
              Hook Transpilation
            </div>
          </button>
          <button
            onClick={() => setActiveTab('scenarios')}
            className={`
              py-4 px-1 border-b-2 font-medium text-sm
              ${
                activeTab === 'scenarios'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }
            `}
          >
            <div className="flex items-center gap-2">
              <Building2 className="w-5 h-5" />
              Workspace Scenarios
            </div>
          </button>
        </nav>
      </div>

      {/* Tab Content */}
      {activeTab === 'api' && (
        <>
          {/* Features */}
          <div className="grid md:grid-cols-3 gap-4">
            <Card className="p-4">
              <div className="flex items-center gap-3 mb-2">
                <Mic className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">Voice Input</h3>
              </div>
              <p className="text-sm text-muted-foreground">
                Use your microphone to speak commands naturally. Works with Chrome, Edge, and Safari.
              </p>
            </Card>
            <Card className="p-4">
              <div className="flex items-center gap-3 mb-2">
                <Sparkles className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">AI-Powered</h3>
              </div>
              <p className="text-sm text-muted-foreground">
                LLM interprets your commands and extracts API requirements automatically.
              </p>
            </Card>
            <Card className="p-4">
              <div className="flex items-center gap-3 mb-2">
                <FileCode className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">OpenAPI Output</h3>
              </div>
              <p className="text-sm text-muted-foreground">
                Generates valid OpenAPI 3.0 specifications ready to use with MockForge.
              </p>
            </Card>
          </div>

          {/* Voice Input Component */}
          <VoiceInput onCommandProcessed={handleCommandProcessed} />
        </>
      )}

      {activeTab === 'hooks' && (
        <>
          {/* Hook Editor */}
          <Card className="p-6">
            <div className="space-y-4">
              <div>
                <h2 className="text-xl font-semibold mb-2">Natural Language Hook Editor</h2>
                <p className="text-muted-foreground">
                  Describe hook logic in natural language and get transpiled hook configurations ready to use
                  in chaos orchestration scenarios.
                </p>
              </div>
              <NLHookEditor onHookGenerated={handleHookGenerated} />
            </div>
          </Card>

          {/* Hook History */}
          {hookHistory.length > 0 && (
            <div className="space-y-2">
              <h2 className="text-xl font-semibold">Recent Hooks</h2>
              <div className="space-y-2">
                {hookHistory.map((item, index) => (
                  <Card key={index} className="p-4">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="font-medium mb-1">{item.description}</div>
                        {item.error ? (
                          <div className="text-sm text-red-600">{item.error}</div>
                        ) : (
                          <div className="text-sm text-muted-foreground">
                            Hook configuration generated successfully
                          </div>
                        )}
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          )}

          {/* Hook Examples */}
          <Card className="p-6">
            <h2 className="text-xl font-semibold mb-4">Example Hook Descriptions</h2>
            <div className="grid md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <div className="font-medium">VIP User Hook</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "For users flagged as VIP, webhooks should fire instantly but payments fail 5% of the time"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">Conditional Logging</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "When a request fails, log the error and send a notification webhook"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">Metric Recording</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "After each successful payment, record a metric and trigger a webhook"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">Complex Condition</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "If the user is a premium member and the order total is greater than $100, fire a webhook
                  instantly"
                </div>
              </div>
            </div>
          </Card>
        </>
      )}

      {activeTab === 'scenarios' && (
        <>
          {/* Workspace Scenario Creator */}
          <Card className="p-6">
            <div className="space-y-4">
              <div>
                <h2 className="text-xl font-semibold mb-2">Chat-Driven Workspace Scenarios</h2>
                <p className="text-muted-foreground">
                  Create complete workspace scenarios with APIs, chaos configurations, and initial
                  data from natural language descriptions.
                </p>
              </div>
              <WorkspaceScenarioCreator onScenarioCreated={handleScenarioCreated} />
            </div>
          </Card>

          {/* Scenario History */}
          {scenarioHistory.length > 0 && (
            <div className="space-y-2">
              <h2 className="text-xl font-semibold">Recent Scenarios</h2>
              <div className="space-y-2">
                {scenarioHistory.map((item, index) => (
                  <Card key={index} className="p-4">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="font-medium mb-1">{item.description}</div>
                        {item.error ? (
                          <div className="text-sm text-red-600">{item.error}</div>
                        ) : item.scenario ? (
                          <div className="text-sm text-muted-foreground">
                            {item.scenario.name} • {item.scenario.config_summary.endpoint_count}{' '}
                            endpoints • {item.scenario.config_summary.chaos_characteristic_count}{' '}
                            chaos characteristics
                          </div>
                        ) : null}
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          )}

          {/* Scenario Examples */}
          <Card className="p-6">
            <h2 className="text-xl font-semibold mb-4">Example Scenario Descriptions</h2>
            <div className="grid md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <div className="font-medium">Banking Scenario</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "Create a workspace that simulates a bank with flaky foreign exchange rates and
                  slow KYC, with 3 existing users and 5 open disputes"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">E-commerce with Chaos</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "Create an e-commerce workspace with high latency on checkout, 10% payment
                  failures, 20 products, and 5 users with active carts"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">Healthcare API</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "Simulate a healthcare API with slow patient lookup, occasional timeouts, 50
                  patients, and 10 appointments"
                </div>
              </div>
              <div className="space-y-2">
                <div className="font-medium">Social Media Platform</div>
                <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
                  "Create a social media workspace with flaky feed updates, rate limiting on posts,
                  100 users, 500 posts, and 1000 comments"
                </div>
              </div>
            </div>
          </Card>
        </>
      )}

      {/* Command History */}
      {history.length > 0 && (
        <div className="space-y-2">
          <h2 className="text-xl font-semibold">Recent Commands</h2>
          <div className="space-y-2">
            {history.map((item, index) => (
              <Card key={index} className="p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="font-medium mb-1">{item.command}</div>
                    <div className="text-sm text-muted-foreground">
                      {item.parsed.apiType} • {item.parsed.endpoints} endpoints • {item.parsed.models} models
                    </div>
                  </div>
                  {item.spec && (
                    <div className="text-xs text-muted-foreground">
                      {item.spec.title} v{item.spec.version}
                    </div>
                  )}
                </div>
              </Card>
            ))}
          </div>
        </div>
      )}

      {/* Examples */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Example Commands</h2>
        <div className="grid md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="font-medium">Simple API</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create a todo API with endpoints for listing, creating, and updating tasks"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">E-commerce</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create an e-commerce API with products, users, and a checkout flow"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">With Models</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Build a blog API with posts, comments, and user authentication"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">Complex</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create a social media API with users, posts, likes, and a feed endpoint"
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
