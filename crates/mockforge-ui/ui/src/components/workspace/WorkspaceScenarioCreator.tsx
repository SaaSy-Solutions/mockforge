//! Workspace Scenario Creator Component
//!
//! Allows users to create complete workspace scenarios from natural language descriptions.

import React, { useState } from 'react';
import { Loader2, CheckCircle2, XCircle, Download, Copy, Building2 } from 'lucide-react';
import { Button } from '../ui/button';
import { Card } from '../ui/Card';
import { cn } from '../../utils/cn';
import { logger } from '@/utils/logger';
import { apiErrorMessage } from '@/utils/errorHandling';
import { aiStudioApi } from '../../services/api/aiStudio';
import { isCloudMode } from '../../utils/cloudMode';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';

interface WorkspaceScenarioCreatorProps {
  onScenarioCreated?: (scenario: WorkspaceScenarioResult) => void;
  className?: string;
}

export interface WorkspaceScenarioResult {
  description: string;
  scenario?: {
    workspace_id: string;
    name: string;
    description: string;
    openapi_spec?: any;
    chaos_config?: string;
    fixtures: Record<string, any[]>;
    config_summary: {
      endpoint_count: number;
      model_count: number;
      chaos_characteristic_count: number;
      initial_data_counts: Record<string, number>;
    };
  };
  error?: string;
}

export function WorkspaceScenarioCreator({
  onScenarioCreated,
  className,
}: WorkspaceScenarioCreatorProps) {
  const [description, setDescription] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [result, setResult] = useState<WorkspaceScenarioResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'openapi' | 'chaos' | 'fixtures'>(
    'overview'
  );
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);

  const processDescription = async () => {
    if (!description.trim() || isProcessing) return;

    setIsProcessing(true);
    setError(null);
    setResult(null);

    try {
      let data: any;
      if (isCloudMode()) {
        // Cloud handler returns { scenario, content, ...meta }; the
        // scenario shape is the LLM's best-effort {name, description,
        // steps[]} JSON, which the renderer below tolerates as long as
        // we provide a workspace_id alongside.
        const cloudResp = await aiStudioApi.voiceCreateWorkspaceScenario({
          description,
          workspace_context: activeWorkspace
            ? `${activeWorkspace.name} (${activeWorkspace.id})`
            : undefined,
        });
        const cloudScenario =
          cloudResp.scenario && typeof cloudResp.scenario === 'object'
            ? (cloudResp.scenario as Record<string, unknown>)
            : null;
        data = {
          scenario: cloudScenario
            ? {
                workspace_id: activeWorkspace?.id ?? '',
                name: (cloudScenario.name as string) ?? 'Generated scenario',
                description: (cloudScenario.description as string) ?? description,
                fixtures: {},
                config_summary: {
                  endpoint_count: 0,
                  model_count: 0,
                  chaos_characteristic_count: 0,
                  initial_data_counts: {},
                },
              }
            : undefined,
          error: cloudScenario ? undefined : 'Model output was not parseable JSON',
        };
      } else {
        const response = await fetch('/api/v2/voice/create-workspace-scenario', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ description }),
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
          throw new Error(apiErrorMessage(response, errorData, `HTTP ${response.status}`));
        }

        const responseData = await response.json();

        // Handle ApiResponse wrapper
        data = responseData.data || responseData;
      }

      const scenarioResult: WorkspaceScenarioResult = {
        description,
        scenario: data.scenario || undefined,
        error: data.error || undefined,
      };

      setResult(scenarioResult);
      onScenarioCreated?.(scenarioResult);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create workspace scenario';
      setError(errorMessage);
      setResult({
        description,
        error: errorMessage,
      });
    } finally {
      setIsProcessing(false);
    }
  };

  const handleTextSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    processDescription();
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(
      () => {
        logger.debug('Copied to clipboard');
      },
      (err) => {
        console.error('Failed to copy:', err);
      }
    );
  };

  const downloadFile = (content: string, filename: string, contentType: string) => {
    const blob = new Blob([content], { type: contentType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <div className={cn('flex flex-col gap-4', className)}>
      <div className="space-y-2">
        <label htmlFor="scenario-description" className="block text-sm font-medium text-foreground">
          Describe your workspace scenario
        </label>
        <textarea
          id="scenario-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder='e.g., Create a workspace that simulates a bank with flaky foreign exchange rates and slow KYC, with 3 existing users and 5 open disputes'
          className="w-full min-h-[120px] px-3 py-2 border border-border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-info"
          disabled={isProcessing}
        />
        <p className="text-xs text-muted-foreground">
          Describe the domain, chaos characteristics, and initial data requirements
        </p>
      </div>

      <div className="flex gap-2">
        <Button
          onClick={processDescription}
          disabled={!description.trim() || isProcessing}
          className="flex items-center gap-2"
        >
          {isProcessing ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Creating Scenario...
            </>
          ) : (
            <>
              <Building2 className="w-4 h-4" />
              Create Workspace Scenario
            </>
          )}
        </Button>
      </div>

      {error && (
        <div className="p-4 bg-danger-50 border border-danger-200 rounded-md flex items-start gap-3">
          <XCircle className="w-5 h-5 text-danger-600 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h3 className="text-sm font-medium text-danger-700">Error</h3>
            <p className="text-sm text-danger-700 mt-1">{error}</p>
          </div>
        </div>
      )}

      {result && !result.error && result.scenario && (
        <div className="space-y-4">
          <div className="p-4 bg-success-50 border border-success-200 rounded-md flex items-start gap-3">
            <CheckCircle2 className="w-5 h-5 text-success-600 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h3 className="text-sm font-medium text-success-700">Workspace Scenario Created</h3>
              <p className="text-sm text-success-700 mt-1">
                Workspace ID: <code className="text-xs bg-card px-1 rounded">{result.scenario.workspace_id}</code>
              </p>
            </div>
          </div>

          {/* Tabs */}
          <div className="border-b border-border">
            <nav className="-mb-px flex space-x-4">
              {['overview', 'openapi', 'chaos', 'fixtures'].map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab as any)}
                  className={`
                    py-2 px-1 border-b-2 font-medium text-sm capitalize
                    ${
                      activeTab === tab
                        ? 'border-primary text-primary'
                        : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                    }
                  `}
                >
                  {tab}
                </button>
              ))}
            </nav>
          </div>

          {/* Tab Content */}
          {activeTab === 'overview' && (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4">{result.scenario.name}</h3>
              <p className="text-foreground mb-4">{result.scenario.description}</p>

              <div className="grid md:grid-cols-2 gap-4">
                <div className="p-4 bg-muted rounded-lg">
                  <div className="text-sm text-muted-foreground">Endpoints</div>
                  <div className="text-2xl font-bold">{result.scenario.config_summary.endpoint_count}</div>
                </div>
                <div className="p-4 bg-muted rounded-lg">
                  <div className="text-sm text-muted-foreground">Models</div>
                  <div className="text-2xl font-bold">{result.scenario.config_summary.model_count}</div>
                </div>
                <div className="p-4 bg-muted rounded-lg">
                  <div className="text-sm text-muted-foreground">Chaos Characteristics</div>
                  <div className="text-2xl font-bold">
                    {result.scenario.config_summary.chaos_characteristic_count}
                  </div>
                </div>
                <div className="p-4 bg-muted rounded-lg">
                  <div className="text-sm text-muted-foreground">Initial Data Entities</div>
                  <div className="text-2xl font-bold">
                    {Object.keys(result.scenario.config_summary.initial_data_counts).length}
                  </div>
                </div>
              </div>

              {Object.keys(result.scenario.config_summary.initial_data_counts).length > 0 && (
                <div className="mt-4">
                  <h4 className="font-medium mb-2">Initial Data Counts</h4>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(result.scenario.config_summary.initial_data_counts).map(
                      ([entity, count]) => (
                        <span
                          key={entity}
                          className="px-3 py-1 bg-info-100 text-info-700 rounded-full text-sm"
                        >
                          {entity}: {count}
                        </span>
                      )
                    )}
                  </div>
                </div>
              )}
            </Card>
          )}

          {activeTab === 'openapi' && result.scenario.openapi_spec && (
            <Card className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold">OpenAPI Specification</h3>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      copyToClipboard(JSON.stringify(result.scenario!.openapi_spec, null, 2))
                    }
                  >
                    <Copy className="w-4 h-4 mr-2" />
                    Copy
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      downloadFile(
                        JSON.stringify(result.scenario!.openapi_spec, null, 2),
                        'openapi.json',
                        'application/json'
                      )
                    }
                  >
                    <Download className="w-4 h-4 mr-2" />
                    Download
                  </Button>
                </div>
              </div>
              <pre className="p-4 bg-muted border border-border rounded-md overflow-x-auto text-sm">
                <code>{JSON.stringify(result.scenario.openapi_spec, null, 2)}</code>
              </pre>
            </Card>
          )}

          {activeTab === 'chaos' && result.scenario.chaos_config && (
            <Card className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold">Chaos Configuration</h3>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => copyToClipboard(result.scenario!.chaos_config!)}
                  >
                    <Copy className="w-4 h-4 mr-2" />
                    Copy
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      downloadFile(result.scenario!.chaos_config!, 'chaos-config.yaml', 'text/yaml')
                    }
                  >
                    <Download className="w-4 h-4 mr-2" />
                    Download
                  </Button>
                </div>
              </div>
              <pre className="p-4 bg-muted border border-border rounded-md overflow-x-auto text-sm">
                <code>{result.scenario.chaos_config}</code>
              </pre>
            </Card>
          )}

          {activeTab === 'fixtures' && Object.keys(result.scenario.fixtures).length > 0 && (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4">Initial Fixture Data</h3>
              <div className="space-y-4">
                {Object.entries(result.scenario.fixtures).map(([entity, data]) => (
                  <div key={entity} className="border border-border rounded-lg overflow-hidden">
                    <div className="px-4 py-2 bg-muted border-b border-border">
                      <h4 className="font-medium capitalize">{entity}</h4>
                      <p className="text-sm text-muted-foreground">{data.length} items</p>
                    </div>
                    <div className="p-4">
                      <pre className="text-sm overflow-x-auto">
                        <code>{JSON.stringify(data, null, 2)}</code>
                      </pre>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}
