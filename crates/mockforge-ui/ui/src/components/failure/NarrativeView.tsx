//! Failure Narrative View Component
//!
//! Displays AI-generated failure narratives with stack traces and contributing factors.

import React, { useState, useEffect } from 'react';
import { AlertCircle, CheckCircle2, XCircle, ChevronDown, ChevronRight, Lightbulb } from 'lucide-react';
import { Card } from '../ui/Card';

interface NarrativeViewProps {
  requestId: string;
  className?: string;
}

interface FailureNarrative {
  summary: string;
  explanation: string;
  stack_trace: NarrativeFrame[];
  contributing_factors: ContributingFactor[];
  suggested_fixes: string[];
  confidence: number;
}

interface NarrativeFrame {
  description: string;
  trigger: string;
  source: string;
  source_type: string;
}

interface ContributingFactor {
  description: string;
  factor_type: string;
  impact: string;
}

export function NarrativeView({ requestId, className }: NarrativeViewProps) {
  const [narrative, setNarrative] = useState<FailureNarrative | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedFrames, setExpandedFrames] = useState<Set<number>>(new Set([0]));

  useEffect(() => {
    loadNarrative();
  }, [requestId]);

  const loadNarrative = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`/api/v2/failures/${requestId}`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const responseData = await response.json();
      const data = responseData.data || responseData;

      if (data.narrative) {
        setNarrative(data.narrative);
      } else {
        setError('No narrative available for this failure');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load narrative');
    } finally {
      setLoading(false);
    }
  };

  const toggleFrame = (index: number) => {
    const newExpanded = new Set(expandedFrames);
    if (newExpanded.has(index)) {
      newExpanded.delete(index);
    } else {
      newExpanded.add(index);
    }
    setExpandedFrames(newExpanded);
  };

  const getImpactColor = (impact: string) => {
    switch (impact.toLowerCase()) {
      case 'high':
        return 'text-red-600 bg-red-50 border-red-200';
      case 'medium':
        return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      case 'low':
        return 'text-blue-600 bg-blue-50 border-blue-200';
      default:
        return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const getSourceTypeColor = (sourceType: string) => {
    switch (sourceType.toLowerCase()) {
      case 'rule':
        return 'bg-purple-100 text-purple-800';
      case 'persona':
        return 'bg-blue-100 text-blue-800';
      case 'contract':
        return 'bg-green-100 text-green-800';
      case 'chaos':
        return 'bg-red-100 text-red-800';
      case 'hook':
        return 'bg-orange-100 text-orange-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  if (loading) {
    return (
      <Card className={`p-6 ${className || ''}`}>
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
          <span className="ml-3 text-muted-foreground">Loading narrative...</span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={`p-6 ${className || ''}`}>
        <div className="flex items-start gap-3 text-red-600">
          <XCircle className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <div>
            <h3 className="font-medium">Error</h3>
            <p className="text-sm mt-1">{error}</p>
          </div>
        </div>
      </Card>
    );
  }

  if (!narrative) {
    return (
      <Card className={`p-6 ${className || ''}`}>
        <div className="text-center py-8 text-muted-foreground">
          No narrative available for this failure
        </div>
      </Card>
    );
  }

  return (
    <div className={`space-y-4 ${className || ''}`}>
      {/* Summary */}
      <Card className="p-6">
        <div className="flex items-start gap-3">
          <AlertCircle className="w-6 h-6 text-red-600 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h2 className="text-xl font-semibold mb-2">Failure Summary</h2>
            <p className="text-gray-700">{narrative.summary}</p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Confidence:</span>
              <div className="flex-1 bg-gray-200 rounded-full h-2 max-w-xs">
                <div
                  className="bg-primary h-2 rounded-full"
                  style={{ width: `${narrative.confidence * 100}%` }}
                ></div>
              </div>
              <span className="text-sm font-medium">
                {Math.round(narrative.confidence * 100)}%
              </span>
            </div>
          </div>
        </div>
      </Card>

      {/* Explanation */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-3">Detailed Explanation</h2>
        <p className="text-gray-700 whitespace-pre-wrap">{narrative.explanation}</p>
      </Card>

      {/* Stack Trace */}
      {narrative.stack_trace.length > 0 && (
        <Card className="p-6">
          <h2 className="text-xl font-semibold mb-4">Narrative Stack Trace</h2>
          <div className="space-y-2">
            {narrative.stack_trace.map((frame, index) => (
              <div
                key={index}
                className="border border-gray-200 rounded-lg overflow-hidden"
              >
                <button
                  onClick={() => toggleFrame(index)}
                  className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    {expandedFrames.has(index) ? (
                      <ChevronDown className="w-4 h-4 text-gray-500" />
                    ) : (
                      <ChevronRight className="w-4 h-4 text-gray-500" />
                    )}
                    <span className="font-medium text-left">{frame.description}</span>
                    <span
                      className={`px-2 py-1 rounded text-xs font-medium ${getSourceTypeColor(
                        frame.source_type
                      )}`}
                    >
                      {frame.source_type}
                    </span>
                  </div>
                </button>
                {expandedFrames.has(index) && (
                  <div className="px-4 pb-3 pt-2 bg-gray-50 border-t border-gray-200">
                    <div className="space-y-2 text-sm">
                      <div>
                        <span className="font-medium text-gray-700">Trigger:</span>
                        <span className="ml-2 text-gray-600">{frame.trigger}</span>
                      </div>
                      <div>
                        <span className="font-medium text-gray-700">Source:</span>
                        <span className="ml-2 text-gray-600">{frame.source}</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Contributing Factors */}
      {narrative.contributing_factors.length > 0 && (
        <Card className="p-6">
          <h2 className="text-xl font-semibold mb-4">Contributing Factors</h2>
          <div className="space-y-3">
            {narrative.contributing_factors.map((factor, index) => (
              <div
                key={index}
                className={`p-4 rounded-lg border ${getImpactColor(factor.impact)}`}
              >
                <div className="flex items-start justify-between mb-2">
                  <span className="font-medium">{factor.description}</span>
                  <span className="text-xs font-medium uppercase px-2 py-1 rounded bg-white/50">
                    {factor.impact} impact
                  </span>
                </div>
                <div className="text-xs text-gray-600 mt-1">
                  Type: {factor.factor_type}
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Suggested Fixes */}
      {narrative.suggested_fixes.length > 0 && (
        <Card className="p-6">
          <div className="flex items-center gap-2 mb-4">
            <Lightbulb className="w-5 h-5 text-yellow-600" />
            <h2 className="text-xl font-semibold">Suggested Fixes</h2>
          </div>
          <ul className="space-y-2">
            {narrative.suggested_fixes.map((fix, index) => (
              <li key={index} className="flex items-start gap-3">
                <CheckCircle2 className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span className="text-gray-700">{fix}</span>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </div>
  );
}

