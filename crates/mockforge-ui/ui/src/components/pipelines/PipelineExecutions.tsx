/**
 * Pipeline Executions Component
 *
 * Displays execution history for a pipeline
 */

import React from 'react';
import { usePipelineExecutions, PipelineExecution } from '../../hooks/usePipelines';
import { Card } from '../ui/Card';
import { ArrowLeft, CheckCircle, XCircle, Clock, Loader } from 'lucide-react';

export interface PipelineExecutionsProps {
  pipelineId: string;
  onBack?: () => void;
}

const statusColors = {
  started: 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200',
  running: 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200',
  completed: 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200',
  failed: 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200',
  cancelled: 'bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200',
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
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="p-6">
        <div className="text-red-600 dark:text-red-400">
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
            className="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          Pipeline Executions
        </h1>
      </div>

      {!executions || executions.length === 0 ? (
        <Card className="p-8 text-center">
          <p className="text-gray-600 dark:text-gray-400">No executions found</p>
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
                      <span className="text-xs text-gray-600 dark:text-gray-400 font-mono">
                        {execution.id}
                      </span>
                    </div>

                    <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
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
                        <div className="text-red-600 dark:text-red-400">
                          <strong>Error:</strong> {execution.error_message}
                        </div>
                      )}
                    </div>

                    {execution.trigger_event && (
                      <div className="mt-3">
                        <details className="text-sm">
                          <summary className="cursor-pointer text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white">
                            Trigger Event
                          </summary>
                          <pre className="mt-2 p-3 bg-gray-50 dark:bg-gray-800 rounded overflow-x-auto">
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
