/**
 * Pipeline Executions Component
 *
 * Displays execution history for a pipeline
 */

import React from 'react';
import { usePipelineExecutions, type PipelineExecution } from '../../hooks/usePipelines';
import { Card } from '../ui/Card';
import { ArrowLeft, CheckCircle, XCircle, Clock, Loader } from 'lucide-react';

export interface PipelineExecutionsProps {
  pipelineId: string;
  onBack?: () => void;
}

const statusColors = {
  started: 'bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-300',
  running: 'bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300',
  completed: 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300',
  failed: 'bg-danger-100 text-danger-700 dark:bg-danger-900/30 dark:text-danger-300',
  cancelled: 'bg-muted text-foreground',
};

const statusIcons = {
  started: Clock,
  running: Loader,
  completed: CheckCircle,
  failed: XCircle,
  cancelled: XCircle,
};

export const PipelineExecutions: React.FC<PipelineExecutionsProps> = ({
  pipelineId,
  onBack,
}) => {
  const { data: executions, isLoading, error } = usePipelineExecutions({ pipeline_id: pipelineId });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-info-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="p-6">
        <div className="text-danger-600 dark:text-danger-400">
          Error loading executions: {error.message}
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        {onBack && (
          <button
            onClick={onBack}
            className="p-2 text-muted-foreground hover:text-foreground transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <h1 className="text-3xl font-bold text-foreground">
          Pipeline Executions
        </h1>
      </div>

      {!executions || executions.length === 0 ? (
        <Card className="p-8 text-center">
          <p className="text-muted-foreground">No executions found</p>
        </Card>
      ) : (
        <div className="space-y-4">
          {executions.map((execution) => {
            const StatusIcon = statusIcons[execution.status] || Clock;
            return (
              <Card key={execution.id} className="p-6">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <span
                        className={`flex items-center gap-1 px-2 py-1 rounded text-xs ${statusColors[execution.status]}`}
                      >
                        <StatusIcon className="h-3 w-3" />
                        {execution.status}
                      </span>
                      <span className="text-xs text-muted-foreground font-mono">
                        {execution.id}
                      </span>
                    </div>

                    <div className="space-y-1 text-sm text-muted-foreground">
                      <div>
                        <strong>Started:</strong>{' '}
                        {new Date(execution.started_at).toLocaleString()}
                      </div>
                      {execution.completed_at && (
                        <div>
                          <strong>Completed:</strong>{' '}
                          {new Date(execution.completed_at).toLocaleString()}
                        </div>
                      )}
                      {execution.error_message && (
                        <div className="text-danger-600 dark:text-danger-400">
                          <strong>Error:</strong> {execution.error_message}
                        </div>
                      )}
                    </div>

                    {execution.trigger_event && (
                      <div className="mt-3">
                        <details className="text-sm">
                          <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                            Trigger Event
                          </summary>
                          <pre className="mt-2 p-3 bg-muted rounded overflow-x-auto">
                            {JSON.stringify(execution.trigger_event, null, 2)}
                          </pre>
                        </details>
                      </div>
                    )}
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
};
