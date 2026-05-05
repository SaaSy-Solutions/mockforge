/**
 * Real-time Orchestration Execution Visualization
 *
 * Provides real-time visualization of orchestration execution with WebSocket updates,
 * step progress tracking, and live metrics display.
 */

import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  LinearProgress,
  Chip,
  Grid,
  List,
  ListItem,
  ListItemText,
  IconButton,
  Button,
  Alert,
  Stepper,
  Step,
  StepLabel,
  StepContent,
  Tooltip,
} from '@mui/material';
import {
  PlayArrow as PlayIcon,
  Stop as StopIcon,
  Pause as PauseIcon,
  SkipNext as SkipIcon,
  CheckCircle as SuccessIcon,
  Error as ErrorIcon,
  HourglassEmpty as PendingIcon,
  Speed as SpeedIcon,
} from '@mui/icons-material';
import { useWebSocket } from '../hooks/useWebSocket';
import { cloudFlowsApi, type Flow as CloudFlow } from '../services/api/cloudFlows';
import { cloudTestRunsApi } from '../services/api/cloudTestRuns';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import { Select, MenuItem } from '@mui/material';

interface ExecutionStep {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
  startTime?: Date;
  endTime?: Date;
  duration?: number;
  error?: string;
  metrics?: {
    requestCount: number;
    errorRate: number;
    avgLatency: number;
  };
}

interface ExecutionState {
  orchestrationId: string;
  name: string;
  status: 'idle' | 'running' | 'paused' | 'completed' | 'failed';
  currentIteration: number;
  maxIterations: number;
  currentStep: number;
  totalSteps: number;
  progress: number;
  steps: ExecutionStep[];
  startTime?: Date;
  endTime?: Date;
  failedSteps: string[];
}

export const OrchestrationExecutionView: React.FC<{ orchestrationId: string }> = ({
  orchestrationId,
}) => {
  const [executionState, setExecutionState] = useState<ExecutionState>({
    orchestrationId,
    name: 'Loading...',
    status: 'idle',
    currentIteration: 0,
    maxIterations: 1,
    currentStep: 0,
    totalSteps: 0,
    progress: 0,
    steps: [],
    failedSteps: [],
  });

  // Local-mode WebSocket. The relative URL is stubbed in cloud mode so
  // the connection is a no-op there; the cloud-mode SSE stream below
  // takes over.
  const { lastMessage, sendMessage, connected: isLocalConnected } = useWebSocket(
    `/api/chaos/orchestration/${orchestrationId}/ws`,
  );

  // Cloud-mode wiring: pick a kind='orchestration' flow from the active
  // workspace, queue runs through cloudFlowsApi.triggerRun, and tail the
  // resulting test_runs row via cloudTestRunsApi.streamRunEvents.
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const [cloudFlows, setCloudFlows] = useState<CloudFlow[]>([]);
  const [selectedCloudFlowId, setSelectedCloudFlowId] = useState<string | null>(null);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [sseConnected, setSseConnected] = useState(false);

  useEffect(() => {
    if (!isCloudMode() || !activeWorkspace?.id) return;
    let cancelled = false;
    (async () => {
      try {
        const flows = await cloudFlowsApi.listForWorkspace(
          activeWorkspace.id,
          'orchestration',
        );
        if (cancelled) return;
        setCloudFlows(flows);
        if (flows.length > 0 && !selectedCloudFlowId) {
          setSelectedCloudFlowId(flows[0].id);
          setExecutionState((prev) => ({ ...prev, name: flows[0].name }));
        }
      } catch (err) {
        console.error('Failed to list cloud orchestrations', err);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeWorkspace?.id]);

  // Open SSE for the active run; close it when the run id changes.
  useEffect(() => {
    if (!isCloudMode() || !activeRunId) return;
    const es = cloudTestRunsApi.streamRunEvents(activeRunId);
    setSseConnected(false);
    es.onopen = () => setSseConnected(true);
    es.onerror = () => setSseConnected(false);

    // Step lifecycle events. The cloud test_run_events use snake_case
    // event_type values; map them onto the existing ExecutionStep shape.
    const handleStep = (status: ExecutionStep['status']) => (e: MessageEvent) => {
      try {
        const payload = JSON.parse(e.data);
        const stepId: string = payload.step_id ?? payload.id ?? '';
        if (!stepId) return;
        setExecutionState((prev) => {
          const existing = prev.steps.find((s) => s.id === stepId);
          const step: ExecutionStep = {
            id: stepId,
            name: payload.name ?? existing?.name ?? stepId,
            status,
            startTime: payload.started_at
              ? new Date(payload.started_at)
              : existing?.startTime,
            endTime: payload.finished_at
              ? new Date(payload.finished_at)
              : existing?.endTime,
            duration: payload.duration_seconds ?? existing?.duration,
            error: payload.error ?? existing?.error,
            metrics: payload.metrics ?? existing?.metrics,
          };
          const steps = existing
            ? prev.steps.map((s) => (s.id === stepId ? step : s))
            : [...prev.steps, step];
          const failedSteps =
            status === 'failed' && !prev.failedSteps.includes(stepId)
              ? [...prev.failedSteps, stepId]
              : prev.failedSteps;
          return {
            ...prev,
            steps,
            failedSteps,
            status: status === 'running' ? 'running' : prev.status,
          };
        });
      } catch {
        // ignore parse errors
      }
    };

    es.addEventListener('step_start', handleStep('running'));
    es.addEventListener('step_pass', handleStep('completed'));
    es.addEventListener('step_fail', handleStep('failed'));
    es.addEventListener('step_skip', handleStep('skipped'));

    es.addEventListener('done', (e: MessageEvent) => {
      try {
        const payload = JSON.parse(e.data);
        setExecutionState((prev) => ({
          ...prev,
          status: payload.status === 'passed' ? 'completed' : 'failed',
          progress: 1,
          endTime: new Date(),
        }));
      } catch {
        setExecutionState((prev) => ({ ...prev, status: 'completed', progress: 1 }));
      }
      es.close();
    });

    return () => {
      es.close();
    };
  }, [activeRunId]);

  const isConnected = isCloudMode() ? sseConnected : isLocalConnected;

  // Handle WebSocket messages
  useEffect(() => {
    if (lastMessage) {
      try {
        const data = JSON.parse(lastMessage.data);
        handleExecutionUpdate(data);
      } catch {
        // ignore parse errors
      }
    }
  }, [lastMessage]);

  const handleExecutionUpdate = useCallback((message: any) => {
    if (message.type === 'status_update') {
      setExecutionState((prev) => ({
        ...prev,
        ...message.data,
        steps: message.data?.steps ?? prev.steps ?? [],
        failedSteps: message.data?.failedSteps ?? prev.failedSteps ?? [],
      }));
    } else if (message.type === 'step_update') {
      setExecutionState((prev) => ({
        ...prev,
        steps: (prev.steps ?? []).map((step) =>
          step.id === message.data.stepId ? { ...step, ...message.data } : step
        ),
      }));
    } else if (message.type === 'metrics_update') {
      setExecutionState((prev) => ({
        ...prev,
        steps: (prev.steps ?? []).map((step) =>
          step.id === message.data.stepId
            ? { ...step, metrics: message.data.metrics }
            : step
        ),
      }));
    }
  }, []);

  const handleControl = useCallback(
    async (action: 'start' | 'stop' | 'pause' | 'resume' | 'skip') => {
      if (isCloudMode()) {
        if (action === 'start') {
          if (!selectedCloudFlowId) return;
          // Reset visual state so the SSE stream populates a fresh run.
          setExecutionState((prev) => ({
            ...prev,
            status: 'running',
            startTime: new Date(),
            endTime: undefined,
            progress: 0,
            steps: [],
            failedSteps: [],
          }));
          try {
            const run = await cloudFlowsApi.triggerRun(selectedCloudFlowId);
            setActiveRunId(run.id);
          } catch (err) {
            console.error('Failed to start cloud orchestration run', err);
            setExecutionState((prev) => ({ ...prev, status: 'failed' }));
          }
          return;
        }
        if (action === 'stop' && activeRunId) {
          try {
            await cloudTestRunsApi.cancelRun(activeRunId);
            setExecutionState((prev) => ({ ...prev, status: 'failed' }));
          } catch (err) {
            console.error('Failed to cancel cloud run', err);
          }
          return;
        }
        // Pause / resume / skip aren't supported by the cloud test_runs
        // lifecycle yet — silently no-op rather than firing local URLs.
        return;
      }
      await fetch(`/api/chaos/orchestration/${orchestrationId}/control`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action }),
      });
    },
    [orchestrationId, selectedCloudFlowId, activeRunId]
  );

  const getStepIcon = (status: ExecutionStep['status']) => {
    switch (status) {
      case 'completed':
        return <SuccessIcon color="success" />;
      case 'failed':
        return <ErrorIcon color="error" />;
      case 'running':
        return <SpeedIcon color="primary" />;
      case 'skipped':
        return <SkipIcon color="disabled" />;
      default:
        return <PendingIcon color="disabled" />;
    }
  };

  const getStatusColor = (status: ExecutionState['status']) => {
    switch (status) {
      case 'running':
        return 'primary';
      case 'completed':
        return 'success';
      case 'failed':
        return 'error';
      case 'paused':
        return 'warning';
      default:
        return 'default';
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      {isCloudMode() && (
        <Card sx={{ mb: 2 }}>
          <CardContent sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <Typography variant="body2" color="text.secondary">
              Orchestration:
            </Typography>
            <Select
              size="small"
              value={selectedCloudFlowId ?? ''}
              onChange={(e) => {
                const id = (e.target.value as string) || null;
                setSelectedCloudFlowId(id);
                const flow = cloudFlows.find((f) => f.id === id);
                if (flow) {
                  setExecutionState((prev) => ({
                    ...prev,
                    name: flow.name,
                    status: 'idle',
                    steps: [],
                    failedSteps: [],
                    progress: 0,
                  }));
                  setActiveRunId(null);
                }
              }}
              displayEmpty
              sx={{ minWidth: 240 }}
              disabled={cloudFlows.length === 0}
            >
              {cloudFlows.length === 0 ? (
                <MenuItem value="">No orchestrations in workspace</MenuItem>
              ) : (
                cloudFlows.map((f) => (
                  <MenuItem key={f.id} value={f.id}>
                    {f.name}
                  </MenuItem>
                ))
              )}
            </Select>
            {activeRunId && (
              <Typography variant="caption" color="text.secondary">
                Run <strong>{activeRunId}</strong>
              </Typography>
            )}
          </CardContent>
        </Card>
      )}
      {/* Header */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Grid container spacing={2} alignItems="center">
            <Grid item xs>
              <Typography variant="h5">{executionState.name}</Typography>
              <Box sx={{ display: 'flex', gap: 1, mt: 1 }}>
                <Chip
                  label={executionState.status}
                  color={getStatusColor(executionState.status) as any}
                  size="small"
                />
                <Chip
                  label={`Iteration ${executionState.currentIteration}/${executionState.maxIterations}`}
                  size="small"
                />
                <Chip
                  label={`Step ${executionState.currentStep}/${executionState.totalSteps}`}
                  size="small"
                />
                {!isConnected && (
                  <Chip label="Disconnected" color="error" size="small" />
                )}
              </Box>
            </Grid>

            {/* Control Buttons */}
            <Grid item>
              <Box sx={{ display: 'flex', gap: 1 }}>
                {executionState.status === 'idle' && (
                  <Button
                    variant="contained"
                    startIcon={<PlayIcon />}
                    onClick={() => handleControl('start')}
                  >
                    Start
                  </Button>
                )}
                {executionState.status === 'running' && (
                  <>
                    <Tooltip title="Pause">
                      <IconButton onClick={() => handleControl('pause')} color="warning">
                        <PauseIcon />
                      </IconButton>
                    </Tooltip>
                    <Tooltip title="Stop">
                      <IconButton onClick={() => handleControl('stop')} color="error">
                        <StopIcon />
                      </IconButton>
                    </Tooltip>
                    <Tooltip title="Skip Current Step">
                      <IconButton onClick={() => handleControl('skip')}>
                        <SkipIcon />
                      </IconButton>
                    </Tooltip>
                  </>
                )}
                {executionState.status === 'paused' && (
                  <Button
                    variant="contained"
                    startIcon={<PlayIcon />}
                    onClick={() => handleControl('resume')}
                  >
                    Resume
                  </Button>
                )}
              </Box>
            </Grid>
          </Grid>

          {/* Progress */}
          <Box sx={{ mt: 2 }}>
            <LinearProgress
              variant="determinate"
              value={executionState.progress * 100}
              sx={{ height: 8, borderRadius: 4 }}
            />
            <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5 }}>
              {Math.round(executionState.progress * 100)}% Complete
            </Typography>
          </Box>
        </CardContent>
      </Card>

      {/* Failed Steps Alert */}
      {executionState.failedSteps?.length > 0 && (
        <Alert severity="error" sx={{ mb: 3 }}>
          Failed Steps: {executionState.failedSteps.join(', ')}
        </Alert>
      )}

      {/* Steps Visualization */}
      <Card>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Execution Steps
          </Typography>

          <Stepper activeStep={executionState.currentStep} orientation="vertical">
            {(executionState.steps ?? []).map((step, index) => (
              <Step key={step.id} completed={step.status === 'completed'}>
                <StepLabel
                  icon={getStepIcon(step.status)}
                  error={step.status === 'failed'}
                >
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                    <Typography variant="subtitle1">{step.name}</Typography>
                    {step.duration && (
                      <Chip
                        label={`${step.duration}s`}
                        size="small"
                        variant="outlined"
                      />
                    )}
                  </Box>
                </StepLabel>

                <StepContent>
                  {/* Step Metrics */}
                  {step.metrics && (
                    <Grid container spacing={2} sx={{ mb: 2 }}>
                      <Grid item xs={4}>
                        <Card variant="outlined">
                          <CardContent sx={{ p: 2 }}>
                            <Typography variant="caption" color="text.secondary">
                              Requests
                            </Typography>
                            <Typography variant="h6">
                              {step.metrics.requestCount.toLocaleString()}
                            </Typography>
                          </CardContent>
                        </Card>
                      </Grid>
                      <Grid item xs={4}>
                        <Card variant="outlined">
                          <CardContent sx={{ p: 2 }}>
                            <Typography variant="caption" color="text.secondary">
                              Error Rate
                            </Typography>
                            <Typography variant="h6">
                              {(step.metrics.errorRate * 100).toFixed(2)}%
                            </Typography>
                          </CardContent>
                        </Card>
                      </Grid>
                      <Grid item xs={4}>
                        <Card variant="outlined">
                          <CardContent sx={{ p: 2 }}>
                            <Typography variant="caption" color="text.secondary">
                              Avg Latency
                            </Typography>
                            <Typography variant="h6">
                              {step.metrics.avgLatency.toFixed(0)}ms
                            </Typography>
                          </CardContent>
                        </Card>
                      </Grid>
                    </Grid>
                  )}

                  {/* Error Message */}
                  {step.error && (
                    <Alert severity="error" sx={{ mb: 2 }}>
                      {step.error}
                    </Alert>
                  )}

                  {/* Time Info */}
                  <Typography variant="caption" color="text.secondary">
                    {step.startTime &&
                      `Started: ${new Date(step.startTime).toLocaleTimeString()}`}
                    {step.endTime &&
                      ` | Ended: ${new Date(step.endTime).toLocaleTimeString()}`}
                  </Typography>
                </StepContent>
              </Step>
            ))}
          </Stepper>
        </CardContent>
      </Card>
    </Box>
  );
};

export default OrchestrationExecutionView;
