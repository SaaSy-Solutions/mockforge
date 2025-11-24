//! API Critique Component
//!
//! Provides UI for analyzing API schemas (OpenAPI, GraphQL, Protobuf)
//! and receiving LLM-powered critique with anti-pattern detection,
//! redundancy analysis, naming quality assessment, tone analysis,
//! and restructuring recommendations.

import React, { useState } from 'react';
import {
  Upload,
  FileText,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Info,
  Download,
  Loader2,
  Code,
  MessageSquare,
  TrendingDown,
  RefreshCw,
} from 'lucide-react';
import { Card } from '../ui/Card';
import { Button } from '../components/ui/button';
import { Textarea } from '../components/ui/textarea';
import { Label } from '../components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';

interface ApiCritique {
  anti_patterns: AntiPattern[];
  redundancies: Redundancy[];
  naming_issues: NamingIssue[];
  tone_analysis: ToneAnalysis;
  restructuring: RestructuringRecommendations;
  overall_score: number;
  summary: string;
  tokens_used?: number;
  cost_usd?: number;
}

interface AntiPattern {
  pattern_type: string;
  severity: string;
  location: string;
  description: string;
  suggestion: string;
  example?: string;
}

interface Redundancy {
  redundancy_type: string;
  severity: string;
  affected_items: string[];
  description: string;
  suggestion: string;
}

interface NamingIssue {
  issue_type: string;
  severity: string;
  location: string;
  current_name: string;
  description: string;
  suggestion: string;
}

interface ToneAnalysis {
  overall_tone: string;
  error_message_issues: ToneIssue[];
  user_facing_issues: ToneIssue[];
  recommendations: string[];
}

interface ToneIssue {
  issue_type: string;
  severity: string;
  location: string;
  current_text: string;
  description: string;
  suggestion: string;
}

interface RestructuringRecommendations {
  hierarchy_improvements: HierarchyImprovement[];
  consolidation_opportunities: ConsolidationOpportunity[];
  resource_modeling: ResourceModelingSuggestion[];
  priority: string;
}

interface HierarchyImprovement {
  current: string;
  suggested: string;
  rationale: string;
  impact: string;
}

interface ConsolidationOpportunity {
  items: string[];
  description: string;
  suggestion: string;
  benefits: string[];
}

interface ResourceModelingSuggestion {
  current: string;
  suggested: string;
  rationale: string;
}

interface ApiCritiqueProps {
  onUsageUpdate?: () => void;
}

export function ApiCritique({ onUsageUpdate }: ApiCritiqueProps) {
  const [schemaType, setSchemaType] = useState<string>('openapi');
  const [schemaContent, setSchemaContent] = useState<string>('');
  const [focusAreas, setFocusAreas] = useState<string[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [critique, setCritique] = useState<ApiCritique | null>(null);
  const [artifactId, setArtifactId] = useState<string | null>(null);

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      setSchemaContent(content);

      // Try to detect schema type from file extension
      if (file.name.endsWith('.graphql') || file.name.endsWith('.gql')) {
        setSchemaType('graphql');
      } else if (file.name.endsWith('.proto')) {
        setSchemaType('protobuf');
      } else {
        setSchemaType('openapi');
      }
    };
    reader.readAsText(file);
  };

  const handleAnalyze = async () => {
    if (!schemaContent.trim()) {
      toast.error('Please provide a schema to analyze');
      return;
    }

    try {
      setIsAnalyzing(true);

      // Parse schema content as JSON (for OpenAPI/Protobuf) or use as-is (for GraphQL)
      let schemaJson;
      if (schemaType === 'graphql') {
        // GraphQL is plain text, wrap it
        schemaJson = schemaContent;
      } else {
        try {
          schemaJson = JSON.parse(schemaContent);
        } catch (e) {
          toast.error('Invalid JSON schema. Please check your schema format.');
          setIsAnalyzing(false);
          return;
        }
      }

      const response = await fetch('/__mockforge/api/v1/ai-studio/api-critique', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          schema: schemaJson,
          schema_type: schemaType,
          focus_areas: focusAreas,
        }),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(error.error || `HTTP ${response.status}`);
      }

      const result = await response.json();
      setCritique(result.critique);
      setArtifactId(result.artifact_id || null);

      toast.success('API critique completed successfully');

      if (onUsageUpdate) {
        onUsageUpdate();
      }
    } catch (error: any) {
      logger.error('API critique failed', error);
      toast.error(`Analysis failed: ${error.message}`);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleExport = () => {
    if (!critique) return;

    const dataStr = JSON.stringify(critique, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `api-critique-${Date.now()}.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const getSeverityColor = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical':
        return 'text-red-600 dark:text-red-400';
      case 'high':
        return 'text-orange-600 dark:text-orange-400';
      case 'medium':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'low':
        return 'text-blue-600 dark:text-blue-400';
      default:
        return 'text-gray-600 dark:text-gray-400';
    }
  };

  const getSeverityBadge = (severity: string) => {
    const color = getSeverityColor(severity);
    return (
      <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${color} bg-opacity-10`}>
        {severity.toUpperCase()}
      </span>
    );
  };

  const getScoreColor = (score: number) => {
    if (score >= 80) return 'text-green-600 dark:text-green-400';
    if (score >= 60) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-red-600 dark:text-red-400';
  };

  return (
    <div className="space-y-6">
      {/* Input Section */}
      <Card className="p-6">
        <div className="space-y-4">
          <div>
            <Label htmlFor="schema-type">Schema Type</Label>
            <Select value={schemaType} onValueChange={setSchemaType}>
              <SelectTrigger id="schema-type">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="openapi">OpenAPI</SelectItem>
                <SelectItem value="graphql">GraphQL</SelectItem>
                <SelectItem value="protobuf">Protobuf</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div>
            <Label htmlFor="schema-content">Schema Content</Label>
            <div className="mt-2 space-y-2">
              <div className="flex items-center space-x-2">
                <input
                  type="file"
                  accept=".json,.yaml,.yml,.graphql,.gql,.proto"
                  onChange={handleFileUpload}
                  className="hidden"
                  id="file-upload"
                />
                <label htmlFor="file-upload">
                  <Button variant="outline" asChild>
                    <span>
                      <Upload className="w-4 h-4 mr-2" />
                      Upload File
                    </span>
                  </Button>
                </label>
              </div>
              <Textarea
                id="schema-content"
                value={schemaContent}
                onChange={(e) => setSchemaContent(e.target.value)}
                placeholder={`Paste your ${schemaType} schema here...`}
                rows={12}
                className="font-mono text-sm"
              />
            </div>
          </div>

          <div>
            <Label>Focus Areas (Optional)</Label>
            <div className="mt-2 flex flex-wrap gap-2">
              {['anti-patterns', 'redundancy', 'naming', 'tone', 'restructuring'].map((area) => (
                <Button
                  key={area}
                  variant={focusAreas.includes(area) ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => {
                    setFocusAreas((prev) =>
                      prev.includes(area)
                        ? prev.filter((a) => a !== area)
                        : [...prev, area]
                    );
                  }}
                >
                  {area}
                </Button>
              ))}
            </div>
          </div>

          <Button
            onClick={handleAnalyze}
            disabled={isAnalyzing || !schemaContent.trim()}
            className="w-full"
          >
            {isAnalyzing ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Analyzing...
              </>
            ) : (
              <>
                <Code className="w-4 h-4 mr-2" />
                Analyze Schema
              </>
            )}
          </Button>
        </div>
      </Card>

      {/* Results Section */}
      {critique && (
        <div className="space-y-6">
          {/* Summary Card */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold">Analysis Summary</h3>
              <div className="flex items-center space-x-2">
                {artifactId && (
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    Artifact ID: {artifactId.substring(0, 8)}...
                  </span>
                )}
                <Button variant="outline" size="sm" onClick={handleExport}>
                  <Download className="w-4 h-4 mr-2" />
                  Export
                </Button>
              </div>
            </div>

            <div className="space-y-4">
              <div className="flex items-center space-x-4">
                <div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Overall Score</div>
                  <div className={`text-3xl font-bold ${getScoreColor(critique.overall_score)}`}>
                    {critique.overall_score.toFixed(1)}
                  </div>
                </div>
                {critique.tokens_used && (
                  <div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">Tokens Used</div>
                    <div className="text-lg font-semibold">{critique.tokens_used.toLocaleString()}</div>
                  </div>
                )}
                {critique.cost_usd && (
                  <div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">Cost</div>
                    <div className="text-lg font-semibold">${critique.cost_usd.toFixed(4)}</div>
                  </div>
                )}
              </div>

              <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
                <p className="text-sm text-gray-700 dark:text-gray-300">{critique.summary}</p>
              </div>
            </div>
          </Card>

          {/* Anti-patterns */}
          {critique.anti_patterns.length > 0 && (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4 flex items-center">
                <AlertTriangle className="w-5 h-5 mr-2 text-orange-600 dark:text-orange-400" />
                Anti-patterns ({critique.anti_patterns.length})
              </h3>
              <div className="space-y-4">
                {critique.anti_patterns.map((pattern, idx) => (
                  <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <div className="font-medium">{pattern.pattern_type}</div>
                        <div className="text-sm text-gray-600 dark:text-gray-400">{pattern.location}</div>
                      </div>
                      {getSeverityBadge(pattern.severity)}
                    </div>
                    <p className="text-sm mb-2">{pattern.description}</p>
                    <div className="text-sm">
                      <strong>Suggestion:</strong> {pattern.suggestion}
                    </div>
                    {pattern.example && (
                      <div className="mt-2 p-2 bg-gray-100 dark:bg-gray-700 rounded text-xs font-mono">
                        {pattern.example}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* Redundancies */}
          {critique.redundancies.length > 0 && (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4 flex items-center">
                <RefreshCw className="w-5 h-5 mr-2 text-blue-600 dark:text-blue-400" />
                Redundancies ({critique.redundancies.length})
              </h3>
              <div className="space-y-4">
                {critique.redundancies.map((redundancy, idx) => (
                  <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <div className="font-medium">{redundancy.redundancy_type}</div>
                        <div className="text-sm text-gray-600 dark:text-gray-400">
                          Affected: {redundancy.affected_items.join(', ')}
                        </div>
                      </div>
                      {getSeverityBadge(redundancy.severity)}
                    </div>
                    <p className="text-sm mb-2">{redundancy.description}</p>
                    <div className="text-sm">
                      <strong>Suggestion:</strong> {redundancy.suggestion}
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* Naming Issues */}
          {critique.naming_issues.length > 0 && (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4 flex items-center">
                <FileText className="w-5 h-5 mr-2 text-purple-600 dark:text-purple-400" />
                Naming Issues ({critique.naming_issues.length})
              </h3>
              <div className="space-y-4">
                {critique.naming_issues.map((issue, idx) => (
                  <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <div className="font-medium">{issue.issue_type}</div>
                        <div className="text-sm text-gray-600 dark:text-gray-400">{issue.location}</div>
                      </div>
                      {getSeverityBadge(issue.severity)}
                    </div>
                    <div className="text-sm mb-2">
                      <strong>Current:</strong> <code className="bg-gray-200 dark:bg-gray-700 px-1 rounded">{issue.current_name}</code>
                    </div>
                    <p className="text-sm mb-2">{issue.description}</p>
                    <div className="text-sm">
                      <strong>Suggestion:</strong> <code className="bg-green-100 dark:bg-green-900 px-1 rounded">{issue.suggestion}</code>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* Tone Analysis */}
          <Card className="p-6">
            <h3 className="text-lg font-semibold mb-4 flex items-center">
              <MessageSquare className="w-5 h-5 mr-2 text-indigo-600 dark:text-indigo-400" />
              Tone Analysis
            </h3>
            <div className="space-y-4">
              <div>
                <div className="text-sm font-medium mb-2">Overall Tone: {critique.tone_analysis.overall_tone}</div>
              </div>

              {critique.tone_analysis.error_message_issues.length > 0 && (
                <div>
                  <div className="text-sm font-medium mb-2">Error Message Issues</div>
                  <div className="space-y-2">
                    {critique.tone_analysis.error_message_issues.map((issue, idx) => (
                      <div key={idx} className="p-3 bg-gray-50 dark:bg-gray-800 rounded">
                        <div className="flex items-center justify-between mb-1">
                          <span className="text-sm font-medium">{issue.location}</span>
                          {getSeverityBadge(issue.severity)}
                        </div>
                        <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                          Current: {issue.current_text}
                        </div>
                        <div className="text-xs">
                          <strong>Suggestion:</strong> {issue.suggestion}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {critique.tone_analysis.recommendations.length > 0 && (
                <div>
                  <div className="text-sm font-medium mb-2">Recommendations</div>
                  <ul className="list-disc list-inside space-y-1 text-sm">
                    {critique.tone_analysis.recommendations.map((rec, idx) => (
                      <li key={idx}>{rec}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </Card>

          {/* Restructuring Recommendations */}
          {critique.restructuring.hierarchy_improvements.length > 0 ||
           critique.restructuring.consolidation_opportunities.length > 0 ||
           critique.restructuring.resource_modeling.length > 0 ? (
            <Card className="p-6">
              <h3 className="text-lg font-semibold mb-4 flex items-center">
                <TrendingDown className="w-5 h-5 mr-2 text-teal-600 dark:text-teal-400" />
                Restructuring Recommendations
                {getSeverityBadge(critique.restructuring.priority)}
              </h3>
              <div className="space-y-6">
                {critique.restructuring.hierarchy_improvements.length > 0 && (
                  <div>
                    <div className="text-sm font-medium mb-3">Hierarchy Improvements</div>
                    <div className="space-y-3">
                      {critique.restructuring.hierarchy_improvements.map((improvement, idx) => (
                        <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                          <div className="grid grid-cols-2 gap-4 mb-2">
                            <div>
                              <div className="text-xs text-gray-600 dark:text-gray-400">Current</div>
                              <div className="text-sm font-mono">{improvement.current}</div>
                            </div>
                            <div>
                              <div className="text-xs text-gray-600 dark:text-gray-400">Suggested</div>
                              <div className="text-sm font-mono text-green-600 dark:text-green-400">
                                {improvement.suggested}
                              </div>
                            </div>
                          </div>
                          <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                            Impact: {improvement.impact}
                          </div>
                          <div className="text-sm">{improvement.rationale}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {critique.restructuring.consolidation_opportunities.length > 0 && (
                  <div>
                    <div className="text-sm font-medium mb-3">Consolidation Opportunities</div>
                    <div className="space-y-3">
                      {critique.restructuring.consolidation_opportunities.map((opp, idx) => (
                        <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                          <div className="text-sm mb-2">
                            <strong>Items:</strong> {opp.items.join(', ')}
                          </div>
                          <p className="text-sm mb-2">{opp.description}</p>
                          <div className="text-sm mb-2">
                            <strong>Suggestion:</strong> {opp.suggestion}
                          </div>
                          {opp.benefits.length > 0 && (
                            <div>
                              <div className="text-xs font-medium mb-1">Benefits:</div>
                              <ul className="list-disc list-inside text-xs space-y-1">
                                {opp.benefits.map((benefit, bidx) => (
                                  <li key={bidx}>{benefit}</li>
                                ))}
                              </ul>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {critique.restructuring.resource_modeling.length > 0 && (
                  <div>
                    <div className="text-sm font-medium mb-3">Resource Modeling Suggestions</div>
                    <div className="space-y-3">
                      {critique.restructuring.resource_modeling.map((suggestion, idx) => (
                        <div key={idx} className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
                          <div className="grid grid-cols-2 gap-4 mb-2">
                            <div>
                              <div className="text-xs text-gray-600 dark:text-gray-400">Current</div>
                              <div className="text-sm">{suggestion.current}</div>
                            </div>
                            <div>
                              <div className="text-xs text-gray-600 dark:text-gray-400">Suggested</div>
                              <div className="text-sm text-green-600 dark:text-green-400">
                                {suggestion.suggested}
                              </div>
                            </div>
                          </div>
                          <div className="text-sm">{suggestion.rationale}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </Card>
          ) : null}
        </div>
      )}
    </div>
  );
}
