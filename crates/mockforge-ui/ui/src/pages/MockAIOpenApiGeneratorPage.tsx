//! MockAI OpenAPI Generator Page
//!
//! UI for generating OpenAPI specifications from recorded traffic
//! with filters, preview, and confidence scores.

import { logger } from '@/utils/logger';
import React, { useState, useCallback } from 'react';
import {
  FileText,
  Database,
  Filter,
  Download,
  Loader2,
  CheckCircle2,
  AlertTriangle,
  Calendar,
  Code,
  BarChart3,
  TrendingUp,
} from 'lucide-react';
import { apiService } from '../services/api';
import {
  PageHeader,
  Section,
  Alert,
  Button,
  Card,
  Badge,
  EmptyState,
} from '../components/ui/DesignSystem';
import { toast } from 'sonner';

interface GenerationRequest {
  database_path?: string;
  since?: string;
  until?: string;
  path_pattern?: string;
  min_confidence?: number;
}

interface GenerationMetadata {
  requests_analyzed: number;
  paths_inferred: number;
  path_confidence: Record<string, { value: number; reason: string }>;
  generated_at: string;
  duration_ms: number;
}

interface GenerationResult {
  spec: unknown;
  metadata: GenerationMetadata;
}

export function MockAIOpenApiGeneratorPage() {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<GenerationResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [formData, setFormData] = useState<GenerationRequest>({
    database_path: '',
    since: '',
    until: '',
    path_pattern: '',
    min_confidence: 0.7,
  });
  const [showPreview, setShowPreview] = useState(false);

  const handleInputChange = useCallback(
    (field: keyof GenerationRequest, value: string | number | undefined) => {
      setFormData((prev) => ({
        ...prev,
        [field]: value,
      }));
    },
    []
  );

  const handleGenerate = useCallback(async () => {
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      // Build request, omitting empty fields
      const request: GenerationRequest = {
        min_confidence: formData.min_confidence || 0.7,
      };

      if (formData.database_path?.trim()) {
        request.database_path = formData.database_path.trim();
      }

      if (formData.since?.trim()) {
        request.since = formData.since.trim();
      }

      if (formData.until?.trim()) {
        request.until = formData.until.trim();
      }

      if (formData.path_pattern?.trim()) {
        request.path_pattern = formData.path_pattern.trim();
      }

      const response = await apiService.generateOpenApiFromTraffic(request);
      setResult(response);
      setShowPreview(true);

      // Track that OpenAPI has been generated
      localStorage.setItem('mockai-openapi-generated', 'true');
      localStorage.setItem('mockai-openapi-last-generated', new Date().toISOString());

      toast.success('OpenAPI specification generated successfully');
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : 'Failed to generate OpenAPI specification';
      setError(errorMessage);
      logger.error('OpenAPI generation failed', err);
      toast.error(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [formData]);

  const handleDownload = useCallback(() => {
    if (!result) return;

    try {
      const specJson = JSON.stringify(result.spec, null, 2);
      const blob = new Blob([specJson], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `openapi-generated-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success('OpenAPI specification downloaded');
    } catch (err) {
      logger.error('Download failed', err);
      toast.error('Failed to download specification');
    }
  }, [result]);

  const handleDownloadYaml = useCallback(async () => {
    if (!result) return;

    try {
      // Convert JSON to YAML (would need a YAML library in production)
      // For now, just download as JSON with .yaml extension
      const specJson = JSON.stringify(result.spec, null, 2);
      const blob = new Blob([specJson], { type: 'application/x-yaml' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `openapi-generated-${new Date().toISOString().split('T')[0]}.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success('OpenAPI specification downloaded as YAML');
    } catch (err) {
      logger.error('YAML download failed', err);
      toast.error('Failed to download specification');
    }
  }, [result]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Generate OpenAPI from Traffic"
        description="Analyze recorded HTTP traffic and generate OpenAPI 3.0 specifications using AI-powered pattern detection"
        icon={<FileText className="h-6 w-6" />}
      />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Form Section */}
        <div className="lg:col-span-1">
          <Card className="p-6">
            <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
              <Filter className="h-5 w-5" />
              Generation Filters
            </h2>

            <div className="space-y-4">
              {/* Database Path */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Database Path
                  <span className="text-gray-500 text-xs ml-1">(optional)</span>
                </label>
                <input
                  type="text"
                  value={formData.database_path || ''}
                  onChange={(e) => handleInputChange('database_path', e.target.value)}
                  placeholder="./recordings.db"
                  className="w-full px-3 py-2 border rounded-md text-sm"
                />
                <p className="text-xs text-gray-500 mt-1">
                  Defaults to ./recordings.db in current directory
                </p>
              </div>

              {/* Time Range */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Start Time
                  <span className="text-gray-500 text-xs ml-1">(optional)</span>
                </label>
                <input
                  type="datetime-local"
                  value={formData.since || ''}
                  onChange={(e) => {
                    // Convert to ISO 8601 format
                    if (e.target.value) {
                      const date = new Date(e.target.value);
                      handleInputChange('since', date.toISOString());
                    } else {
                      handleInputChange('since', undefined);
                    }
                  }}
                  className="w-full px-3 py-2 border rounded-md text-sm"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1">
                  End Time
                  <span className="text-gray-500 text-xs ml-1">(optional)</span>
                </label>
                <input
                  type="datetime-local"
                  value={formData.until || ''}
                  onChange={(e) => {
                    if (e.target.value) {
                      const date = new Date(e.target.value);
                      handleInputChange('until', date.toISOString());
                    } else {
                      handleInputChange('until', undefined);
                    }
                  }}
                  className="w-full px-3 py-2 border rounded-md text-sm"
                />
              </div>

              {/* Path Pattern */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Path Pattern
                  <span className="text-gray-500 text-xs ml-1">(optional)</span>
                </label>
                <input
                  type="text"
                  value={formData.path_pattern || ''}
                  onChange={(e) => handleInputChange('path_pattern', e.target.value)}
                  placeholder="/api/*"
                  className="w-full px-3 py-2 border rounded-md text-sm font-mono"
                />
                <p className="text-xs text-gray-500 mt-1">
                  Supports wildcards (e.g., /api/*)
                </p>
              </div>

              {/* Confidence Threshold */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Minimum Confidence: {((formData.min_confidence || 0.7) * 100).toFixed(0)}%
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={formData.min_confidence || 0.7}
                  onChange={(e) =>
                    handleInputChange('min_confidence', parseFloat(e.target.value))
                  }
                  className="w-full"
                />
                <div className="flex justify-between text-xs text-gray-500 mt-1">
                  <span>0%</span>
                  <span>50%</span>
                  <span>100%</span>
                </div>
              </div>

              {/* Generate Button */}
              <Button
                onClick={handleGenerate}
                disabled={loading}
                className="w-full"
                variant="primary"
              >
                {loading ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Generating...
                  </>
                ) : (
                  <>
                    <Code className="h-4 w-4 mr-2" />
                    Generate OpenAPI Spec
                  </>
                )}
              </Button>
            </div>
          </Card>
        </div>

        {/* Results Section */}
        <div className="lg:col-span-2 space-y-6">
          {/* Error Display */}
          {error && (
            <Alert variant="error" title="Generation Failed">
              {error}
            </Alert>
          )}

          {/* Statistics Card */}
          {result && (
            <Card className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold flex items-center gap-2">
                  <BarChart3 className="h-5 w-5" />
                  Generation Statistics
                </h2>
                <div className="flex gap-2">
                  <Button
                    onClick={handleDownload}
                    variant="outline"
                    size="sm"
                    disabled={!result}
                  >
                    <Download className="h-4 w-4 mr-2" />
                    JSON
                  </Button>
                  <Button
                    onClick={handleDownloadYaml}
                    variant="outline"
                    size="sm"
                    disabled={!result}
                  >
                    <Download className="h-4 w-4 mr-2" />
                    YAML
                  </Button>
                </div>
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
                <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                  <div className="text-sm text-gray-600 dark:text-gray-400">
                    Requests Analyzed
                  </div>
                  <div className="text-2xl font-bold mt-1">
                    {result.metadata.requests_analyzed}
                  </div>
                </div>
                <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                  <div className="text-sm text-gray-600 dark:text-gray-400">
                    Paths Inferred
                  </div>
                  <div className="text-2xl font-bold mt-1">
                    {result.metadata.paths_inferred}
                  </div>
                </div>
                <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                  <div className="text-sm text-gray-600 dark:text-gray-400">
                    Generation Time
                  </div>
                  <div className="text-2xl font-bold mt-1">
                    {result.metadata.duration_ms}ms
                  </div>
                </div>
                <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                  <div className="text-sm text-gray-600 dark:text-gray-400">
                    Generated At
                  </div>
                  <div className="text-sm font-medium mt-1">
                    {new Date(result.metadata.generated_at).toLocaleString()}
                  </div>
                </div>
              </div>

              {/* Confidence Scores */}
              {Object.keys(result.metadata.path_confidence).length > 0 && (
                <div>
                  <h3 className="text-md font-semibold mb-3 flex items-center gap-2">
                    <TrendingUp className="h-4 w-4" />
                    Path Confidence Scores
                  </h3>
                  <div className="space-y-2">
                    {Object.entries(result.metadata.path_confidence)
                      .filter(
                        ([, score]) =>
                          score.value >= (formData.min_confidence || 0.7)
                      )
                      .sort(([, a], [, b]) => b.value - a.value)
                      .map(([path, score]) => (
                        <div
                          key={path}
                          className="p-3 border rounded-lg bg-gray-50 dark:bg-gray-800"
                        >
                          <div className="flex items-center justify-between mb-1">
                            <code className="text-sm font-mono">{path}</code>
                            <Badge
                              variant={
                                score.value >= 0.8
                                  ? 'success'
                                  : score.value >= 0.6
                                    ? 'warning'
                                    : 'default'
                              }
                            >
                              {(score.value * 100).toFixed(0)}%
                            </Badge>
                          </div>
                          <p className="text-xs text-gray-600 dark:text-gray-400">
                            {score.reason}
                          </p>
                        </div>
                      ))}
                  </div>
                </div>
              )}
            </Card>
          )}

          {/* Preview Card */}
          {result && showPreview && (
            <Card className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold flex items-center gap-2">
                  <FileText className="h-5 w-5" />
                  OpenAPI Specification Preview
                </h2>
                <Button
                  onClick={() => setShowPreview(!showPreview)}
                  variant="outline"
                  size="sm"
                >
                  {showPreview ? 'Hide' : 'Show'} Preview
                </Button>
              </div>

              <div className="bg-gray-900 text-gray-100 p-4 rounded-lg overflow-auto max-h-96">
                <pre className="text-xs">
                  {JSON.stringify(result.spec, null, 2)}
                </pre>
              </div>
            </Card>
          )}

          {/* Empty State */}
          {!result && !loading && !error && (
            <EmptyState
              icon={<Database className="h-12 w-12 text-gray-400" />}
              title="No OpenAPI Specification Generated"
              description="Configure filters and click 'Generate OpenAPI Spec' to analyze recorded traffic and generate an OpenAPI specification."
            >
              <div className="mt-6 space-y-3 text-sm text-gray-600 dark:text-gray-400">
                <div className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <div className="font-medium">Record Traffic First</div>
                    <div className="text-xs">
                      Use the API Flight Recorder to capture HTTP traffic before generating specs
                    </div>
                  </div>
                </div>
                <div className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <div className="font-medium">Configure Filters</div>
                    <div className="text-xs">
                      Filter by time range, path patterns, or minimum confidence to focus on specific API endpoints
                    </div>
                  </div>
                </div>
                <div className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <div className="font-medium">Review & Download</div>
                    <div className="text-xs">
                      Review the generated spec, check confidence scores, and download as JSON or YAML
                    </div>
                  </div>
                </div>
              </div>
            </EmptyState>
          )}
        </div>
      </div>
    </div>
  );
}
