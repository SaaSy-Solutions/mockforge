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

  const { messages, sendMessage, isConnected } = useWebSocket(
    `/api/chaos/orchestration/${orchestrationId}/ws`
  );

  // Handle WebSocket messages
  useEffect(() => {
    if (messages.length > 0) {
      const latestMessage = messages[messages.length - 1];
      handleExecutionUpdate(latestMessage);
    }
  }, [messages]);

  const handleExecutionUpdate = useCallback((message: any) => {
    if (message.type === 'status_update') {
      setExecutionState((prev) => ({
        ...prev,
        ...message.data,
      }));
    } else if (message.type === 'step_update') {
      setExecutionState((prev) => ({
        ...prev,
        steps: prev.steps.map((step) =>
          step.id === message.data.stepId ? { ...step, ...message.data } : step
        ),
      }));
    } else if (message.type === 'metrics_update') {
      setExecutionState((prev) => ({
        ...prev,
        steps: prev.steps.map((step) =>
          step.id === message.data.stepId
            ? { ...step, metrics: message.data.metrics }
            : step
        ),
      }));
    }
  }, []);

  const handleControl = useCallback(
    async (action: 'start' | 'stop' | 'pause' | 'resume' | 'skip') => {
      await fetch(`/api/chaos/orchestration/${orchestrationId}/control`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action }),
      });
    },
    [orchestrationId]
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
      {executionState.failedSteps.length > 0 && (
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
            {executionState.steps.map((step, index) => (
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
