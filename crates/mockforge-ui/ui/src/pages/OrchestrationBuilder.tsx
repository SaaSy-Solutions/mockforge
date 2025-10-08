/**
 * Visual Orchestration Builder
 *
 * Provides a drag-and-drop interface for building chaos orchestrations
 * with conditional logic, variables, hooks, and assertions.
 */

import React, { useState, useCallback } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Drawer,
  Grid,
  IconButton,
  List,
  ListItem,
  ListItemText,
  TextField,
  Typography,
  Select,
  MenuItem,
  Chip,
  Tabs,
  Tab,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  PlayArrow as PlayIcon,
  Save as SaveIcon,
  Download as DownloadIcon,
  Upload as UploadIcon,
} from '@mui/icons-material';

// Type definitions
interface Variable {
  name: string;
  value: any;
}

interface Hook {
  name: string;
  hookType: 'pre_step' | 'post_step' | 'pre_orchestration' | 'post_orchestration';
  actions: HookAction[];
  condition?: Condition;
}

interface HookAction {
  type: 'set_variable' | 'log' | 'http_request' | 'command' | 'record_metric';
  [key: string]: any;
}

interface Condition {
  type: 'equals' | 'not_equals' | 'greater_than' | 'less_than' | 'exists' | 'and' | 'or' | 'not' | 'metric_threshold';
  [key: string]: any;
}

interface Assertion {
  type: 'variable_equals' | 'metric_in_range' | 'step_succeeded' | 'step_failed' | 'condition';
  [key: string]: any;
}

interface Step {
  id: string;
  name: string;
  scenario: string;
  duration_seconds?: number;
  condition?: Condition;
  preHooks: Hook[];
  postHooks: Hook[];
  assertions: Assertion[];
  variables: Record<string, any>;
}

interface ConditionalStep {
  id: string;
  name: string;
  condition: Condition;
  thenSteps: Step[];
  elseSteps: Step[];
}

interface Orchestration {
  name: string;
  description: string;
  variables: Variable[];
  hooks: Hook[];
  steps: Step[];
  conditionalSteps: ConditionalStep[];
  assertions: Assertion[];
  enableReporting: boolean;
}

export const OrchestrationBuilder: React.FC = () => {
  const [orchestration, setOrchestration] = useState<Orchestration>({
    name: 'New Orchestration',
    description: '',
    variables: [],
    hooks: [],
    steps: [],
    conditionalSteps: [],
    assertions: [],
    enableReporting: true,
  });

  const [selectedStep, setSelectedStep] = useState<Step | null>(null);
  const [propertyPanelOpen, setPropertyPanelOpen] = useState(false);
  const [currentTab, setCurrentTab] = useState(0);

  // Step management
  const addStep = useCallback(() => {
    const newStep: Step = {
      id: `step-${Date.now()}`,
      name: `Step ${orchestration.steps.length + 1}`,
      scenario: 'network_degradation',
      preHooks: [],
      postHooks: [],
      assertions: [],
      variables: {},
    };
    setOrchestration((prev) => ({
      ...prev,
      steps: [...prev.steps, newStep],
    }));
  }, [orchestration.steps.length]);

  const deleteStep = useCallback((stepId: string) => {
    setOrchestration((prev) => ({
      ...prev,
      steps: prev.steps.filter((s) => s.id !== stepId),
    }));
  }, []);

  const updateStep = useCallback((step: Step) => {
    setOrchestration((prev) => ({
      ...prev,
      steps: prev.steps.map((s) => (s.id === step.id ? step : s)),
    }));
  }, []);

  // Variable management
  const addVariable = useCallback(() => {
    const newVar: Variable = {
      name: `var_${orchestration.variables.length + 1}`,
      value: '',
    };
    setOrchestration((prev) => ({
      ...prev,
      variables: [...prev.variables, newVar],
    }));
  }, [orchestration.variables.length]);

  const deleteVariable = useCallback((index: number) => {
    setOrchestration((prev) => ({
      ...prev,
      variables: prev.variables.filter((_, i) => i !== index),
    }));
  }, []);

  // Hook management
  const addHook = useCallback(() => {
    const newHook: Hook = {
      name: `hook_${orchestration.hooks.length + 1}`,
      hookType: 'pre_step',
      actions: [],
    };
    setOrchestration((prev) => ({
      ...prev,
      hooks: [...prev.hooks, newHook],
    }));
  }, [orchestration.hooks.length]);

  // Assertion management
  const addAssertion = useCallback(() => {
    const newAssertion: Assertion = {
      type: 'variable_equals',
      variable: '',
      expected: '',
    };
    setOrchestration((prev) => ({
      ...prev,
      assertions: [...prev.assertions, newAssertion],
    }));
  }, []);

  // Export/Import
  const exportOrchestration = useCallback(() => {
    const json = JSON.stringify(orchestration, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${orchestration.name.replace(/\s+/g, '-').toLowerCase()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [orchestration]);

  const importOrchestration = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const imported = JSON.parse(e.target?.result as string);
          setOrchestration(imported);
        } catch (error) {
          alert('Failed to import orchestration');
        }
      };
      reader.readAsText(file);
    }
  }, []);

  // Execute orchestration
  const executeOrchestration = useCallback(async () => {
    try {
      const response = await fetch('/api/chaos/orchestration/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(orchestration),
      });
      if (response.ok) {
        alert('Orchestration started successfully!');
      } else {
        alert('Failed to start orchestration');
      }
    } catch (error) {
      alert('Error executing orchestration');
    }
  }, [orchestration]);

  return (
    <Box sx={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Toolbar */}
      <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper' }}>
        <Grid container spacing={2} alignItems="center">
          <Grid item xs>
            <TextField
              fullWidth
              value={orchestration.name}
              onChange={(e) => setOrchestration((prev) => ({ ...prev, name: e.target.value }))}
              placeholder="Orchestration Name"
              variant="outlined"
              size="small"
            />
          </Grid>
          <Grid item>
            <Button
              startIcon={<PlayIcon />}
              variant="contained"
              color="primary"
              onClick={executeOrchestration}
            >
              Execute
            </Button>
          </Grid>
          <Grid item>
            <Button startIcon={<SaveIcon />} onClick={() => alert('Save functionality')}>
              Save
            </Button>
          </Grid>
          <Grid item>
            <Button startIcon={<DownloadIcon />} onClick={exportOrchestration}>
              Export
            </Button>
          </Grid>
          <Grid item>
            <input
              type="file"
              accept=".json"
              style={{ display: 'none' }}
              id="import-file"
              onChange={importOrchestration}
            />
            <label htmlFor="import-file">
              <Button startIcon={<UploadIcon />} component="span">
                Import
              </Button>
            </label>
          </Grid>
        </Grid>
      </Box>

      <Grid container sx={{ flexGrow: 1, overflow: 'hidden' }}>
        {/* Left Panel - Configuration */}
        <Grid item xs={3} sx={{ borderRight: 1, borderColor: 'divider', overflow: 'auto' }}>
          <Box sx={{ p: 2 }}>
            <Tabs value={currentTab} onChange={(_, v) => setCurrentTab(v)}>
              <Tab label="Variables" />
              <Tab label="Hooks" />
              <Tab label="Assertions" />
            </Tabs>

            {/* Variables Tab */}
            {currentTab === 0 && (
              <Box sx={{ mt: 2 }}>
                <Button
                  fullWidth
                  startIcon={<AddIcon />}
                  variant="outlined"
                  onClick={addVariable}
                  sx={{ mb: 2 }}
                >
                  Add Variable
                </Button>
                <List>
                  {orchestration.variables.map((v, index) => (
                    <ListItem
                      key={index}
                      secondaryAction={
                        <IconButton edge="end" onClick={() => deleteVariable(index)}>
                          <DeleteIcon />
                        </IconButton>
                      }
                    >
                      <ListItemText
                        primary={v.name}
                        secondary={typeof v.value === 'object' ? JSON.stringify(v.value) : v.value}
                      />
                    </ListItem>
                  ))}
                </List>
              </Box>
            )}

            {/* Hooks Tab */}
            {currentTab === 1 && (
              <Box sx={{ mt: 2 }}>
                <Button
                  fullWidth
                  startIcon={<AddIcon />}
                  variant="outlined"
                  onClick={addHook}
                  sx={{ mb: 2 }}
                >
                  Add Hook
                </Button>
                <List>
                  {orchestration.hooks.map((hook, index) => (
                    <ListItem key={index}>
                      <ListItemText
                        primary={hook.name}
                        secondary={
                          <Box sx={{ display: 'flex', gap: 1, mt: 1 }}>
                            <Chip label={hook.hookType} size="small" />
                            <Chip label={`${hook.actions.length} actions`} size="small" />
                          </Box>
                        }
                      />
                    </ListItem>
                  ))}
                </List>
              </Box>
            )}

            {/* Assertions Tab */}
            {currentTab === 2 && (
              <Box sx={{ mt: 2 }}>
                <Button
                  fullWidth
                  startIcon={<AddIcon />}
                  variant="outlined"
                  onClick={addAssertion}
                  sx={{ mb: 2 }}
                >
                  Add Assertion
                </Button>
                <List>
                  {orchestration.assertions.map((assertion, index) => (
                    <ListItem key={index}>
                      <ListItemText primary={assertion.type} />
                    </ListItem>
                  ))}
                </List>
              </Box>
            )}
          </Box>
        </Grid>

        {/* Center Panel - Canvas */}
        <Grid item xs={6} sx={{ overflow: 'auto', bgcolor: '#f5f5f5' }}>
          <Box sx={{ p: 3 }}>
            <Box sx={{ mb: 2 }}>
              <Button
                fullWidth
                startIcon={<AddIcon />}
                variant="contained"
                onClick={addStep}
              >
                Add Step
              </Button>
            </Box>

            {orchestration.steps.map((step, index) => (
              <Card
                key={step.id}
                sx={{
                  mb: 2,
                  cursor: 'pointer',
                  '&:hover': { boxShadow: 3 },
                }}
                onClick={() => {
                  setSelectedStep(step);
                  setPropertyPanelOpen(true);
                }}
              >
                <CardContent>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                    <Box>
                      <Typography variant="h6">{step.name}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        Scenario: {step.scenario}
                      </Typography>
                      {step.duration_seconds && (
                        <Typography variant="body2" color="text.secondary">
                          Duration: {step.duration_seconds}s
                        </Typography>
                      )}
                      <Box sx={{ mt: 1, display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                        {step.condition && <Chip label="Conditional" size="small" color="primary" />}
                        {step.preHooks.length > 0 && (
                          <Chip label={`${step.preHooks.length} Pre-Hooks`} size="small" />
                        )}
                        {step.postHooks.length > 0 && (
                          <Chip label={`${step.postHooks.length} Post-Hooks`} size="small" />
                        )}
                        {step.assertions.length > 0 && (
                          <Chip
                            label={`${step.assertions.length} Assertions`}
                            size="small"
                            color="secondary"
                          />
                        )}
                      </Box>
                    </Box>
                    <IconButton onClick={(e) => {
                      e.stopPropagation();
                      deleteStep(step.id);
                    }}>
                      <DeleteIcon />
                    </IconButton>
                  </Box>
                </CardContent>
              </Card>
            ))}

            {orchestration.steps.length === 0 && (
              <Box sx={{ textAlign: 'center', py: 8 }}>
                <Typography variant="h6" color="text.secondary">
                  No steps added yet
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Click "Add Step" to start building your orchestration
                </Typography>
              </Box>
            )}
          </Box>
        </Grid>

        {/* Right Panel - Step Properties */}
        <Drawer
          anchor="right"
          open={propertyPanelOpen}
          onClose={() => setPropertyPanelOpen(false)}
          sx={{ '& .MuiDrawer-paper': { width: 400 } }}
        >
          {selectedStep && (
            <Box sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Step Properties
              </Typography>

              <TextField
                fullWidth
                label="Step Name"
                value={selectedStep.name}
                onChange={(e) => updateStep({ ...selectedStep, name: e.target.value })}
                sx={{ mb: 2 }}
              />

              <Select
                fullWidth
                value={selectedStep.scenario}
                onChange={(e) => updateStep({ ...selectedStep, scenario: e.target.value })}
                sx={{ mb: 2 }}
              >
                <MenuItem value="network_degradation">Network Degradation</MenuItem>
                <MenuItem value="service_instability">Service Instability</MenuItem>
                <MenuItem value="cascading_failure">Cascading Failure</MenuItem>
                <MenuItem value="peak_traffic">Peak Traffic</MenuItem>
                <MenuItem value="slow_backend">Slow Backend</MenuItem>
              </Select>

              <TextField
                fullWidth
                type="number"
                label="Duration (seconds)"
                value={selectedStep.duration_seconds || ''}
                onChange={(e) =>
                  updateStep({
                    ...selectedStep,
                    duration_seconds: parseInt(e.target.value) || undefined,
                  })
                }
                sx={{ mb: 2 }}
              />

              <Typography variant="subtitle1" sx={{ mt: 3, mb: 1 }}>
                Assertions ({selectedStep.assertions.length})
              </Typography>
              <Button
                fullWidth
                variant="outlined"
                size="small"
                onClick={() => {
                  updateStep({
                    ...selectedStep,
                    assertions: [
                      ...selectedStep.assertions,
                      { type: 'variable_equals', variable: '', expected: '' },
                    ],
                  });
                }}
              >
                Add Assertion
              </Button>

              <Typography variant="subtitle1" sx={{ mt: 3, mb: 1 }}>
                Pre-Hooks ({selectedStep.preHooks.length})
              </Typography>
              <Button fullWidth variant="outlined" size="small">
                Add Pre-Hook
              </Button>

              <Typography variant="subtitle1" sx={{ mt: 3, mb: 1 }}>
                Post-Hooks ({selectedStep.postHooks.length})
              </Typography>
              <Button fullWidth variant="outlined" size="small">
                Add Post-Hook
              </Button>
            </Box>
          )}
        </Drawer>
      </Grid>
    </Box>
  );
};

export default OrchestrationBuilder;
