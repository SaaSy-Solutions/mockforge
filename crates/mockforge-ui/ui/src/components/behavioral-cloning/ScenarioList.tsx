import React, { useState } from 'react';
import { Download, Eye, Play, FileCode, Snowflake, Sparkles } from 'lucide-react';
import type { Scenario } from '../../types';
import { apiService } from '../../services/api';
import { ModernCard, ModernBadge } from '../ui/DesignSystem';
import { logger } from '../../utils/logger';

interface ScenarioListProps {
  scenarios: Scenario[];
  onRefresh: () => void;
}

export function ScenarioList({ scenarios, onRefresh }: ScenarioListProps) {
  const [exporting, setExporting] = useState<string | null>(null);

  const handleExport = async (scenario: Scenario, format: 'yaml' | 'json') => {
    try {
      setExporting(scenario.id);
      const content = await apiService.exportScenario(scenario.id, format);

      // Create download link
      const blob = new Blob([content], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${scenario.name}-v${scenario.version}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      logger.error('Failed to export scenario', { error: err });
      alert(`Failed to export scenario: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setExporting(null);
    }
  };

  if (scenarios.length === 0) {
    return (
      <ModernCard className="p-12 text-center">
        <FileCode className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
        <h3 className="text-lg font-semibold mb-2">No scenarios yet</h3>
        <p className="text-muted-foreground">
          Compile a flow to create your first behavioral scenario.
        </p>
      </ModernCard>
    );
  }

  return (
    <div className="space-y-4">
      {scenarios.map((scenario) => (
        <ModernCard key={scenario.id} className="p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-2">
                <h3 className="text-lg font-semibold">{scenario.name}</h3>
                <ModernBadge variant="outline">v{scenario.version}</ModernBadge>
                {scenario.ai_generated && (
                  <ModernBadge
                    variant="secondary"
                    size="sm"
                    className="flex items-center gap-1"
                    title="AI-generated scenario"
                  >
                    <Sparkles className="h-3 w-3" />
                    AI
                  </ModernBadge>
                )}
                {scenario.frozen && (
                  <ModernBadge
                    variant="outline"
                    size="sm"
                    className="flex items-center gap-1 border-blue-300 text-blue-700"
                    title={`Frozen artifact (deterministic mode)${scenario.frozen_path ? `: ${scenario.frozen_path}` : ''}`}
                  >
                    <Snowflake className="h-3 w-3" />
                    Frozen
                  </ModernBadge>
                )}
                {scenario.tags && scenario.tags.length > 0 && (
                  <div className="flex gap-2">
                    {scenario.tags.map((tag) => (
                      <ModernBadge key={tag} variant="secondary" size="sm">
                        {tag}
                      </ModernBadge>
                    ))}
                  </div>
                )}
              </div>
              {scenario.description && (
                <p className="text-sm text-muted-foreground mb-3">{scenario.description}</p>
              )}
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <div>Created: {new Date(scenario.created_at).toLocaleString()}</div>
                <div>Updated: {new Date(scenario.updated_at).toLocaleString()}</div>
              </div>
            </div>
            <div className="flex gap-2 ml-4">
              <button
                onClick={() => handleExport(scenario, 'yaml')}
                disabled={exporting === scenario.id}
                className="px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded-md transition-colors flex items-center gap-2 disabled:opacity-50"
                title="Export as YAML"
              >
                <Download className="h-4 w-4" />
                YAML
              </button>
              <button
                onClick={() => handleExport(scenario, 'json')}
                disabled={exporting === scenario.id}
                className="px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded-md transition-colors flex items-center gap-2 disabled:opacity-50"
                title="Export as JSON"
              >
                <Download className="h-4 w-4" />
                JSON
              </button>
            </div>
          </div>
        </ModernCard>
      ))}
    </div>
  );
}
