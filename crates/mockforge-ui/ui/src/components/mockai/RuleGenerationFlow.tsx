//! Rule Generation Flow Visualization Component
//!
//! Visualizes the rule generation process: input examples → pattern detection → rule generation
//! with confidence scores and intermediate steps.

import React, { useState } from 'react';
import {
  FileText,
  Search,
  Sparkles,
  CheckCircle2,
  ArrowRight,
  TrendingUp,
  AlertCircle,
  Loader2,
  Eye,
} from 'lucide-react';
import { Card, Badge, Button } from '../ui/DesignSystem';

interface PatternMatch {
  pattern: string;
  match_count: number;
  example_ids: string[];
  confidence: number;
}

interface RuleGenerationStep {
  id: string;
  title: string;
  description: string;
  status: 'pending' | 'processing' | 'completed' | 'error';
  icon: React.ReactNode;
  data?: {
    examples_count?: number;
    patterns_detected?: number;
    rules_generated?: number;
    confidence?: number;
    details?: string;
  };
}

interface RuleGenerationFlowProps {
  examples?: Array<{ id: string; method: string; path: string }>;
  patterns?: PatternMatch[];
  rules?: Array<{ id: string; type: string; confidence: number }>;
  onExampleClick?: (exampleId: string) => void;
  onPatternClick?: (pattern: string) => void;
  onRuleClick?: (ruleId: string) => void;
}

export function RuleGenerationFlow({
  examples = [],
  patterns = [],
  rules = [],
  onExampleClick,
  onPatternClick,
  onRuleClick,
}: RuleGenerationFlowProps) {
  const [selectedStep, setSelectedStep] = useState<string | null>(null);

  // Determine step statuses based on available data
  const steps: RuleGenerationStep[] = [
    {
      id: 'input',
      title: 'Input Examples',
      description: 'Example request/response pairs provided for learning',
      status: examples.length > 0 ? 'completed' : 'pending',
      icon: <FileText className="h-5 w-5" />,
      data: {
        examples_count: examples.length,
      },
    },
    {
      id: 'pattern',
      title: 'Pattern Detection',
      description: 'Analyzing examples to detect behavioral patterns',
      status:
        patterns.length > 0
          ? 'completed'
          : examples.length > 0
            ? 'processing'
            : 'pending',
      icon: <Search className="h-5 w-5" />,
      data: {
        patterns_detected: patterns.length,
      },
    },
    {
      id: 'generation',
      title: 'Rule Generation',
      description: 'Generating behavioral rules from detected patterns',
      status:
        rules.length > 0
          ? 'completed'
          : patterns.length > 0
            ? 'processing'
            : 'pending',
      icon: <Sparkles className="h-5 w-5" />,
      data: {
        rules_generated: rules.length,
        confidence:
          rules.length > 0
            ? rules.reduce((sum, r) => sum + r.confidence, 0) / rules.length
            : undefined,
      },
    },
    {
      id: 'output',
      title: 'Generated Rules',
      description: 'Final behavioral rules ready for use',
      status: rules.length > 0 ? 'completed' : 'pending',
      icon: <CheckCircle2 className="h-5 w-5" />,
      data: {
        rules_generated: rules.length,
      },
    },
  ];

  const getStepColor = (status: RuleGenerationStep['status']) => {
    switch (status) {
      case 'completed':
        return 'text-green-600 dark:text-green-400 bg-green-100 dark:bg-green-900';
      case 'processing':
        return 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900';
      case 'error':
        return 'text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-900';
      default:
        return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800';
    }
  };

  const getStepBorderColor = (status: RuleGenerationStep['status']) => {
    switch (status) {
      case 'completed':
        return 'border-green-500';
      case 'processing':
        return 'border-blue-500';
      case 'error':
        return 'border-red-500';
      default:
        return 'border-gray-300 dark:border-gray-700';
    }
  };

  return (
    <div className="space-y-6">
      {/* Flow Steps */}
      <div className="relative">
        {/* Connection Lines */}
        <div className="absolute top-1/2 left-0 right-0 h-0.5 bg-gray-200 dark:bg-gray-700 -translate-y-1/2 z-0" />
        <div
          className="absolute top-1/2 left-0 h-0.5 bg-blue-500 dark:bg-blue-400 -translate-y-1/2 z-0 transition-all duration-500"
          style={{
            width: `${((steps.findIndex((s) => s.status !== 'completed') + 1) / steps.length) * 100}%`,
          }}
        />

        {/* Steps */}
        <div className="relative z-10 grid grid-cols-4 gap-4">
          {steps.map((step, index) => (
            <div key={step.id} className="flex flex-col items-center">
              <button
                onClick={() =>
                  setSelectedStep(selectedStep === step.id ? null : step.id)
                }
                className={`relative w-16 h-16 rounded-full border-4 ${getStepBorderColor(
                  step.status
                )} ${getStepColor(step.status)} flex items-center justify-center transition-all hover:scale-110`}
              >
                {step.status === 'processing' ? (
                  <Loader2 className="h-6 w-6 animate-spin" />
                ) : (
                  step.icon
                )}
                {step.status === 'completed' && (
                  <div className="absolute -top-1 -right-1 w-5 h-5 bg-green-500 rounded-full flex items-center justify-center">
                    <CheckCircle2 className="h-3 w-3 text-white" />
                  </div>
                )}
              </button>
              <div className="mt-3 text-center">
                <div className="text-sm font-semibold">{step.title}</div>
                {step.data && (
                  <div className="text-xs text-gray-600 dark:text-gray-400 mt-1">
                    {step.data.examples_count !== undefined &&
                      `${step.data.examples_count} examples`}
                    {step.data.patterns_detected !== undefined &&
                      `${step.data.patterns_detected} patterns`}
                    {step.data.rules_generated !== undefined &&
                      `${step.data.rules_generated} rules`}
                    {step.data.confidence !== undefined &&
                      `${(step.data.confidence * 100).toFixed(0)}% avg confidence`}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Step Details */}
      {selectedStep && (
        <Card className="p-6">
          {(() => {
            const step = steps.find((s) => s.id === selectedStep);
            if (!step) return null;

            return (
              <div>
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div className={`p-2 rounded-lg ${getStepColor(step.status)}`}>
                      {step.icon}
                    </div>
                    <div>
                      <h3 className="text-lg font-semibold">{step.title}</h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {step.description}
                      </p>
                    </div>
                  </div>
                  <Badge
                    variant={
                      step.status === 'completed'
                        ? 'success'
                        : step.status === 'processing'
                          ? 'default'
                          : 'default'
                    }
                  >
                    {step.status}
                  </Badge>
                </div>

                {/* Step-specific content */}
                {step.id === 'input' && examples.length > 0 && (
                  <div className="space-y-2">
                    <h4 className="font-medium">Examples ({examples.length})</h4>
                    <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto">
                      {examples.map((example) => (
                        <button
                          key={example.id}
                          onClick={() => onExampleClick?.(example.id)}
                          className="p-2 text-left border rounded hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                        >
                          <div className="text-xs font-mono">
                            {example.method} {example.path}
                          </div>
                          <div className="text-xs text-gray-500">{example.id}</div>
                        </button>
                      ))}
                    </div>
                  </div>
                )}

                {step.id === 'pattern' && patterns.length > 0 && (
                  <div className="space-y-3">
                    <h4 className="font-medium">Detected Patterns ({patterns.length})</h4>
                    {patterns.map((pattern, idx) => (
                      <div
                        key={idx}
                        className="p-3 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                      >
                        <div className="flex items-center justify-between mb-2">
                          <code
                            className="text-sm font-mono cursor-pointer hover:text-blue-600 dark:hover:text-blue-400"
                            onClick={() => onPatternClick?.(pattern.pattern)}
                          >
                            {pattern.pattern}
                          </code>
                          <Badge variant="default">
                            {pattern.match_count} matches
                          </Badge>
                        </div>
                        <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                          <TrendingUp className="h-3 w-3" />
                          {pattern.confidence > 0 && (
                            <span>
                              {(pattern.confidence * 100).toFixed(0)}% confidence
                            </span>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {step.id === 'generation' && rules.length > 0 && (
                  <div className="space-y-3">
                    <h4 className="font-medium">Generated Rules ({rules.length})</h4>
                    <div className="grid grid-cols-2 gap-2">
                      {rules.map((rule) => (
                        <button
                          key={rule.id}
                          onClick={() => onRuleClick?.(rule.id)}
                          className="p-3 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-left"
                        >
                          <div className="flex items-center justify-between mb-1">
                            <span className="text-sm font-medium">{rule.id}</span>
                            <Badge
                              variant={
                                rule.confidence >= 0.8
                                  ? 'success'
                                  : rule.confidence >= 0.6
                                    ? 'warning'
                                    : 'default'
                              }
                            >
                              {(rule.confidence * 100).toFixed(0)}%
                            </Badge>
                          </div>
                          <div className="text-xs text-gray-600 dark:text-gray-400">
                            {rule.type}
                          </div>
                        </button>
                      ))}
                    </div>
                  </div>
                )}

                {step.id === 'output' && rules.length > 0 && (
                  <div className="space-y-3">
                    <h4 className="font-medium">Final Rules</h4>
                    <div className="p-4 bg-green-50 dark:bg-green-900/20 rounded-lg">
                      <div className="flex items-center gap-2 text-green-700 dark:text-green-400 mb-2">
                        <CheckCircle2 className="h-5 w-5" />
                        <span className="font-medium">
                          Successfully generated {rules.length} rule
                          {rules.length !== 1 ? 's' : ''}
                        </span>
                      </div>
                      <p className="text-sm text-green-600 dark:text-green-400">
                        Rules are ready to use for intelligent mock behavior. View details
                        in the Rules Dashboard.
                      </p>
                    </div>
                    <Button
                      variant="outline"
                      onClick={() => {
                        window.location.hash = 'mockai-rules';
                      }}
                    >
                      <Eye className="h-4 w-4 mr-2" />
                      View Rules Dashboard
                    </Button>
                  </div>
                )}
              </div>
            );
          })()}
        </Card>
      )}

      {/* Summary Stats */}
      <div className="grid grid-cols-4 gap-4">
        <Card className="p-4 text-center">
          <div className="text-2xl font-bold text-blue-600 dark:text-blue-400 mb-1">
            {examples.length}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">Examples</div>
        </Card>
        <Card className="p-4 text-center">
          <div className="text-2xl font-bold text-purple-600 dark:text-purple-400 mb-1">
            {patterns.length}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">Patterns</div>
        </Card>
        <Card className="p-4 text-center">
          <div className="text-2xl font-bold text-green-600 dark:text-green-400 mb-1">
            {rules.length}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">Rules</div>
        </Card>
        <Card className="p-4 text-center">
          <div className="text-2xl font-bold text-orange-600 dark:text-orange-400 mb-1">
            {rules.length > 0
              ? `${(rules.reduce((sum, r) => sum + r.confidence, 0) / rules.length * 100).toFixed(0)}%`
              : 'N/A'}
          </div>
          <div className="text-sm text-gray-600 dark:text-gray-400">Avg Confidence</div>
        </Card>
      </div>
    </div>
  );
}
