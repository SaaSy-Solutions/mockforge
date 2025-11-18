//! MockForge AI Studio - Unified AI Copilot
//!
//! This page provides a unified interface for all AI-powered features in MockForge,
//! including natural language mock generation, AI-guided debugging, persona generation,
//! and artifact freezing.

import React, { useState, useEffect } from 'react';
import {
  Brain,
  MessageSquare,
  Code2,
  Bug,
  User,
  Download,
  Settings,
  TrendingUp,
  DollarSign,
  Zap,
} from 'lucide-react';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/button';
import { apiService } from '../services/api';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';

type TabType = 'chat' | 'generate' | 'debug' | 'personas' | 'budget';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  intent?: string;
  data?: any;
}

interface UsageStats {
  tokens_used: number;
  cost_usd: number;
  calls_made: number;
  budget_limit: number;
  usage_percentage: number;
}

export function AIStudioPage() {
  const [activeTab, setActiveTab] = useState<TabType>('chat');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [usageStats, setUsageStats] = useState<UsageStats | null>(null);
  const [loadingStats, setLoadingStats] = useState(true);

  // Load usage stats on mount
  useEffect(() => {
    loadUsageStats();
  }, []);

  const loadUsageStats = async () => {
    try {
      setLoadingStats(true);
      // Load usage stats from API
      const response = await fetch('/__mockforge/ai-studio/usage');
      if (response.ok) {
        const result = await response.json();
        if (result.success && result.data) {
          setUsageStats({
            tokens_used: result.data.tokens_used || 0,
            cost_usd: result.data.cost_usd || 0.0,
            calls_made: result.data.calls_made || 0,
            budget_limit: result.data.budget_limit || 100000,
            usage_percentage: result.data.usage_percentage || 0.0,
          });
        }
      }
    } catch (err) {
      logger.error('Failed to load usage stats', err);
    } finally {
      setLoadingStats(false);
    }
  };

  const handleSendMessage = async () => {
    if (!inputMessage.trim() || isProcessing) return;

    const userMessage: ChatMessage = {
      role: 'user',
      content: inputMessage,
      timestamp: new Date(),
    };

    setChatMessages(prev => [...prev, userMessage]);
    setInputMessage('');
    setIsProcessing(true);

    try {
      const response = await fetch('/__mockforge/ai-studio/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          message: inputMessage,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to process chat message');
      }

      const result = await response.json();
      if (result.success && result.data) {
        const assistantMessage: ChatMessage = {
          role: 'assistant',
          content: result.data.message,
          timestamp: new Date(),
          intent: result.data.intent,
          data: result.data.data,
        };
        setChatMessages(prev => [...prev, assistantMessage]);

        // If a spec was generated, show a success toast with download option
        const hasSpec = result.data.data?.spec || (result.data.data?.type === 'openapi_spec' && result.data.data?.spec);
        if (hasSpec && (result.data.intent === 'generate_mock' || result.data.intent === 'GenerateMock')) {
          toast.success('Mock API generated successfully!', {
            description: 'You can preview and download the OpenAPI spec from the chat.',
          });
        }
      } else {
        throw new Error(result.error || 'Unknown error');
      }
    } catch (err) {
      logger.error('Failed to send chat message', err);
      toast.error('Failed to process message. Please try again.');
      const errorMessage: ChatMessage = {
        role: 'assistant',
        content: 'Sorry, I encountered an error processing your message. Please try again.',
        timestamp: new Date(),
      };
      setChatMessages(prev => [...prev, errorMessage]);
    } finally {
      setIsProcessing(false);
      loadUsageStats(); // Refresh usage stats
    }
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Brain className="h-8 w-8 text-primary" />
            <div>
              <h1 className="text-3xl font-bold">AI Studio</h1>
              <p className="text-muted-foreground">
                Unified AI Copilot for all MockForge AI features
              </p>
            </div>
          </div>
          {/* Usage Stats Widget */}
          {usageStats && (
            <Card className="p-4 min-w-[200px]">
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Tokens Used</span>
                  <span className="font-medium">{usageStats.tokens_used.toLocaleString()}</span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Cost</span>
                  <span className="font-medium">${usageStats.cost_usd.toFixed(4)}</span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-primary h-2 rounded-full transition-all"
                    style={{ width: `${Math.min(usageStats.usage_percentage * 100, 100)}%` }}
                  />
                </div>
              </div>
            </Card>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          {[
            { id: 'chat' as TabType, label: 'Chat', icon: MessageSquare },
            { id: 'generate' as TabType, label: 'Generate', icon: Code2 },
            { id: 'debug' as TabType, label: 'Debug', icon: Bug },
            { id: 'personas' as TabType, label: 'Personas', icon: User },
            { id: 'budget' as TabType, label: 'Budget', icon: DollarSign },
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`
                py-4 px-1 border-b-2 font-medium text-sm flex items-center gap-2
                ${
                  activeTab === tab.id
                    ? 'border-primary text-primary'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }
              `}
            >
              <tab.icon className="w-5 h-5" />
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      {activeTab === 'chat' && (
        <div className="space-y-4">
          {/* Chat Interface */}
          <Card className="p-6">
            <div className="space-y-4">
              <div className="h-96 overflow-y-auto space-y-4 p-4 bg-gray-50 rounded-lg">
                {chatMessages.length === 0 ? (
                  <div className="text-center text-muted-foreground py-12">
                    <Brain className="h-12 w-12 mx-auto mb-4 opacity-50" />
                    <p className="text-lg font-medium mb-2">Welcome to AI Studio</p>
                    <p className="text-sm">
                      Ask me to generate mocks, debug tests, create personas, or analyze contracts.
                    </p>
                    <div className="mt-6 space-y-2 text-left max-w-md mx-auto">
                      <p className="text-sm font-medium">Try asking:</p>
                      <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
                        <li>"Create a user API with CRUD operations"</li>
                        <li>"Why did my test fail?"</li>
                        <li>"Generate a premium customer persona"</li>
                        <li>"Run contract diff analysis"</li>
                      </ul>
                    </div>
                  </div>
                ) : (
                  chatMessages.map((msg, idx) => (
                    <div
                      key={idx}
                      className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
                    >
                      <div
                        className={`max-w-[80%] rounded-lg p-3 ${
                          msg.role === 'user'
                            ? 'bg-primary text-primary-foreground'
                            : 'bg-white border border-gray-200'
                        }`}
                      >
                        <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                        {msg.intent && (
                          <p className="text-xs mt-1 opacity-70">Intent: {msg.intent}</p>
                        )}
                        {(() => {
                          const spec = msg.data?.spec;
                          if (!spec) return null;
                          const specStr = JSON.stringify(spec, null, 2);
                          return (
                            <div className="mt-3 pt-3 border-t border-gray-200">
                              <div className="flex items-center justify-between mb-2">
                                <span className="text-xs font-medium">Generated OpenAPI Spec</span>
                                <div className="flex gap-2">
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    onClick={async () => {
                                      try {
                                        const response = await fetch('/__mockforge/ai-studio/freeze', {
                                          method: 'POST',
                                          headers: {
                                            'Content-Type': 'application/json',
                                          },
                                          body: JSON.stringify({
                                            artifact_type: 'mock',
                                            content: spec,
                                            format: 'yaml',
                                          }),
                                        });
                                        if (response.ok) {
                                          const result = await response.json();
                                          if (result.success) {
                                            toast.success('Artifact frozen successfully!', {
                                              description: `Saved to ${result.data.path}`,
                                            });
                                          }
                                        }
                                      } catch (err) {
                                        logger.error('Failed to freeze artifact', err);
                                        toast.error('Failed to freeze artifact');
                                      }
                                    }}
                                  >
                                    <Download className="h-3 w-3 mr-1" />
                                    Freeze
                                  </Button>
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    onClick={() => {
                                      const blob = new Blob([specStr], {
                                        type: 'application/json',
                                      });
                                      const url = URL.createObjectURL(blob);
                                      const a = document.createElement('a');
                                      a.href = url;
                                      a.download = 'openapi-spec.json';
                                      a.click();
                                      URL.revokeObjectURL(url);
                                    }}
                                  >
                                    <Download className="h-3 w-3 mr-1" />
                                    Download
                                  </Button>
                                </div>
                              </div>
                              <pre className="text-xs bg-gray-50 p-2 rounded overflow-x-auto max-h-40">
                                {specStr.substring(0, 500)}
                                {specStr.length > 500 ? '...' : ''}
                              </pre>
                            </div>
                          );
                        })()}
                      </div>
                    </div>
                  ))
                )}
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={inputMessage}
                  onChange={e => setInputMessage(e.target.value)}
                  onKeyPress={e => e.key === 'Enter' && handleSendMessage()}
                  placeholder="Type your message..."
                  className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                  disabled={isProcessing}
                />
                <Button onClick={handleSendMessage} disabled={isProcessing || !inputMessage.trim()}>
                  {isProcessing ? 'Processing...' : 'Send'}
                </Button>
              </div>
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'generate' && (
        <div className="space-y-4">
          <Card className="p-6">
            <div className="space-y-4">
              <div>
                <h2 className="text-xl font-semibold mb-2">Generate Mocks</h2>
                <p className="text-muted-foreground">
                  Describe your API in natural language and we'll generate a complete OpenAPI specification.
                </p>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={inputMessage}
                  onChange={e => setInputMessage(e.target.value)}
                  onKeyPress={e => e.key === 'Enter' && handleSendMessage()}
                  placeholder="e.g., Create a user API with CRUD operations for managing users"
                  className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                  disabled={isProcessing}
                />
                <Button onClick={handleSendMessage} disabled={isProcessing || !inputMessage.trim()}>
                  {isProcessing ? 'Generating...' : 'Generate'}
                </Button>
              </div>
            </div>
          </Card>

          {/* Example prompts */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4">Example Prompts</h3>
            <div className="grid md:grid-cols-2 gap-4">
              {[
                {
                  title: 'Simple CRUD API',
                  prompt: 'Create a todo API with endpoints for listing, creating, updating, and deleting tasks',
                },
                {
                  title: 'E-commerce API',
                  prompt: 'Create an e-commerce API with products, users, shopping cart, and checkout flow',
                },
                {
                  title: 'Blog API',
                  prompt: 'Create a blog API with posts, comments, and user authentication',
                },
                {
                  title: 'Social Media API',
                  prompt: 'Create a social media API with users, posts, likes, and a feed endpoint',
                },
              ].map((example, idx) => (
                <div
                  key={idx}
                  className="p-4 border border-gray-200 rounded-lg hover:border-primary cursor-pointer transition-colors"
                  onClick={() => {
                    setInputMessage(example.prompt);
                    setActiveTab('chat'); // Switch to chat tab to use the prompt
                  }}
                >
                  <div className="font-medium mb-1">{example.title}</div>
                  <div className="text-sm text-muted-foreground">{example.prompt}</div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'debug' && (
        <div className="space-y-4">
          <Card className="p-6">
            <div className="space-y-4">
              <div>
                <h2 className="text-xl font-semibold mb-2">AI-Guided Debugging</h2>
                <p className="text-muted-foreground">
                  Paste your test failure logs and get AI-powered analysis with specific suggestions
                  for fixing the issue. The analyzer identifies root causes and links to relevant
                  mock configurations (personas, reality settings, contracts).
                </p>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Test Failure Logs</label>
                <textarea
                  value={inputMessage}
                  onChange={e => setInputMessage(e.target.value)}
                  placeholder="Paste your test failure logs here...&#10;&#10;Example:&#10;GET /api/users/123&#10;Status: 404&#10;Error: User not found"
                  className="w-full h-48 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary font-mono text-sm"
                  disabled={isProcessing}
                />
              </div>
              <div className="flex gap-2">
                <Button
                  onClick={async () => {
                    if (!inputMessage.trim() || isProcessing) return;

                    // Prepend "debug" to help intent detection
                    const debugMessage = `debug test failure:\n\n${inputMessage}`;
                    const userMessage: ChatMessage = {
                      role: 'user',
                      content: inputMessage,
                      timestamp: new Date(),
                    };

                    setChatMessages(prev => [...prev, userMessage]);
                    setInputMessage('');
                    setIsProcessing(true);

                    try {
                      const response = await fetch('/__mockforge/ai-studio/chat', {
                        method: 'POST',
                        headers: {
                          'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                          message: debugMessage,
                        }),
                      });

                      if (!response.ok) {
                        throw new Error('Failed to analyze test failure');
                      }

                      const result = await response.json();
                      if (result.success && result.data) {
                        const assistantMessage: ChatMessage = {
                          role: 'assistant',
                          content: result.data.message,
                          timestamp: new Date(),
                          intent: result.data.intent,
                          data: result.data.data,
                        };
                        setChatMessages(prev => [...prev, assistantMessage]);
                      } else {
                        throw new Error(result.error || 'Unknown error');
                      }
                    } catch (err) {
                      logger.error('Failed to analyze test failure', err);
                      toast.error('Failed to analyze test failure. Please try again.');
                      const errorMessage: ChatMessage = {
                        role: 'assistant',
                        content: 'Sorry, I encountered an error analyzing the test failure. Please try again.',
                        timestamp: new Date(),
                      };
                      setChatMessages(prev => [...prev, errorMessage]);
                    } finally {
                      setIsProcessing(false);
                      loadUsageStats();
                    }
                  }}
                  disabled={isProcessing || !inputMessage.trim()}
                >
                  {isProcessing ? 'Analyzing...' : 'Analyze Failure'}
                </Button>
              </div>
            </div>
          </Card>

          {/* Debug results from chat */}
          {chatMessages
            .filter(msg => msg.intent === 'debug_test' || msg.intent === 'DebugTest')
            .map((msg, idx) => (
              <Card key={idx} className="p-6">
                <div className="space-y-4">
                  <div>
                    <h3 className="text-lg font-semibold mb-2">Analysis Results</h3>
                    <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                  </div>
                  {msg.data?.root_cause && (
                    <div>
                      <h4 className="font-medium mb-2">Root Cause</h4>
                      <div className="p-3 bg-yellow-50 border border-yellow-200 rounded-lg">
                        <p className="text-sm">{msg.data.root_cause}</p>
                      </div>
                    </div>
                  )}
                  {msg.data?.suggestions && Array.isArray(msg.data.suggestions) && msg.data.suggestions.length > 0 && (
                    <div>
                      <h4 className="font-medium mb-2">Suggestions</h4>
                      <div className="space-y-2">
                        {msg.data.suggestions.map((suggestion: any, sidx: number) => (
                          <div key={sidx} className="p-3 bg-blue-50 border border-blue-200 rounded-lg">
                            <div className="font-medium text-sm mb-1">{suggestion.title || `Suggestion ${sidx + 1}`}</div>
                            <p className="text-sm text-muted-foreground mb-2">
                              {suggestion.description || suggestion.action}
                            </p>
                            {suggestion.config_path && (
                              <div className="text-xs text-muted-foreground">
                                Config: <code className="bg-white px-1 rounded">{suggestion.config_path}</code>
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                  {msg.data?.related_configs && Array.isArray(msg.data.related_configs) && msg.data.related_configs.length > 0 && (
                    <div>
                      <h4 className="font-medium mb-2">Related Configurations</h4>
                      <div className="flex flex-wrap gap-2">
                        {msg.data.related_configs.map((config: string, cidx: number) => (
                          <span
                            key={cidx}
                            className="px-2 py-1 bg-gray-100 text-gray-700 rounded text-sm"
                          >
                            {config}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </Card>
            ))}
        </div>
      )}

      {activeTab === 'personas' && (
        <div className="space-y-4">
          <Card className="p-6">
            <div className="space-y-4">
              <div>
                <h2 className="text-xl font-semibold mb-2">Persona Generation</h2>
                <p className="text-muted-foreground">
                  Generate realistic personas with traits, backstories, and lifecycle states from
                  natural language descriptions. Perfect for creating consistent test data.
                </p>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Persona Description</label>
                <textarea
                  value={inputMessage}
                  onChange={e => setInputMessage(e.target.value)}
                  placeholder="e.g., Create a premium customer persona with high spending, active subscription, and priority support"
                  className="w-full h-32 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                  disabled={isProcessing}
                />
              </div>
              <div className="flex gap-2">
                <Button
                  onClick={async () => {
                    if (!inputMessage.trim() || isProcessing) return;

                    const personaMessage = `generate persona: ${inputMessage}`;
                    const userMessage: ChatMessage = {
                      role: 'user',
                      content: inputMessage,
                      timestamp: new Date(),
                    };

                    setChatMessages(prev => [...prev, userMessage]);
                    setInputMessage('');
                    setIsProcessing(true);

                    try {
                      const response = await fetch('/__mockforge/ai-studio/chat', {
                        method: 'POST',
                        headers: {
                          'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                          message: personaMessage,
                        }),
                      });

                      if (!response.ok) {
                        throw new Error('Failed to generate persona');
                      }

                      const result = await response.json();
                      if (result.success && result.data) {
                        const assistantMessage: ChatMessage = {
                          role: 'assistant',
                          content: result.data.message,
                          timestamp: new Date(),
                          intent: result.data.intent,
                          data: result.data.data,
                        };
                        setChatMessages(prev => [...prev, assistantMessage]);
                        toast.success('Persona generated successfully!');
                      } else {
                        throw new Error(result.error || 'Unknown error');
                      }
                    } catch (err) {
                      logger.error('Failed to generate persona', err);
                      toast.error('Failed to generate persona. Please try again.');
                      const errorMessage: ChatMessage = {
                        role: 'assistant',
                        content: 'Sorry, I encountered an error generating the persona. Please try again.',
                        timestamp: new Date(),
                      };
                      setChatMessages(prev => [...prev, errorMessage]);
                    } finally {
                      setIsProcessing(false);
                      loadUsageStats();
                    }
                  }}
                  disabled={isProcessing || !inputMessage.trim()}
                >
                  {isProcessing ? 'Generating...' : 'Generate Persona'}
                </Button>
              </div>
            </div>
          </Card>

          {/* Example persona descriptions */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4">Example Persona Descriptions</h3>
            <div className="grid md:grid-cols-2 gap-4">
              {[
                {
                  title: 'Premium Customer',
                  description: 'Create a premium customer persona with high spending, active subscription, and priority support',
                },
                {
                  title: 'Churned User',
                  description: 'Generate a churned customer persona who cancelled their subscription due to price concerns',
                },
                {
                  title: 'Trial User',
                  description: 'Create a trial user persona with 7 days remaining, no payment method, and high engagement',
                },
                {
                  title: 'Power User',
                  description: 'Generate a power user persona with extensive feature usage, multiple integrations, and enterprise tier',
                },
              ].map((example, idx) => (
                <div
                  key={idx}
                  className="p-4 border border-gray-200 rounded-lg hover:border-primary cursor-pointer transition-colors"
                  onClick={() => {
                    setInputMessage(example.description);
                  }}
                >
                  <div className="font-medium mb-1">{example.title}</div>
                  <div className="text-sm text-muted-foreground">{example.description}</div>
                </div>
              ))}
            </div>
          </Card>

          {/* Generated personas from chat */}
          {chatMessages
            .filter(msg => msg.intent === 'generate_persona' || msg.intent === 'GeneratePersona')
            .map((msg, idx) => (
              <Card key={idx} className="p-6">
                <div className="space-y-4">
                  <div>
                    <h3 className="text-lg font-semibold mb-2">Generated Persona</h3>
                    <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                  </div>
                  {msg.data?.persona && (
                    <div className="space-y-3">
                      {msg.data.persona.name && (
                        <div>
                          <h4 className="font-medium mb-1">Name</h4>
                          <p className="text-sm">{msg.data.persona.name}</p>
                        </div>
                      )}
                      {msg.data.persona.id && (
                        <div>
                          <h4 className="font-medium mb-1">ID</h4>
                          <code className="text-sm bg-gray-100 px-2 py-1 rounded">
                            {msg.data.persona.id}
                          </code>
                        </div>
                      )}
                      {msg.data.persona.domain && (
                        <div>
                          <h4 className="font-medium mb-1">Domain</h4>
                          <span className="text-sm px-2 py-1 bg-blue-100 text-blue-700 rounded">
                            {msg.data.persona.domain}
                          </span>
                        </div>
                      )}
                      {msg.data.persona.traits && Object.keys(msg.data.persona.traits).length > 0 && (
                        <div>
                          <h4 className="font-medium mb-2">Traits</h4>
                          <div className="grid grid-cols-2 gap-2">
                            {Object.entries(msg.data.persona.traits).map(([key, value]) => (
                              <div key={key} className="p-2 bg-gray-50 rounded">
                                <div className="text-xs font-medium text-gray-600">{key}</div>
                                <div className="text-sm">{String(value)}</div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                      {msg.data.persona.backstory && (
                        <div>
                          <h4 className="font-medium mb-1">Backstory</h4>
                          <p className="text-sm text-muted-foreground">{msg.data.persona.backstory}</p>
                        </div>
                      )}
                      {msg.data.persona.lifecycle_state && (
                        <div>
                          <h4 className="font-medium mb-1">Lifecycle State</h4>
                          <span className="text-sm px-2 py-1 bg-green-100 text-green-700 rounded">
                            {msg.data.persona.lifecycle_state}
                          </span>
                        </div>
                      )}
                      <div className="pt-3 border-t flex gap-2">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={async () => {
                            try {
                              const response = await fetch('/__mockforge/ai-studio/freeze', {
                                method: 'POST',
                                headers: {
                                  'Content-Type': 'application/json',
                                },
                                body: JSON.stringify({
                                  artifact_type: 'persona',
                                  content: msg.data.persona,
                                  format: 'yaml',
                                }),
                              });
                              if (response.ok) {
                                const result = await response.json();
                                if (result.success) {
                                  toast.success('Persona frozen successfully!', {
                                    description: `Saved to ${result.data.path}`,
                                  });
                                }
                              }
                            } catch (err) {
                              logger.error('Failed to freeze persona', err);
                              toast.error('Failed to freeze persona');
                            }
                          }}
                        >
                          <Download className="h-3 w-3 mr-1" />
                          Freeze
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => {
                            const personaJson = JSON.stringify(msg.data.persona, null, 2);
                            const blob = new Blob([personaJson], {
                              type: 'application/json',
                            });
                            const url = URL.createObjectURL(blob);
                            const a = document.createElement('a');
                            a.href = url;
                            a.download = `persona-${msg.data.persona.name || 'generated'}.json`;
                            a.click();
                            URL.revokeObjectURL(url);
                          }}
                        >
                          <Download className="h-3 w-3 mr-1" />
                          Download
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </Card>
            ))}
        </div>
      )}

      {activeTab === 'budget' && (
        <Card className="p-6">
          <div className="space-y-4">
            <h2 className="text-xl font-semibold">Budget & Usage</h2>
            {loadingStats ? (
              <p className="text-muted-foreground">Loading usage statistics...</p>
            ) : usageStats ? (
              <div className="space-y-4">
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <p className="text-sm text-muted-foreground">Tokens Used</p>
                    <p className="text-2xl font-bold">{usageStats.tokens_used.toLocaleString()}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Cost (USD)</p>
                    <p className="text-2xl font-bold">${usageStats.cost_usd.toFixed(4)}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Calls Made</p>
                    <p className="text-2xl font-bold">{usageStats.calls_made}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Usage</p>
                    <p className="text-2xl font-bold">
                      {(usageStats.usage_percentage * 100).toFixed(1)}%
                    </p>
                  </div>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground mb-2">Budget Progress</p>
                  <div className="w-full bg-gray-200 rounded-full h-4">
                    <div
                      className="bg-primary h-4 rounded-full transition-all"
                      style={{ width: `${Math.min(usageStats.usage_percentage * 100, 100)}%` }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    {usageStats.tokens_used.toLocaleString()} / {usageStats.budget_limit.toLocaleString()} tokens
                  </p>
                </div>
              </div>
            ) : (
              <p className="text-muted-foreground">Failed to load usage statistics</p>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
