/**
 * Pipeline Form Component
 *
 * Form for creating and editing pipelines
 */

import React, { useState, useEffect } from 'react';
import { useCreatePipeline, useUpdatePipeline, Pipeline, PipelineDefinition } from '../../hooks/usePipelines';
import { Card } from '../ui/Card';
import { ArrowLeft, Save } from 'lucide-react';

export interface PipelineFormProps {
  pipeline?: Pipeline;
  workspaceId?: string;
  orgId?: string;
  onSave?: () => void;
  onCancel?: () => void;
}

export const PipelineForm: React.FC<PipelineFormProps> = ({
  pipeline,
  workspaceId,
  orgId,
  onSave,
  onCancel,
}) => {
  const [name, setName] = useState(pipeline?.name || '');
  const [enabled, setEnabled] = useState(pipeline?.definition.enabled ?? true);
  const [triggersJson, setTriggersJson] = useState(
    JSON.stringify(pipeline?.definition.triggers || [], null, 2)
  );
  const [stepsJson, setStepsJson] = useState(
    JSON.stringify(pipeline?.definition.steps || [], null, 2)
  );

  const createPipeline = useCreatePipeline();
  const updatePipeline = useUpdatePipeline();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    try {
      let triggers: any[] = [];
      let steps: any[] = [];

      try {
        triggers = JSON.parse(triggersJson);
      } catch (err) {
        alert('Invalid triggers JSON');
        return;
      }

      try {
        steps = JSON.parse(stepsJson);
      } catch (err) {
        alert('Invalid steps JSON');
        return;
      }

      const definition: PipelineDefinition = {
        enabled,
        triggers,
        steps,
      };

      if (pipeline) {
        await updatePipeline.mutateAsync({
          id: pipeline.id,
          data: { name, definition, enabled },
        });
      } else {
        await createPipeline.mutateAsync({
          name,
          definition,
          workspace_id: workspaceId,
          org_id: orgId,
        });
      }

      onSave?.();
    } catch (err) {
      alert(`Failed to save pipeline: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        {onCancel && (
          <button
            onClick={onCancel}
            className="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          {pipeline ? 'Edit Pipeline' : 'Create Pipeline'}
        </h1>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        <Card className="p-6">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Pipeline Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
              />
            </div>

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="enabled"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
                className="w-4 h-4"
              />
              <label htmlFor="enabled" className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Enabled
              </label>
            </div>
          </div>
        </Card>

        <Card className="p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Triggers (JSON)
          </h3>
          <textarea
            value={triggersJson}
            onChange={(e) => setTriggersJson(e.target.value)}
            rows={10}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono text-sm"
            placeholder='[{"event_type": "schema_changed", "filters": {}}]'
          />
        </Card>

        <Card className="p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Steps (JSON)
          </h3>
          <textarea
            value={stepsJson}
            onChange={(e) => setStepsJson(e.target.value)}
            rows={15}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono text-sm"
            placeholder='[{"name": "regenerate_sdk", "type": "regenerate_sdk", "config": {"languages": ["typescript", "rust"]}}]'
          />
        </Card>

        <div className="flex items-center justify-end gap-4">
          {onCancel && (
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            >
              Cancel
            </button>
          )}
          <button
            type="submit"
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Save className="h-4 w-4" />
            Save Pipeline
          </button>
        </div>
      </form>
    </div>
  );
};
