import React, { useState } from 'react';
import { X } from 'lucide-react';
import type { Flow, CompileFlowRequest } from '../../types';
import { apiService } from '../../services/api';
import { logger } from '../../utils/logger';

interface CompileFlowModalProps {
  flow: Flow;
  onClose: () => void;
  onCompiled: () => void;
}

export function CompileFlowModal({ flow, onClose, onCompiled }: CompileFlowModalProps) {
  const [scenarioName, setScenarioName] = useState(flow.name || '');
  const [flexMode, setFlexMode] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!scenarioName.trim()) {
      setError('Scenario name is required');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const request: CompileFlowRequest = {
        scenario_name: scenarioName.trim(),
        flex_mode: flexMode,
      };

      const result = await apiService.compileFlow(flow.id, request);
      alert(`Scenario compiled successfully!\nID: ${result.scenario_id}\nVersion: ${result.version}`);
      onCompiled();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to compile flow';
      setError(errorMessage);
      logger.error('Failed to compile flow', { error: err });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Compile Flow to Scenario</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-destructive/10 text-destructive rounded-md text-sm">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Scenario Name *</label>
            <input
              type="text"
              value={scenarioName}
              onChange={(e) => setScenarioName(e.target.value)}
              className="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="e.g., checkout_success"
              required
            />
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="flexMode"
              checked={flexMode}
              onChange={(e) => setFlexMode(e.target.checked)}
              className="w-4 h-4"
            />
            <label htmlFor="flexMode" className="text-sm">
              Flex Mode (allow minor variations in sequence)
            </label>
          </div>

          <div className="text-sm text-muted-foreground p-3 bg-muted rounded-md">
            <p className="font-medium mb-1">What happens:</p>
            <ul className="list-disc list-inside space-y-1">
              <li>Extracts state variables (user_id, cart_id, etc.)</li>
              <li>Generates step dependencies</li>
              <li>Preserves timing information</li>
              <li>Creates a replayable scenario</li>
            </ul>
          </div>

          <div className="flex gap-2 justify-end pt-4">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded-md transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !scenarioName.trim()}
              className="px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              {loading ? 'Compiling...' : 'Compile'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

