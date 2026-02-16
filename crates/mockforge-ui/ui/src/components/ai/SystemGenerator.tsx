//! System Generator Component
//!
//! Provides UI for generating complete backend systems from natural language descriptions.
//! Supports draft artifacts with review + apply/freeze workflow, versioning (v1, v2, never mutates),
//! and integration with deterministic mode settings.

import React, { useState } from 'react';
import {
  Sparkles,
  FileText,
  Users,
  GitBranch,
  Webhook,
  AlertTriangle,
  Code2,
  Download,
  CheckCircle2,
  Loader2,
  RefreshCw,
  Play,
  Snowflake,
  Eye,
  EyeOff,
} from 'lucide-react';
import { Card } from '../ui/Card';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';

interface GeneratedSystem {
  system_id: string;
  version: string;
  artifacts: Record<string, SystemArtifact>;
  workspace_id?: string;
  status: 'draft' | 'frozen';
  tokens_used?: number;
  cost_usd?: number;
  metadata: SystemMetadata;
}

interface SystemArtifact {
  artifact_type: string;
  content: any;
  format: string;
  artifact_id: string;
}

interface SystemMetadata {
  description: string;
  entities: string[];
  relationships: string[];
  operations: string[];
  generated_at: string;
}

interface AppliedSystem {
  system_id: string;
  version: string;
  applied_artifacts: string[];
  frozen: boolean;
}

interface SystemGeneratorProps {
  onUsageUpdate?: () => void;
}

export function SystemGenerator({ onUsageUpdate }: SystemGeneratorProps) {
  const [description, setDescription] = useState<string>('');
  const [outputFormats, setOutputFormats] = useState<string[]>(['openapi', 'personas', 'lifecycles']);
  const [isGenerating, setIsGenerating] = useState(false);
  const [generatedSystem, setGeneratedSystem] = useState<GeneratedSystem | null>(null);
  const [selectedArtifact, setSelectedArtifact] = useState<string | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [isFreezing, setIsFreezing] = useState(false);

  const availableFormats = [
    { id: 'openapi', label: 'OpenAPI', icon: FileText },
    { id: 'graphql', label: 'GraphQL', icon: Code2 },
    { id: 'personas', label: 'Personas', icon: Users },
    { id: 'lifecycles', label: 'Lifecycles', icon: GitBranch },
    { id: 'websocket', label: 'WebSocket Topics', icon: Webhook },
    { id: 'chaos', label: 'Chaos Profiles', icon: AlertTriangle },
    { id: 'ci', label: 'CI/CD Templates', icon: RefreshCw },
    { id: 'typings', label: 'TypeScript Typings', icon: Code2 },
  ];

  const handleGenerate = async () => {
    if (!description.trim()) {
      toast.error('Please provide a system description');
      return;
    }

    try {
      setIsGenerating(true);

      const response = await fetch('/__mockforge/api/v1/ai-studio/generate-system', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          description: description.trim(),
          output_formats: outputFormats,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();
      setGeneratedSystem(result.system);
      setSelectedArtifact(null);

      toast.success(`System generated successfully! Version: ${result.system.version}`);

      if (onUsageUpdate) {
        onUsageUpdate();
      }
    } catch (error: any) {
      logger.error('System generation failed', error);
      toast.error(`Generation failed: ${error.message}`);
    } finally {
      setIsGenerating(false);
    }
  };

  const handleApply = async () => {
    if (!generatedSystem) return;

    try {
      setIsApplying(true);

      const response = await fetch(
        `/__mockforge/api/v1/ai-studio/system/${generatedSystem.system_id}/apply`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            artifact_ids: null, // Apply all
          }),
        }
      );

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();

      // Update system status
      setGeneratedSystem({
        ...generatedSystem,
        status: result.applied.frozen ? 'frozen' : 'draft',
      });

      toast.success(
        `System design applied! ${result.applied.frozen ? 'Artifacts frozen.' : 'Artifacts ready for use.'}`
      );
    } catch (error: any) {
      logger.error('Apply system failed', error);
      toast.error(`Apply failed: ${error.message}`);
    } finally {
      setIsApplying(false);
    }
  };

  const handleFreeze = async (artifactIds?: string[]) => {
    if (!generatedSystem) return;

    try {
      setIsFreezing(true);

      const response = await fetch(
        `/__mockforge/api/v1/ai-studio/system/${generatedSystem.system_id}/freeze`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            artifact_ids: artifactIds || Object.values(generatedSystem.artifacts).map(a => a.artifact_id),
          }),
        }
      );

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();

      toast.success(`Artifacts frozen successfully!`);
    } catch (error: any) {
      logger.error('Freeze artifacts failed', error);
      toast.error(`Freeze failed: ${error.message}`);
    } finally {
      setIsFreezing(false);
    }
  };

  const handleDownload = (artifactType: string, artifact: SystemArtifact) => {
    const contentStr = artifact.format === 'json'
      ? JSON.stringify(artifact.content, null, 2)
      : artifact.content;

    const blob = new Blob([contentStr], {
      type: artifact.format === 'json' ? 'application/json' : 'text/plain'
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${artifactType}_${generatedSystem?.version || 'v1'}.${artifact.format}`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const formatArtifactContent = (artifact: SystemArtifact): string => {
    if (artifact.format === 'json') {
      return JSON.stringify(artifact.content, null, 2);
    }
    return String(artifact.content);
  };

  return (
    <div className="space-y-6">
      {/* Input Section */}
      <Card className="p-6">
        <div className="space-y-4">
          <div>
            <Label htmlFor="description">System Description</Label>
            <Textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="I'm building a ride-sharing app with drivers, riders, trips, payments, live-location updates, pricing, and surge events."
              rows={6}
              className="mt-2"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Describe your backend system in natural language. Be specific about entities, relationships, and features.
            </p>
          </div>

          <div>
            <Label>Output Formats</Label>
            <div className="mt-2 grid grid-cols-2 md:grid-cols-4 gap-2">
              {availableFormats.map((format) => {
                const Icon = format.icon;
                const isSelected = outputFormats.includes(format.id);
                return (
                  <Button
                    key={format.id}
                    variant={isSelected ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => {
                      setOutputFormats((prev) =>
                        isSelected
                          ? prev.filter((f) => f !== format.id)
                          : [...prev, format.id]
                      );
                    }}
                    className="justify-start"
                  >
                    <Icon className="w-4 h-4 mr-2" />
                    {format.label}
                  </Button>
                );
              })}
            </div>
          </div>

          <Button
            onClick={handleGenerate}
            disabled={isGenerating || !description.trim()}
            className="w-full"
            size="lg"
          >
            {isGenerating ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Generating System...
              </>
            ) : (
              <>
                <Sparkles className="w-4 h-4 mr-2" />
                Generate System
              </>
            )}
          </Button>
        </div>
      </Card>

      {/* Generated System Display */}
      {generatedSystem && (
        <div className="space-y-6">
          {/* System Summary Card */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <div>
                <h3 className="text-lg font-semibold">Generated System</h3>
                <div className="flex items-center space-x-2 mt-1">
                  <span className="text-sm text-gray-600 dark:text-gray-400">
                    {generatedSystem.system_id} • {generatedSystem.version}
                  </span>
                  {generatedSystem.status === 'frozen' ? (
                    <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                      <Snowflake className="w-3 h-3 mr-1" />
                      Frozen
                    </span>
                  ) : (
                    <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200">
                      Draft
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-center space-x-2">
                {generatedSystem.tokens_used && (
                  <div className="text-right">
                    <div className="text-xs text-gray-600 dark:text-gray-400">Tokens</div>
                    <div className="text-sm font-semibold">{generatedSystem.tokens_used.toLocaleString()}</div>
                  </div>
                )}
                {generatedSystem.cost_usd && (
                  <div className="text-right">
                    <div className="text-xs text-gray-600 dark:text-gray-400">Cost</div>
                    <div className="text-sm font-semibold">${generatedSystem.cost_usd.toFixed(4)}</div>
                  </div>
                )}
              </div>
            </div>

            {/* Metadata */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
                <div>
                  <div className="text-gray-600 dark:text-gray-400 mb-1">Entities</div>
                  <div className="flex flex-wrap gap-1">
                    {generatedSystem.metadata.entities.map((entity, idx) => (
                      <span
                        key={idx}
                        className="px-2 py-1 bg-gray-100 dark:bg-gray-800 rounded text-xs"
                      >
                        {entity}
                      </span>
                    ))}
                  </div>
                </div>
                <div>
                  <div className="text-gray-600 dark:text-gray-400 mb-1">Relationships</div>
                  <div className="text-xs text-gray-700 dark:text-gray-300">
                    {generatedSystem.metadata.relationships.length} relationships
                  </div>
                </div>
                <div>
                  <div className="text-gray-600 dark:text-gray-400 mb-1">Generated</div>
                  <div className="text-xs text-gray-700 dark:text-gray-300">
                    {new Date(generatedSystem.metadata.generated_at).toLocaleString()}
                  </div>
                </div>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700 mt-4 flex items-center space-x-2">
              <Button
                onClick={handleApply}
                disabled={isApplying || generatedSystem.status === 'frozen'}
                variant="default"
              >
                {isApplying ? (
                  <>
                    <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    Applying...
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="w-4 h-4 mr-2" />
                    Apply System Design
                  </>
                )}
              </Button>
              {generatedSystem.status === 'draft' && (
                <Button
                  onClick={() => handleFreeze()}
                  disabled={isFreezing}
                  variant="outline"
                >
                  {isFreezing ? (
                    <>
                      <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                      Freezing...
                    </>
                  ) : (
                    <>
                      <Snowflake className="w-4 h-4 mr-2" />
                      Freeze All Artifacts
                    </>
                  )}
                </Button>
              )}
            </div>
          </Card>

          {/* Artifacts Preview */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4">Generated Artifacts</h3>
            <div className="space-y-4">
              {Object.entries(generatedSystem.artifacts).map(([artifactType, artifact]) => (
                <div
                  key={artifactType}
                  className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center space-x-2">
                      <FileText className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                      <div>
                        <div className="font-medium capitalize">{artifactType}</div>
                        <div className="text-xs text-gray-600 dark:text-gray-400">
                          {artifact.format.toUpperCase()} • {artifact.artifact_id.substring(0, 8)}...
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setSelectedArtifact(
                            selectedArtifact === artifactType ? null : artifactType
                          );
                        }}
                      >
                        {selectedArtifact === artifactType ? (
                          <EyeOff className="w-4 h-4" />
                        ) : (
                          <Eye className="w-4 h-4" />
                        )}
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDownload(artifactType, artifact)}
                      >
                        <Download className="w-4 h-4" />
                      </Button>
                      {generatedSystem.status === 'draft' && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleFreeze([artifact.artifact_id])}
                          disabled={isFreezing}
                        >
                          <Snowflake className="w-4 h-4" />
                        </Button>
                      )}
                    </div>
                  </div>

                  {selectedArtifact === artifactType && (
                    <div className="mt-4 p-4 bg-gray-50 dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700">
                      <pre className="text-xs overflow-x-auto max-h-96">
                        {formatArtifactContent(artifact)}
                      </pre>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
