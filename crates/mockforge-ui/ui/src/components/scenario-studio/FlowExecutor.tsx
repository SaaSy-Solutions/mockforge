//! Flow Execution Preview Component
//!
//! Component for previewing and executing flows with real-time step-by-step visualization.

import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import { Play, Square, CheckCircle2, XCircle, Clock, Loader2 } from 'lucide-react';
import { cloudFlowsApi } from '../../services/api/cloudFlows';
import { isCloudMode } from '../../utils/cloudMode';

interface FlowExecutionStep {
  stepId: string;
  stepName: string;
  stepType: string;
  status: 'pending' | 'running' | 'success' | 'error';
  duration?: number;
  error?: string;
  result?: any;
}

interface FlowExecutorProps {
  flowId: string;
  onClose: () => void;
}

export function FlowExecutor({ flowId, onClose }: FlowExecutorProps) {
  const [isExecuting, setIsExecuting] = useState(false);
  const [executionSteps, setExecutionSteps] = useState<FlowExecutionStep[]>([]);
  const [currentStepIndex, setCurrentStepIndex] = useState(-1);
  const [cloudRunInfo, setCloudRunInfo] = useState<{
    runId: string;
    status: string;
  } | null>(null);

  const executeFlow = async () => {
    setIsExecuting(true);
    setExecutionSteps([]);
    setCurrentStepIndex(-1);
    setCloudRunInfo(null);

    try {
      if (isCloudMode()) {
        // Cloud-mode scenario flows queue a test_run and stream live
        // progress over cloudTestRunsApi.streamRunEvents. This panel
        // surfaces the queued status; users follow the SSE timeline on
        // the Cloud Test Runs page.
        const run = await cloudFlowsApi.triggerRun(flowId);
        setCloudRunInfo({ runId: run.id, status: run.status });
        return;
      }

      const response = await fetch(`/api/v1/scenario-studio/flows/${flowId}/execute`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ variables: {} }),
      });

      if (!response.ok) {
        throw new Error('Failed to execute flow');
      }

      const result = await response.json();

      // Handle both direct result and wrapped response
      const executionResult = result.data || result;

      // Simulate step-by-step execution visualization
      if (executionResult.step_results && Array.isArray(executionResult.step_results)) {
        for (let i = 0; i < executionResult.step_results.length; i++) {
          const stepResult = executionResult.step_results[i];
          setCurrentStepIndex(i);
          setExecutionSteps((prev) => [
            ...prev,
            {
              stepId: stepResult.step_id || stepResult.id || `step-${i}`,
              stepName: stepResult.name || `Step ${i + 1}`,
              stepType: stepResult.step_type || 'api_call',
              status: 'running',
            },
          ]);

          // Simulate execution delay
          await new Promise((resolve) => setTimeout(resolve, 500));

          setExecutionSteps((prev) => {
            const updated = [...prev];
            updated[i] = {
              ...updated[i],
              status: stepResult.success ? 'success' : 'error',
              duration: stepResult.duration_ms,
              error: stepResult.error,
              result: stepResult.response,
            };
            return updated;
          });
        }
      }
    } catch (error) {
      console.error('Flow execution error:', error);
    } finally {
      setIsExecuting(false);
      setCurrentStepIndex(-1);
    }
  };

  const stopExecution = () => {
    setIsExecuting(false);
    setCurrentStepIndex(-1);
  };

  const getStatusIcon = (status: FlowExecutionStep['status']) => {
    switch (status) {
      case 'pending':
        return <Clock className="h-4 w-4 text-muted-foreground" />;
      case 'running':
        return <Loader2 className="h-4 w-4 text-info-500 animate-spin" />;
      case 'success':
        return <CheckCircle2 className="h-4 w-4 text-success-500" />;
      case 'error':
        return <XCircle className="h-4 w-4 text-danger-500" />;
    }
  };

  const getStatusBadge = (status: FlowExecutionStep['status']) => {
    switch (status) {
      case 'pending':
        return <Badge variant="outline">Pending</Badge>;
      case 'running':
        return <Badge className="bg-info-500">Running</Badge>;
      case 'success':
        return <Badge className="bg-success-500">Success</Badge>;
      case 'error':
        return <Badge className="bg-danger-500">Error</Badge>;
    }
  };

  return (
    <Card className="w-96">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Flow Execution</CardTitle>
          <div className="flex gap-2">
            {!isExecuting ? (
              <Button size="sm" onClick={executeFlow}>
                <Play className="h-4 w-4 mr-2" />
                Execute
              </Button>
            ) : (
              <Button size="sm" variant="destructive" onClick={stopExecution}>
                <Square className="h-4 w-4 mr-2" />
                Stop
              </Button>
            )}
            <Button size="sm" variant="ghost" onClick={onClose}>
              Close
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {cloudRunInfo && (
            <div className="p-3 border rounded-lg bg-info-50 dark:bg-info-900/20 text-xs space-y-1">
              <div className="font-medium">
                Run queued: <span className="font-mono">{cloudRunInfo.runId}</span>
              </div>
              <div className="text-muted-foreground">
                Status: {cloudRunInfo.status}. Live progress streams on the
                Cloud Test Runs page.
              </div>
            </div>
          )}
          {executionSteps.length === 0 && !isExecuting && !cloudRunInfo && (
            <div className="text-center text-muted-foreground py-8">
              Click Execute to run the flow
            </div>
          )}
          {executionSteps.map((step, index) => (
            <div
              key={step.stepId}
              className={`p-3 border rounded-lg ${
                index === currentStepIndex ? 'bg-info-50 border-info-300' : ''
              }`}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  {getStatusIcon(step.status)}
                  <span className="font-medium text-sm">{step.stepName}</span>
                </div>
                {getStatusBadge(step.status)}
              </div>
              <div className="text-xs text-muted-foreground mb-1">
                Type: {step.stepType}
                {step.duration && ` • Duration: ${step.duration}ms`}
              </div>
              {step.error && (
                <div className="text-xs text-danger-600 mt-1 bg-danger-50 p-2 rounded">
                  {step.error}
                </div>
              )}
              {step.result && (
                <div className="text-xs text-muted-foreground mt-1 bg-muted p-2 rounded font-mono">
                  {JSON.stringify(step.result, null, 2)}
                </div>
              )}
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
