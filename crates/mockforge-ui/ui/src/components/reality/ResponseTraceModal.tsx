import React, { useEffect, useState } from 'react';
import { X, FileCode, Network, GitBranch, Zap, Code2, Sparkles } from 'lucide-react';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';

/**
 * Response Generation Trace
 */
interface ResponseGenerationTrace {
  template_path?: string;
  fixture_path?: string;
  response_selection_mode: string;
  selected_example?: string;
  persona_graph_nodes: Array<{
    persona_id: string;
    entity_type: string;
    usage_type: string;
    relationship_path?: string[];
  }>;
  rules_executed: Array<{
    name: string;
    rule_type: string;
    condition_matched: boolean;
    actions_executed: string[];
    execution_time_ms?: number;
    error?: string;
  }>;
  template_expansions: Array<{
    template: string;
    value: unknown;
    source: string;
    step: number;
  }>;
  blending_decision?: {
    blend_ratio: number;
    ratio_source: string;
    blended: boolean;
    merge_strategy?: string;
    field_decisions: Array<{
      field_path: string;
      field_ratio: number;
      value_source: string;
    }>;
  };
  metadata: Record<string, unknown>;
}

interface ResponseTraceModalProps {
  requestId?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * Response Trace Modal Component
 *
 * Shows detailed explanation of how a response was generated, including:
 * - Template/fixture selection
 * - Persona graph nodes used
 * - Rules/hooks execution
 * - Template expansion steps
 */
export function ResponseTraceModal({ requestId, open, onOpenChange }: ResponseTraceModalProps) {
  const [traceData, setTraceData] = useState<ResponseGenerationTrace | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !requestId) {
      setTraceData(null);
      return;
    }

    const fetchTrace = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`/__mockforge/api/reality/response-trace/${requestId}`);
        if (!response.ok) {
          throw new Error('Failed to fetch response trace');
        }
        const data = await response.json();
        if (data.success && data.data) {
          // Parse JSON string if needed
          const trace = typeof data.data === 'string'
            ? JSON.parse(data.data)
            : data.data;
          setTraceData(trace);
        } else {
          setTraceData(null);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load response trace');
        setTraceData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchTrace();
  }, [open, requestId]);

  const getSelectionModeLabel = (mode: string) => {
    switch (mode.toLowerCase()) {
      case 'first':
        return 'First Available';
      case 'scenario':
        return 'Scenario-Based';
      case 'sequential':
        return 'Round-Robin';
      case 'random':
        return 'Random';
      case 'weightedrandom':
      case 'weighted_random':
      case 'weighted':
        return 'Weighted Random';
      default:
        return mode;
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            Why Did I Get This Response?
          </DialogTitle>
        </DialogHeader>

        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            <span className="ml-3 text-sm text-muted-foreground">Loading trace data...</span>
          </div>
        )}

        {error && (
          <div className="p-4 bg-destructive/10 border border-destructive rounded-md">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        )}

        {!loading && !error && traceData && (
          <div className="space-y-6">
            {/* Template/Fixture Selection */}
            <div>
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                <FileCode className="h-4 w-4" />
                Template & Fixture Selection
              </h3>
              <div className="bg-muted/30 rounded-md p-4 space-y-2">
                {traceData.template_path && (
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Template Path:</span>
                    <code className="text-xs bg-background px-2 py-1 rounded">{traceData.template_path}</code>
                  </div>
                )}
                {traceData.fixture_path && (
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Fixture Path:</span>
                    <code className="text-xs bg-background px-2 py-1 rounded">{traceData.fixture_path}</code>
                  </div>
                )}
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Selection Mode:</span>
                  <Badge variant="outline">{getSelectionModeLabel(traceData.response_selection_mode)}</Badge>
                </div>
                {traceData.selected_example && (
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Selected Example:</span>
                    <Badge variant="secondary">{traceData.selected_example}</Badge>
                  </div>
                )}
              </div>
            </div>

            {/* Persona Graph Nodes */}
            {traceData.persona_graph_nodes.length > 0 && (
              <div>
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <GitBranch className="h-4 w-4" />
                  Persona Graph Nodes Used
                </h3>
                <div className="bg-muted/30 rounded-md p-4 space-y-3">
                  {traceData.persona_graph_nodes.map((node, idx) => (
                    <div key={idx} className="border-l-2 border-primary pl-3">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant="outline" className="font-mono text-xs">
                          {node.persona_id}
                        </Badge>
                        <Badge variant="secondary" className="text-xs">
                          {node.entity_type}
                        </Badge>
                      </div>
                      <div className="text-xs text-muted-foreground">
                        Usage: <span className="font-medium">{node.usage_type}</span>
                      </div>
                      {node.relationship_path && node.relationship_path.length > 0 && (
                        <div className="text-xs text-muted-foreground mt-1">
                          Path: {node.relationship_path.join(' → ')}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Rules Executed */}
            {traceData.rules_executed.length > 0 && (
              <div>
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Zap className="h-4 w-4" />
                  Rules & Hooks Executed
                </h3>
                <div className="bg-muted/30 rounded-md p-4 space-y-3">
                  {traceData.rules_executed.map((rule, idx) => (
                    <div key={idx} className="border-l-2 border-yellow-500 pl-3">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-medium">{rule.name}</span>
                        <Badge variant={rule.condition_matched ? 'default' : 'secondary'} className="text-xs">
                          {rule.rule_type}
                        </Badge>
                        {rule.condition_matched && (
                          <Badge variant="outline" className="text-xs bg-green-500/10">
                            Matched
                          </Badge>
                        )}
                      </div>
                      {rule.actions_executed.length > 0 && (
                        <div className="text-xs text-muted-foreground mt-1">
                          Actions: {rule.actions_executed.join(', ')}
                        </div>
                      )}
                      {rule.execution_time_ms && (
                        <div className="text-xs text-muted-foreground">
                          Execution time: {rule.execution_time_ms}ms
                        </div>
                      )}
                      {rule.error && (
                        <div className="text-xs text-destructive mt-1">
                          Error: {rule.error}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Template Expansions */}
            {traceData.template_expansions.length > 0 && (
              <div>
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Code2 className="h-4 w-4" />
                  Template Expansion Steps
                </h3>
                <div className="bg-muted/30 rounded-md p-4 space-y-3">
                  {traceData.template_expansions.map((expansion, idx) => (
                    <div key={idx} className="border-l-2 border-blue-500 pl-3">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-xs text-muted-foreground">Step {expansion.step}</span>
                        <Badge variant="outline" className="text-xs">
                          {expansion.source}
                        </Badge>
                      </div>
                      <div className="text-xs font-mono bg-background p-2 rounded mt-1">
                        {expansion.template}
                      </div>
                      <div className="text-xs text-muted-foreground mt-1">
                        → {typeof expansion.value === 'string' ? expansion.value : JSON.stringify(expansion.value)}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Blending Decision */}
            {traceData.blending_decision && (
              <div>
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <Network className="h-4 w-4" />
                  Reality Blending Decision
                </h3>
                <div className="bg-muted/30 rounded-md p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Blend Ratio:</span>
                    <span className="text-sm font-mono">{(traceData.blending_decision.blend_ratio * 100).toFixed(1)}%</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Ratio Source:</span>
                    <Badge variant="outline">{traceData.blending_decision.ratio_source}</Badge>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Blended:</span>
                    <Badge variant={traceData.blending_decision.blended ? 'default' : 'secondary'}>
                      {traceData.blending_decision.blended ? 'Yes' : 'No'}
                    </Badge>
                  </div>
                  {traceData.blending_decision.merge_strategy && (
                    <div className="flex items-center justify-between">
                      <span className="text-sm text-muted-foreground">Merge Strategy:</span>
                      <Badge variant="outline">{traceData.blending_decision.merge_strategy}</Badge>
                    </div>
                  )}
                  {traceData.blending_decision.field_decisions.length > 0 && (
                    <div className="mt-3">
                      <div className="text-xs text-muted-foreground mb-2">Field-Level Decisions:</div>
                      <div className="space-y-1">
                        {traceData.blending_decision.field_decisions.map((field, idx) => (
                          <div key={idx} className="flex items-center justify-between text-xs">
                            <code className="bg-background px-2 py-0.5 rounded">{field.field_path}</code>
                            <div className="flex items-center gap-2">
                              <span className="text-muted-foreground">{(field.field_ratio * 100).toFixed(0)}%</span>
                              <Badge variant="outline" className="text-xs">{field.value_source}</Badge>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Metadata */}
            {Object.keys(traceData.metadata).length > 0 && (
              <div>
                <h3 className="text-sm font-semibold mb-3">Additional Metadata</h3>
                <div className="bg-muted/30 rounded-md p-4">
                  <pre className="text-xs font-mono overflow-x-auto">
                    {JSON.stringify(traceData.metadata, null, 2)}
                  </pre>
                </div>
              </div>
            )}
          </div>
        )}

        {!loading && !error && !traceData && (
          <div className="p-4 text-center text-sm text-muted-foreground">
            No trace data available for this request. Response generation trace may not be enabled.
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
