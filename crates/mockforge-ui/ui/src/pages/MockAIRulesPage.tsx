//! MockAI Rules Dashboard Page
//!
//! Displays a dashboard of all generated rules with their explanations,
//! filtering, search, and detailed rule information.

import { logger } from '@/utils/logger';
import React, { useState, useEffect, useCallback } from 'react';
import {
  Search,
  Filter,
  TrendingUp,
  AlertCircle,
  RefreshCw,
  Eye,
  Code,
  BarChart3,
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
import { RuleExplanationPanel } from '../components/mockai/RuleExplanationPanel';
import { RuleGenerationFlow } from '../components/mockai/RuleGenerationFlow';
import { toast } from 'sonner';

interface RuleExplanation {
  rule_id: string;
  rule_type: string;
  confidence: number;
  source_examples: string[];
  reasoning: string;
  pattern_matches: Array<{
    pattern: string;
    match_count: number;
    example_ids: string[];
  }>;
  generated_at: string;
}

export function MockAIRulesPage() {
  const [loading, setLoading] = useState(true);
  const [explanations, setExplanations] = useState<RuleExplanation[]>([]);
  const [filteredExplanations, setFilteredExplanations] = useState<
    RuleExplanation[]
  >([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedRule, setSelectedRule] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [ruleTypeFilter, setRuleTypeFilter] = useState<string>('all');
  const [minConfidence, setMinConfidence] = useState<number>(0);
  const [showFlow, setShowFlow] = useState(false);

  const fetchExplanations = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const filters: {
        rule_type?: string;
        min_confidence?: number;
      } = {};

      if (ruleTypeFilter !== 'all') {
        filters.rule_type = ruleTypeFilter;
      }

      if (minConfidence > 0) {
        filters.min_confidence = minConfidence;
      }

      const response = await apiService.listRuleExplanations(filters);
      setExplanations(response.explanations);
      setFilteredExplanations(response.explanations);
    } catch (err) {
      const errorMessage =
        err instanceof Error
          ? err.message
          : 'Failed to fetch rule explanations';
      setError(errorMessage);
      logger.error('Failed to fetch rule explanations', err);
      toast.error(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [ruleTypeFilter, minConfidence]);

  useEffect(() => {
    fetchExplanations();
  }, [fetchExplanations]);

  // Filter by search query
  useEffect(() => {
    if (!searchQuery.trim()) {
      setFilteredExplanations(explanations);
      return;
    }

    const query = searchQuery.toLowerCase();
    const filtered = explanations.filter(
      (exp) =>
        exp.rule_id.toLowerCase().includes(query) ||
        exp.reasoning.toLowerCase().includes(query) ||
        exp.rule_type.toLowerCase().includes(query) ||
        exp.pattern_matches.some((pm) =>
          pm.pattern.toLowerCase().includes(query)
        )
    );
    setFilteredExplanations(filtered);
  }, [searchQuery, explanations]);

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

  const formatRuleType = (ruleType: string) => {
    return ruleType
      .split('_')
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  };

  const handleExampleClick = (exampleId: string) => {
    // TODO: Navigate to example viewer or show example details
    toast.info(`Viewing example: ${exampleId}`);
  };

  if (loading && explanations.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <RefreshCw className="h-8 w-8 animate-spin mx-auto mb-4 text-gray-400" />
          <p className="text-gray-600 dark:text-gray-400">Loading rules...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="MockAI Rules Dashboard"
        description="View and explore all generated behavioral rules with detailed explanations, confidence scores, and source examples"
        icon={<BarChart3 className="h-6 w-6" />}
      />

      {/* Filters and Search */}
      <Card className="p-4">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {/* Search */}
          <div className="md:col-span-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search rules by ID, reasoning, or pattern..."
                className="w-full pl-10 pr-4 py-2 border rounded-md text-sm"
              />
            </div>
          </div>

          {/* Rule Type Filter */}
          <div>
            <select
              value={ruleTypeFilter}
              onChange={(e) => setRuleTypeFilter(e.target.value)}
              className="w-full px-3 py-2 border rounded-md text-sm"
            >
              <option value="all">All Rule Types</option>
              <option value="consistency">Consistency</option>
              <option value="validation">Validation</option>
              <option value="pagination">Pagination</option>
              <option value="statetransition">State Transition</option>
              <option value="crud">CRUD</option>
            </select>
          </div>

          {/* Min Confidence Filter */}
          <div>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min="0"
                max="1"
                step="0.1"
                value={minConfidence}
                onChange={(e) => setMinConfidence(parseFloat(e.target.value))}
                className="flex-1"
              />
              <span className="text-sm text-gray-600 dark:text-gray-400 min-w-[60px]">
                ≥{(minConfidence * 100).toFixed(0)}%
              </span>
            </div>
          </div>
        </div>

        {/* Stats */}
        <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
          <span>
            {filteredExplanations.length} of {explanations.length} rules
          </span>
          {explanations.length > 0 && (
            <span>
              Avg confidence:{' '}
              {(
                explanations.reduce((sum, e) => sum + e.confidence, 0) /
                explanations.length
              ).toFixed(2)}
            </span>
          )}
        </div>
      </Card>

      {/* Error Display */}
      {error && (
        <Alert variant="error" title="Error Loading Rules">
          {error}
        </Alert>
      )}

      {/* Rule Generation Flow Visualization */}
      {explanations.length > 0 && (
        <Card className="p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
              Rule Generation Flow
            </h2>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowFlow(!showFlow)}
            >
              {showFlow ? 'Hide' : 'Show'} Flow
            </Button>
          </div>
          {showFlow && (
            <RuleGenerationFlow
              examples={explanations.flatMap((exp) =>
                exp.source_examples.map((id) => ({
                  id,
                  method: 'GET', // Would need to fetch actual method from examples
                  path: exp.pattern_matches[0]?.pattern || 'unknown',
                }))
              )}
              patterns={explanations.flatMap((exp) =>
                exp.pattern_matches.map((pm) => ({
                  pattern: pm.pattern,
                  match_count: pm.match_count,
                  example_ids: pm.example_ids,
                  confidence: exp.confidence,
                }))
              )}
              rules={explanations.map((exp) => ({
                id: exp.rule_id,
                type: exp.rule_type,
                confidence: exp.confidence,
              }))}
              onExampleClick={(id) => toast.info(`Example: ${id}`)}
              onPatternClick={(pattern) => {
                setSearchQuery(pattern);
                toast.info(`Searching for pattern: ${pattern}`);
              }}
              onRuleClick={(ruleId) => {
                setSelectedRule(ruleId);
                toast.info(`Viewing rule: ${ruleId}`);
              }}
            />
          )}
        </Card>
      )}

      {/* Rules List */}
      {filteredExplanations.length === 0 ? (
        <EmptyState
          icon={<Code className="h-12 w-12 text-gray-400" />}
          title="No Rules Found"
          description={
            searchQuery || ruleTypeFilter !== 'all' || minConfidence > 0
              ? 'Try adjusting your filters or search query'
              : 'No rules have been generated yet. Use MockAI to learn from examples and generate rules.'
          }
        />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {filteredExplanations.map((explanation) => (
            <div key={explanation.rule_id}>
              <RuleExplanationPanel
                explanation={explanation}
                onExampleClick={handleExampleClick}
              />
            </div>
          ))}
        </div>
      )}

      {/* Refresh Button */}
      <div className="flex justify-center">
        <Button onClick={fetchExplanations} variant="outline">
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh Rules
        </Button>
      </div>
    </div>
  );
}
