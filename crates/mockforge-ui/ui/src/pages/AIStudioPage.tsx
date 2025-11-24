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
  GitCompare,
  FileText,
  RefreshCw,
  Play,
  CheckCircle2,
  XCircle,
  Filter,
  Plus,
} from 'lucide-react';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { Textarea } from '../components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { apiService, contractDiffApi, type CapturedRequest, type ContractDiffResult, type AnalyzeRequestPayload } from '../services/api';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { AIStudioNav } from '../components/ai/AIStudioNav';
import { Link } from 'react-router-dom';

type TabType = 'chat' | 'generate' | 'debug' | 'personas' | 'budget' | 'contract-diff';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  intent?: string;
  data?: any;
}

interface FeatureUsage {
  tokens_used: number;
  cost_usd: number;
  calls_made: number;
}

interface UsageStats {
  tokens_used: number;
  cost_usd: number;
  calls_made: number;
  budget_limit: number;
  usage_percentage: number;
  feature_breakdown?: Record<string, FeatureUsage>;
}

export function AIStudioPage() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<TabType>('chat');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [usageStats, setUsageStats] = useState<UsageStats | null>(null);
  const [loadingStats, setLoadingStats] = useState(true);

  // Contract Diff state
  const [selectedCapture, setSelectedCapture] = useState<string | null>(null);
  const [analysisResult, setAnalysisResult] = useState<ContractDiffResult | null>(null);
  const [specPath, setSpecPath] = useState('');
  const [specContent, setSpecContent] = useState('');
  const [filterSource, setFilterSource] = useState<string>('all');
  const [filterMethod, setFilterMethod] = useState<string>('all');
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  // Load usage stats on mount
  useEffect(() => {
    loadUsageStats();
  }, []);

  // Fetch captured requests for contract diff
  const { data: capturesData, isLoading: capturesLoading, refetch: refetchCaptures } = useQuery({
    queryKey: ['contract-diff-captures', filterSource, filterMethod],
    queryFn: async () => {
      const params: any = {};
      if (filterSource !== 'all') params.source = filterSource;
      if (filterMethod !== 'all') params.method = filterMethod;
      return contractDiffApi.getCapturedRequests(params);
    },
    enabled: activeTab === 'contract-diff',
  });

  // Fetch statistics for contract diff
  const { data: statsData } = useQuery({
    queryKey: ['contract-diff-statistics'],
    queryFn: () => contractDiffApi.getStatistics(),
    refetchInterval: activeTab === 'contract-diff' ? 5000 : false,
    enabled: activeTab === 'contract-diff',
  });

  // Analyze mutation for contract diff
  const analyzeMutation = useMutation({
    mutationFn: async ({ captureId, payload }: { captureId: string; payload: AnalyzeRequestPayload }) => {
      return contractDiffApi.analyzeCapturedRequest(captureId, payload);
    },
    onSuccess: (data) => {
      setAnalysisResult(data.result);
      queryClient.invalidateQueries({ queryKey: ['contract-diff-captures'] });
      queryClient.invalidateQueries({ queryKey: ['contract-diff-statistics'] });
      loadUsageStats(); // Refresh usage stats after analysis
    },
    onError: (error: Error) => {
      logger.error('Analysis failed', error);
      toast.error(`Analysis failed: ${error.message}`);
    },
  });

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
            feature_breakdown: result.data.feature_breakdown || {},
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
      {/* Navigation */}
      <AIStudioNav showQuickActions={activeTab === 'chat'} />

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
            { id: 'contract-diff' as TabType, label: 'Contract Diff', icon: GitCompare },
            { id: 'budget' as TabType, label: 'Budget', icon: DollarSign },
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`
                py-4 px-1 border-b-2 font-medium text-sm flex items-center gap-2
                ${activeTab === tab.id
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
                        className={`max-w-[80%] rounded-lg p-3 ${msg.role === 'user'
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
                              <div className="text-xs text-muted-foreground mb-2">
                                Config: <code className="bg-white px-1 rounded">{suggestion.config_path}</code>
                              </div>
                            )}
                            {suggestion.patch && (
                              <div className="mt-3 pt-3 border-t border-blue-300">
                                <div className="flex items-center justify-between mb-2">
                                  <span className="text-xs font-medium text-blue-900">JSON Patch Available</span>
                                  <div className="flex gap-2">
                                    <Button
                                      size="sm"
                                      variant="outline"
                                      onClick={async () => {
                                        try {
                                          // Preview patch
                                          const patchStr = JSON.stringify(suggestion.patch, null, 2);
                                          const preview = window.open('', '_blank');
                                          if (preview) {
                                            preview.document.write(`
                                              <html>
                                                <head><title>Patch Preview</title></head>
                                                <body style="font-family: monospace; padding: 20px;">
                                                  <h2>JSON Patch Preview</h2>
                                                  <pre style="background: #f5f5f5; padding: 15px; border-radius: 5px;">${patchStr}</pre>
                                                </body>
                                              </html>
                                            `);
                                          }
                                        } catch (err) {
                                          logger.error('Failed to preview patch', err);
                                          toast.error('Failed to preview patch');
                                        }
                                      }}
                                    >
                                      Preview
                                    </Button>
                                    <Button
                                      size="sm"
                                      variant="outline"
                                      onClick={async () => {
                                        try {
                                          // Apply patch via API
                                          const response = await fetch('/__mockforge/ai-studio/apply-patch', {
                                            method: 'POST',
                                            headers: {
                                              'Content-Type': 'application/json',
                                            },
                                            body: JSON.stringify({
                                              patch: suggestion.patch,
                                              config_path: suggestion.config_path,
                                            }),
                                          });
                                          if (response.ok) {
                                            const result = await response.json();
                                            if (result.success) {
                                              toast.success('Patch applied successfully!', {
                                                description: `Updated ${suggestion.config_path}`,
                                              });
                                            } else {
                                              toast.error(result.error || 'Failed to apply patch');
                                            }
                                          } else {
                                            toast.error('Failed to apply patch');
                                          }
                                        } catch (err) {
                                          logger.error('Failed to apply patch', err);
                                          toast.error('Failed to apply patch');
                                        }
                                      }}
                                    >
                                      Apply
                                    </Button>
                                  </div>
                                </div>
                                <div className="text-xs bg-white p-2 rounded border border-blue-200">
                                  <div className="mb-1">
                                    <span className="font-medium">Operation:</span> {suggestion.patch.op}
                                  </div>
                                  <div className="mb-1">
                                    <span className="font-medium">Path:</span> <code>{suggestion.patch.path}</code>
                                  </div>
                                  {suggestion.patch.value && (
                                    <div>
                                      <span className="font-medium">Value:</span>
                                      <pre className="mt-1 text-xs overflow-x-auto">
                                        {JSON.stringify(suggestion.patch.value, null, 2).substring(0, 200)}
                                        {JSON.stringify(suggestion.patch.value, null, 2).length > 200 ? '...' : ''}
                                      </pre>
                                    </div>
                                  )}
                                </div>
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
            .map((msg, idx) => {
              const isFrozen = msg.content?.includes('frozen') || msg.content?.includes('Frozen') || msg.data?.persona?._frozen_metadata;
              const isAiGenerated = true; // All personas from this tab are AI-generated

              return (
                <Card key={idx} className="p-6">
                  <div className="space-y-4">
                    <div>
                      <div className="flex items-center gap-2 mb-2">
                        <h3 className="text-lg font-semibold">Generated Persona</h3>
                        {isAiGenerated && (
                          <span className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium bg-purple-100 text-purple-700 rounded border border-purple-300" title="AI-generated persona">
                            <Sparkles className="h-3 w-3" />
                            AI
                          </span>
                        )}
                        {isFrozen && (
                          <span className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium bg-blue-50 text-blue-700 rounded border border-blue-300" title="Frozen artifact (deterministic mode)">
                            <Snowflake className="h-3 w-3" />
                            Frozen
                          </span>
                        )}
                      </div>
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
              );
            })}
        </div>
      )}

      {activeTab === 'contract-diff' && (
        <div className="space-y-6">
          {/* Statistics Cards */}
          {statsData?.statistics && (
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <Card className="p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-muted-foreground">Total Captures</p>
                    <p className="text-2xl font-bold">{statsData.statistics.total_captures}</p>
                  </div>
                  <FileText className="w-8 h-8 text-blue-500" />
                </div>
              </Card>
              <Card className="p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-muted-foreground">Analyzed</p>
                    <p className="text-2xl font-bold">{statsData.statistics.analyzed_captures}</p>
                  </div>
                  <CheckCircle2 className="w-8 h-8 text-green-500" />
                </div>
              </Card>
              <Card className="p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-muted-foreground">Sources</p>
                    <p className="text-2xl font-bold">{Object.keys(statsData.statistics.sources).length}</p>
                  </div>
                  <TrendingUp className="w-8 h-8 text-purple-500" />
                </div>
              </Card>
              <Card className="p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm text-muted-foreground">Methods</p>
                    <p className="text-2xl font-bold">{Object.keys(statsData.statistics.methods).length}</p>
                  </div>
                  <Filter className="w-8 h-8 text-orange-500" />
                </div>
              </Card>
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Left Column: Captured Requests */}
            <Card className="p-6">
              <div className="space-y-4">
                <h3 className="text-lg font-semibold">Captured Requests</h3>

                {/* Filters */}
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label>Source</Label>
                    <Select value={filterSource} onValueChange={setFilterSource}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Sources</SelectItem>
                        {Array.from(new Set((capturesData?.captures || []).map(c => c.source))).filter(Boolean).map(source => (
                          <SelectItem key={source} value={source}>{source}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label>Method</Label>
                    <Select value={filterMethod} onValueChange={setFilterMethod}>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Methods</SelectItem>
                        {Array.from(new Set((capturesData?.captures || []).map(c => c.method))).filter(Boolean).map(method => (
                          <SelectItem key={method} value={method}>{method}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                {/* Request List */}
                <div className="border border-gray-200 rounded-lg divide-y divide-gray-200 max-h-96 overflow-y-auto">
                  {capturesLoading ? (
                    <div className="p-4 text-center text-muted-foreground">Loading...</div>
                  ) : (capturesData?.captures || []).length === 0 ? (
                    <div className="p-4 text-center text-muted-foreground">No captured requests</div>
                  ) : (
                    (capturesData?.captures || []).map(capture => (
                      <div
                        key={capture.id}
                        onClick={() => setSelectedCapture(capture.id || null)}
                        className={`p-4 cursor-pointer hover:bg-gray-50 transition-colors ${selectedCapture === capture.id ? 'bg-blue-50 border-l-4 border-blue-500' : ''
                          }`}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex-1">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="px-2 py-1 bg-gray-100 text-gray-700 rounded text-xs font-medium">
                                {capture.method}
                              </span>
                              <span className="text-sm font-mono text-gray-900">{capture.path}</span>
                            </div>
                            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                              <span>{capture.source}</span>
                              {capture.analyzed && (
                                <span className="px-2 py-0.5 bg-green-100 text-green-700 rounded text-xs">Analyzed</span>
                              )}
                            </div>
                          </div>
                        </div>
                      </div>
                    ))
                  )}
                </div>

                <Button onClick={() => refetchCaptures()} variant="outline" className="w-full">
                  <RefreshCw className="w-4 h-4 mr-2" />
                  Refresh
                </Button>
              </div>
            </Card>

            {/* Right Column: Analysis Configuration */}
            <Card className="p-6">
              <div className="space-y-4">
                <h3 className="text-lg font-semibold">Analysis Configuration</h3>
                <div>
                  <Label>Contract Spec Path</Label>
                  <Input
                    placeholder="/path/to/openapi.yaml"
                    value={specPath}
                    onChange={(e) => setSpecPath(e.target.value)}
                  />
                </div>
                <div>
                  <Label>Or Contract Spec Content (YAML/JSON)</Label>
                  <Textarea
                    placeholder="Paste OpenAPI spec content here..."
                    value={specContent}
                    onChange={(e) => setSpecContent(e.target.value)}
                    rows={8}
                    className="font-mono text-xs"
                  />
                </div>
                <Button
                  onClick={async () => {
                    if (!selectedCapture) {
                      toast.error('Please select a captured request');
                      return;
                    }

                    if (!specPath && !specContent) {
                      toast.error('Please provide either a spec path or spec content');
                      return;
                    }

                    setIsAnalyzing(true);
                    try {
                      const payload: AnalyzeRequestPayload = {
                        spec_path: specPath || undefined,
                        spec_content: specContent || undefined,
                        config: {
                          llm_provider: 'openai',
                          confidence_threshold: 0.5,
                        },
                      };

                      await analyzeMutation.mutateAsync({ captureId: selectedCapture, payload });
                      toast.success('Analysis completed successfully!');
                    } catch (error) {
                      // Error handled by mutation
                    } finally {
                      setIsAnalyzing(false);
                    }
                  }}
                  disabled={!selectedCapture || isAnalyzing || (!specPath && !specContent)}
                  className="w-full"
                >
                  <Play className="w-4 h-4 mr-2" />
                  {isAnalyzing ? 'Analyzing...' : 'Analyze Request'}
                </Button>
              </div>
            </Card>
          </div>

          {/* Analysis Results */}
          {analysisResult && (
            <div className="space-y-6">
              <Card className="p-6">
                <div className="space-y-4">
                  {/* Overall Status */}
                  <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                    <div className="flex items-center gap-4">
                      {analysisResult.matches ? (
                        <CheckCircle2 className="w-6 h-6 text-green-500" />
                      ) : (
                        <XCircle className="w-6 h-6 text-red-500" />
                      )}
                      <div>
                        <p className="font-semibold text-gray-900">
                          {analysisResult.matches ? 'Contract Matches' : 'Contract Mismatches Detected'}
                        </p>
                        <p className="text-sm text-muted-foreground">
                          {analysisResult.mismatches.length} mismatch(es) found
                        </p>
                      </div>
                    </div>
                    <div className={`inline-flex items-center px-2 py-1 rounded-full ${analysisResult.confidence >= 0.8 ? 'bg-green-100' : analysisResult.confidence >= 0.5 ? 'bg-yellow-100' : 'bg-red-100'
                      }`}>
                      <span className={`text-sm font-medium ${analysisResult.confidence >= 0.8 ? 'text-green-600' : analysisResult.confidence >= 0.5 ? 'text-yellow-600' : 'text-red-600'
                        }`}>
                        {Math.round(analysisResult.confidence * 100)}%
                      </span>
                    </div>
                  </div>

                  {/* Mismatches Table */}
                  {analysisResult.mismatches.length > 0 && (
                    <div>
                      <h3 className="text-lg font-semibold mb-3">Mismatches</h3>
                      <div className="overflow-x-auto">
                        <table className="w-full border-collapse">
                          <thead>
                            <tr className="border-b border-gray-200">
                              <th className="text-left p-3 font-semibold text-sm text-gray-700">Path</th>
                              <th className="text-left p-3 font-semibold text-sm text-gray-700">Type</th>
                              <th className="text-left p-3 font-semibold text-sm text-gray-700">Severity</th>
                              <th className="text-left p-3 font-semibold text-sm text-gray-700">Description</th>
                            </tr>
                          </thead>
                          <tbody>
                            {analysisResult.mismatches.map((mismatch, idx) => (
                              <tr key={idx} className="border-b border-gray-100 hover:bg-gray-50">
                                <td className="p-3 text-sm font-mono text-gray-900">{mismatch.path}</td>
                                <td className="p-3 text-sm text-gray-600">{mismatch.mismatch_type}</td>
                                <td className="p-3">
                                  <span className={`px-2 py-1 rounded text-xs font-medium ${mismatch.severity === 'critical' ? 'bg-red-100 text-red-800' :
                                      mismatch.severity === 'high' ? 'bg-orange-100 text-orange-800' :
                                        mismatch.severity === 'medium' ? 'bg-yellow-100 text-yellow-800' :
                                          'bg-blue-100 text-blue-800'
                                    }`}>
                                    {mismatch.severity.toUpperCase()}
                                  </span>
                                </td>
                                <td className="p-3 text-sm text-gray-700">{mismatch.description}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}

                  {/* Recommendations */}
                  {analysisResult.recommendations.length > 0 && (
                    <div>
                      <h3 className="text-lg font-semibold mb-3">AI Recommendations</h3>
                      <div className="space-y-3">
                        {analysisResult.recommendations.map((rec, idx) => (
                          <div key={idx} className="border border-gray-200 rounded-lg p-4 bg-white">
                            <div className="flex items-start justify-between mb-2">
                              <div className="flex-1">
                                <p className="text-sm text-gray-900">{rec.recommendation}</p>
                                {rec.suggested_fix && (
                                  <div className="mt-2 p-2 bg-blue-50 border border-blue-200 rounded">
                                    <p className="text-xs font-semibold text-blue-900 mb-1">Suggested Fix:</p>
                                    <p className="text-xs text-blue-800">{rec.suggested_fix}</p>
                                  </div>
                                )}
                              </div>
                              <div className={`inline-flex items-center px-2 py-1 rounded-full ${rec.confidence >= 0.8 ? 'bg-green-100' : rec.confidence >= 0.5 ? 'bg-yellow-100' : 'bg-red-100'
                                }`}>
                                <span className={`text-sm font-medium ${rec.confidence >= 0.8 ? 'text-green-600' : rec.confidence >= 0.5 ? 'text-yellow-600' : 'text-red-600'
                                  }`}>
                                  {Math.round(rec.confidence * 100)}%
                                </span>
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Correction Proposals */}
                  {analysisResult.corrections.length > 0 && (
                    <div>
                      <h3 className="text-lg font-semibold mb-3">Correction Proposals</h3>
                      <div className="space-y-3">
                        {analysisResult.corrections.map((correction, idx) => (
                          <div key={idx} className="border border-gray-200 rounded-lg p-4 bg-white">
                            <div className="flex items-start justify-between mb-2">
                              <div className="flex-1">
                                <p className="text-sm font-semibold text-gray-900 mb-1">{correction.description}</p>
                                <p className="text-xs text-gray-600 font-mono mb-2">Path: {correction.path}</p>
                                <div className="flex items-center gap-2">
                                  <span className="px-2 py-1 bg-gray-100 text-gray-700 rounded text-xs">
                                    {correction.operation}
                                  </span>
                                  {correction.value && (
                                    <div className="text-xs text-gray-600">
                                      Value: <code className="bg-gray-100 px-1 rounded">{JSON.stringify(correction.value)}</code>
                                    </div>
                                  )}
                                </div>
                              </div>
                              <div className={`inline-flex items-center px-2 py-1 rounded-full ${correction.confidence >= 0.8 ? 'bg-green-100' : correction.confidence >= 0.5 ? 'bg-yellow-100' : 'bg-red-100'
                                }`}>
                                <span className={`text-sm font-medium ${correction.confidence >= 0.8 ? 'text-green-600' : correction.confidence >= 0.5 ? 'text-yellow-600' : 'text-red-600'
                                  }`}>
                                  {Math.round(correction.confidence * 100)}%
                                </span>
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                      <div className="mt-4">
                        <Button
                          variant="outline"
                          onClick={async () => {
                            if (!selectedCapture) return;
                            try {
                              const payload: AnalyzeRequestPayload = {
                                spec_path: specPath || undefined,
                                spec_content: specContent || undefined,
                                config: {
                                  llm_provider: 'openai',
                                  confidence_threshold: 0.5,
                                },
                              };
                              const result = await contractDiffApi.generatePatchFile(selectedCapture, payload);
                              const blob = new Blob([JSON.stringify(result.patch_file, null, 2)], { type: 'application/json' });
                              const url = URL.createObjectURL(blob);
                              const a = document.createElement('a');
                              a.href = url;
                              a.download = `contract-patch-${selectedCapture}.json`;
                              document.body.appendChild(a);
                              a.click();
                              document.body.removeChild(a);
                              URL.revokeObjectURL(url);
                              toast.success('Patch file downloaded successfully!');
                            } catch (error: any) {
                              toast.error(`Failed to generate patch: ${error.message}`);
                            }
                          }}
                        >
                          <Download className="w-4 h-4 mr-2" />
                          Download Patch File
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </Card>
            </div>
          )}
        </div>
      )}

      {activeTab === 'budget' && (
        <Card className="p-6">
          <div className="space-y-4">
            <h2 className="text-xl font-semibold">Budget & Usage</h2>
            {loadingStats ? (
              <p className="text-muted-foreground">Loading usage statistics...</p>
            ) : usageStats ? (
              <div className="space-y-6">
                {/* Overall Stats */}
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

                {/* Budget Progress */}
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

                {/* Feature Breakdown */}
                {usageStats.feature_breakdown && Object.keys(usageStats.feature_breakdown).length > 0 && (
                  <div>
                    <h3 className="text-lg font-semibold mb-4">Usage by Feature</h3>
                    <div className="space-y-3">
                      {Object.entries(usageStats.feature_breakdown).map(([feature, stats]) => {
                        const featureName = feature.replace('AiFeature::', '').replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase());
                        const percentage = usageStats.tokens_used > 0
                          ? (stats.tokens_used / usageStats.tokens_used) * 100
                          : 0;
                        return (
                          <div key={feature} className="p-4 border border-gray-200 rounded-lg">
                            <div className="flex items-center justify-between mb-2">
                              <div>
                                <h4 className="font-medium">{featureName}</h4>
                                <p className="text-xs text-muted-foreground">
                                  {percentage.toFixed(1)}% of total usage
                                </p>
                              </div>
                              <div className="text-right">
                                <p className="text-sm font-medium">{stats.calls_made} calls</p>
                                <p className="text-xs text-muted-foreground">
                                  ${stats.cost_usd.toFixed(4)}
                                </p>
                              </div>
                            </div>
                            <div className="space-y-1">
                              <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground">Tokens</span>
                                <span className="font-medium">{stats.tokens_used.toLocaleString()}</span>
                              </div>
                              <div className="w-full bg-gray-200 rounded-full h-2">
                                <div
                                  className="bg-blue-500 h-2 rounded-full transition-all"
                                  style={{ width: `${percentage}%` }}
                                />
                              </div>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
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
