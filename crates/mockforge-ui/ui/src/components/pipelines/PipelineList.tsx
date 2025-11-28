/**
 * Pipeline List Component
 *
 * Displays a list of all pipelines with their status, triggers, and actions
 */

import React from 'react';
import { usePipelines, useDeletePipeline, Pipeline } from '../../hooks/usePipelines';
import { Card } from '../ui/Card';
import { Play, Edit, Trash2, Plus, CheckCircle, XCircle, Clock } from 'lucide-react';

export interface PipelineListProps {
  workspaceId?: string;
  orgId?: string;
  onSelect?: (pipeline: Pipeline) => void;
  onCreate?: () => void;
}

export const PipelineList: React.FC<PipelineListProps> = ({
  workspaceId,
  orgId,
  onSelect,
  onCreate,
}) => {
  const { data: pipelines, isLoading, error } = usePipelines({ workspace_id: workspaceId, org_id: orgId });
  const deletePipeline = useDeletePipeline();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="p-6">
        <div className="text-red-600 dark:text-red-400">
          Error loading pipelines: {error.message}
        </div>
      </Card>
    );
  }

  const handleDelete = async (id: string, name: string) => {
    if (window.confirm(`Are you sure you want to delete pipeline "${name}"?`)) {
      try {
        await deletePipeline.mutateAsync(id);
      } catch (err) {
        alert(`Failed to delete pipeline: ${err instanceof Error ? err.message : 'Unknown error'}`);
      }
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">Pipelines</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Automate mock lifecycle management with event-driven pipelines
          </p>
        </div>
        {onCreate && (
          <button
            onClick={onCreate}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Plus className="h-4 w-4" />
            Create Pipeline
          </button>
        )}
      </div>

      {/* Pipeline List */}
      {!pipelines || pipelines.length === 0 ? (
        <Card className="p-8 text-center">
          <p className="text-gray-600 dark:text-gray-400 mb-4">No pipelines found</p>
          {onCreate && (
            <button
              onClick={onCreate}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Create Your First Pipeline
            </button>
          )}
        </Card>
      ) : (
        <div className="grid gap-4">
          {pipelines.map((pipeline) => (
            <Card key={pipeline.id} className="p-6 hover:shadow-lg transition-shadow">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                      {pipeline.name}
                    </h3>
                    {pipeline.definition.enabled ? (
                      <span className="flex items-center gap-1 px-2 py-1 bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 rounded text-xs">
                        <CheckCircle className="h-3 w-3" />
                        Enabled
                      </span>
                    ) : (
                      <span className="flex items-center gap-1 px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200 rounded text-xs">
                        <XCircle className="h-3 w-3" />
                        Disabled
                      </span>
                    )}
                  </div>

                  <div className="space-y-2 text-sm text-gray-600 dark:text-gray-400">
                    <div className="flex items-center gap-4">
                      <span>
                        <strong>Triggers:</strong> {pipeline.definition.triggers.length}
                      </span>
                      <span>
                        <strong>Steps:</strong> {pipeline.definition.steps.length}
                      </span>
                    </div>

                    {pipeline.definition.triggers.length > 0 && (
                      <div>
                        <strong>Trigger Events:</strong>{' '}
                        {pipeline.definition.triggers
                          .map((t) => t.event_type)
                          .join(', ')}
                      </div>
                    )}

                    <div className="flex items-center gap-2 text-xs">
                      <Clock className="h-3 w-3" />
                      <span>
                        Updated {new Date(pipeline.updated_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 ml-4">
                  {onSelect && (
                    <button
                      onClick={() => onSelect(pipeline)}
                      className="p-2 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                      title="View Details"
                    >
                      <Edit className="h-4 w-4" />
                    </button>
                  )}
                  <button
                    onClick={() => handleDelete(pipeline.id, pipeline.name)}
                    className="p-2 text-gray-600 dark:text-gray-400 hover:text-red-600 dark:hover:text-red-400 transition-colors"
                    title="Delete Pipeline"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
};
