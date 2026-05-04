/**
 * Pipeline Detail Component
 *
 * Displays detailed information about a pipeline including its configuration,
 * execution history, and status
 */

import React from 'react';
import { usePipeline, useTriggerPipeline, type Pipeline } from '../../hooks/usePipelines';
import { Card } from '../ui/Card';
import { ArrowLeft, Edit, Play, CheckCircle, XCircle, Clock, Settings } from 'lucide-react';

export interface PipelineDetailProps {
  pipeline: Pipeline;
  onEdit?: () => void;
  onViewExecutions?: () => void;
  onBack?: () => void;
}

export const PipelineDetail: React.FC<PipelineDetailProps> = ({
  pipeline: initialPipeline,
  onEdit,
  onViewExecutions,
  onBack,
}) => {
  const { data: pipeline, isLoading } = usePipeline(initialPipeline.id);
  const triggerPipeline = useTriggerPipeline();

  const currentPipeline = pipeline || initialPipeline;

  const handleTrigger = async () => {
    if (window.confirm('Trigger this pipeline manually?')) {
      try {
        await triggerPipeline.mutateAsync({ id: currentPipeline.id });
        alert('Pipeline triggered successfully!');
      } catch (err) {
        alert(`Failed to trigger pipeline: ${err instanceof Error ? err.message : 'Unknown error'}`);
      }
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-info-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          {onBack && (
            <button
              onClick={onBack}
              className="p-2 text-muted-foreground hover:text-foreground transition-colors"
            >
              <ArrowLeft className="h-5 w-5" />
            </button>
          )}
          <div>
            <h1 className="text-3xl font-bold text-foreground">
              {currentPipeline.name}
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              Pipeline Details
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {currentPipeline.definition.enabled ? (
            <span className="flex items-center gap-1 px-3 py-1 bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300 rounded">
              <CheckCircle className="h-4 w-4" />
              Enabled
            </span>
          ) : (
            <span className="flex items-center gap-1 px-3 py-1 bg-muted text-foreground rounded">
              <XCircle className="h-4 w-4" />
              Disabled
            </span>
          )}
          <button
            onClick={handleTrigger}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
          >
            <Play className="h-4 w-4" />
            Trigger
          </button>
          {onEdit && (
            <button
              onClick={onEdit}
              className="flex items-center gap-2 px-4 py-2 border border-border rounded-lg hover:bg-accent hover:text-accent-foreground transition-colors"
            >
              <Edit className="h-4 w-4" />
              Edit
            </button>
          )}
        </div>
      </div>

      {/* Pipeline Information */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Basic Info */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Basic Information
          </h3>
          <div className="space-y-3 text-sm">
            <div>
              <span className="text-muted-foreground">Pipeline ID:</span>
              <span className="ml-2 font-mono text-foreground">{currentPipeline.id}</span>
            </div>
            {currentPipeline.workspace_id && (
              <div>
                <span className="text-muted-foreground">Workspace ID:</span>
                <span className="ml-2 font-mono text-foreground">{currentPipeline.workspace_id}</span>
              </div>
            )}
            {currentPipeline.org_id && (
              <div>
                <span className="text-muted-foreground">Organization ID:</span>
                <span className="ml-2 font-mono text-foreground">{currentPipeline.org_id}</span>
              </div>
            )}
            <div>
              <span className="text-muted-foreground">Created:</span>
              <span className="ml-2 text-foreground">
                {new Date(currentPipeline.created_at).toLocaleString()}
              </span>
            </div>
            <div>
              <span className="text-muted-foreground">Updated:</span>
              <span className="ml-2 text-foreground">
                {new Date(currentPipeline.updated_at).toLocaleString()}
              </span>
            </div>
          </div>
        </Card>

        {/* Triggers */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-foreground mb-4">
            Triggers ({currentPipeline.definition.triggers.length})
          </h3>
          {currentPipeline.definition.triggers.length === 0 ? (
            <p className="text-sm text-muted-foreground">No triggers configured</p>
          ) : (
            <div className="space-y-2">
              {currentPipeline.definition.triggers.map((trigger, index) => (
                <div key={index} className="p-3 bg-muted rounded">
                  <div className="font-medium text-foreground">
                    {trigger.event_type}
                  </div>
                  {trigger.filters && Object.keys(trigger.filters).length > 0 && (
                    <div className="text-xs text-muted-foreground mt-1">
                      Filters: {JSON.stringify(trigger.filters)}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      {/* Steps */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold text-foreground mb-4">
          Steps ({currentPipeline.definition.steps.length})
        </h3>
        {currentPipeline.definition.steps.length === 0 ? (
          <p className="text-sm text-muted-foreground">No steps configured</p>
        ) : (
          <div className="space-y-3">
            {currentPipeline.definition.steps.map((step, index) => (
              <div key={index} className="p-4 bg-muted rounded border-l-4 border-info-600">
                <div className="flex items-center justify-between mb-2">
                  <div className="font-medium text-foreground">
                    {step.name || `Step ${index + 1}`}
                  </div>
                  <span className="px-2 py-1 bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-300 rounded text-xs">
                    {step.type}
                  </span>
                </div>
                {step.config && Object.keys(step.config).length > 0 && (
                  <div className="text-xs text-muted-foreground mt-2">
                    <pre className="bg-muted dark:bg-gray-900 p-2 rounded overflow-x-auto">
                      {JSON.stringify(step.config, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Actions */}
      {onViewExecutions && (
        <Card className="p-6">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-lg font-semibold text-foreground mb-1">
                Execution History
              </h3>
              <p className="text-sm text-muted-foreground">
                View past pipeline executions and their results
              </p>
            </div>
            <button
              onClick={onViewExecutions}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              View Executions
            </button>
          </div>
        </Card>
      )}
    </div>
  );
};
