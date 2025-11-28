//! Rule Explanation Panel Component
//!
//! Displays detailed information about a generated rule, including
//! confidence scores, reasoning, source examples, and pattern matches.

import React, { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Info,
  TrendingUp,
  FileText,
  Code,
  CheckCircle2,
  AlertCircle,
  Clock,
} from 'lucide-react';
import { Card, Badge, Button } from '../ui/DesignSystem';

interface PatternMatch {
  pattern: string;
  match_count: number;
  example_ids: string[];
}

interface RuleExplanation {
  rule_id: string;
  rule_type: string;
  confidence: number;
  source_examples: string[];
  reasoning: string;
  pattern_matches: PatternMatch[];
  generated_at: string;
}

interface RuleExplanationPanelProps {
  explanation: RuleExplanation;
  onExampleClick?: (exampleId: string) => void;
}

export function RuleExplanationPanel({
  explanation,
  onExampleClick,
}: RuleExplanationPanelProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['reasoning', 'pattern_matches'])
  );

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(section)) {
        next.delete(section);
      } else {
        next.add(section);
      }
      return next;
    });
  };

  const getRuleTypeColor = (ruleType: string) => {
    switch (ruleType.toLowerCase()) {
      case 'consistency':
        return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
      case 'validation':
        return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
      case 'pagination':
        return 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200';
      case 'statetransition':
      case 'state_transition':
        return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
      case 'crud':
        return 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200';
    }
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return 'success';
    if (confidence >= 0.6) return 'warning';
    return 'default';
  };

  const formatRuleType = (ruleType: string) => {
    return ruleType
      .split('_')
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  };

  return (
    <Card className="p-6 space-y-4">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <h3 className="text-lg font-semibold">{explanation.rule_id}</h3>
            <Badge className={getRuleTypeColor(explanation.rule_type)}>
              {formatRuleType(explanation.rule_type)}
            </Badge>
            <Badge variant={getConfidenceColor(explanation.confidence)}>
              {(explanation.confidence * 100).toFixed(0)}% confidence
            </Badge>
          </div>
          <div className="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
            <div className="flex items-center gap-1">
              <Clock className="h-4 w-4" />
              {new Date(explanation.generated_at).toLocaleString()}
            </div>
            {explanation.source_examples.length > 0 && (
              <div className="flex items-center gap-1">
                <FileText className="h-4 w-4" />
                {explanation.source_examples.length} source example
                {explanation.source_examples.length !== 1 ? 's' : ''}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Reasoning Section */}
      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <button
          onClick={() => toggleSection('reasoning')}
          className="flex items-center justify-between w-full text-left mb-2"
        >
          <div className="flex items-center gap-2 font-medium">
            {expandedSections.has('reasoning') ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
            <Info className="h-4 w-4" />
            Reasoning
          </div>
        </button>
        {expandedSections.has('reasoning') && (
          <div className="pl-6 text-sm text-gray-700 dark:text-gray-300">
            {explanation.reasoning}
          </div>
        )}
      </div>

      {/* Source Examples Section */}
      {explanation.source_examples.length > 0 && (
        <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
          <button
            onClick={() => toggleSection('source_examples')}
            className="flex items-center justify-between w-full text-left mb-2"
          >
            <div className="flex items-center gap-2 font-medium">
              {expandedSections.has('source_examples') ? (
                <ChevronDown className="h-4 w-4" />
              ) : (
                <ChevronRight className="h-4 w-4" />
              )}
              <FileText className="h-4 w-4" />
              Source Examples ({explanation.source_examples.length})
            </div>
          </button>
          {expandedSections.has('source_examples') && (
            <div className="pl-6 space-y-2">
              {explanation.source_examples.map((exampleId, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-2 bg-gray-50 dark:bg-gray-800 rounded"
                >
                  <code className="text-sm font-mono">{exampleId}</code>
                  {onExampleClick && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onExampleClick(exampleId)}
                    >
                      View
                    </Button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Pattern Matches Section */}
      {explanation.pattern_matches.length > 0 && (
        <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
          <button
            onClick={() => toggleSection('pattern_matches')}
            className="flex items-center justify-between w-full text-left mb-2"
          >
            <div className="flex items-center gap-2 font-medium">
              {expandedSections.has('pattern_matches') ? (
                <ChevronDown className="h-4 w-4" />
              ) : (
                <ChevronRight className="h-4 w-4" />
              )}
              <TrendingUp className="h-4 w-4" />
              Pattern Matches ({explanation.pattern_matches.length})
            </div>
          </button>
          {expandedSections.has('pattern_matches') && (
            <div className="pl-6 space-y-3">
              {explanation.pattern_matches.map((match, idx) => (
                <div
                  key={idx}
                  className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg"
                >
                  <div className="flex items-center justify-between mb-2">
                    <code className="text-sm font-mono font-semibold">
                      {match.pattern}
                    </code>
                    <Badge variant="default">
                      {match.match_count} match{match.match_count !== 1 ? 'es' : ''}
                    </Badge>
                  </div>
                  {match.example_ids.length > 0 && (
                    <div className="mt-2">
                      <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                        Matched Examples:
                      </div>
                      <div className="flex flex-wrap gap-1">
                        {match.example_ids.slice(0, 5).map((exampleId, eIdx) => (
                          <code
                            key={eIdx}
                            className="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded"
                          >
                            {exampleId}
                          </code>
                        ))}
                        {match.example_ids.length > 5 && (
                          <span className="text-xs text-gray-500">
                            +{match.example_ids.length - 5} more
                          </span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Confidence Indicator */}
      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <div className="flex items-center gap-2 mb-2">
          <TrendingUp className="h-4 w-4 text-gray-600 dark:text-gray-400" />
          <span className="text-sm font-medium">Confidence Score</span>
        </div>
        <div className="pl-6">
          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div
              className={`h-2 rounded-full ${
                explanation.confidence >= 0.8
                  ? 'bg-green-500'
                  : explanation.confidence >= 0.6
                    ? 'bg-yellow-500'
                    : 'bg-red-500'
              }`}
              style={{ width: `${explanation.confidence * 100}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-gray-600 dark:text-gray-400 mt-1">
            <span>Low (0%)</span>
            <span>High (100%)</span>
          </div>
        </div>
      </div>
    </Card>
  );
}
