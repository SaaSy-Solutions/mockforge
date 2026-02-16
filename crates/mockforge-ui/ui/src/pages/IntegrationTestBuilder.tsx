import React, { useState } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Button,
  TextField,
  IconButton,
  Card,
  CardContent,
  CardActions,
  Grid,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Chip,
  Divider,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  Stepper,
  Step,
  StepLabel,
  StepContent,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  PlayArrow as PlayArrowIcon,
  Code as CodeIcon,
  ArrowDownward as ArrowDownwardIcon,
  Edit as EditIcon,
} from '@mui/icons-material';

interface WorkflowStep {
  id: string;
  name: string;
  description: string;
  request: {
    method: string;
    path: string;
    headers: { [key: string]: string };
    body?: string;
    query_params: { [key: string]: string };
  };
  validation: {
    status_code?: number;
    body_assertions: any[];
    header_assertions: any[];
    max_response_time_ms?: number;
  };
  extract: Array<{
    name: string;
    source: 'Body' | 'Header' | 'StatusCode';
    pattern: string;
    default?: string;
  }>;
  condition?: {
    variable: string;
    operator: string;
    value: string;
  };
  delay_ms?: number;
}

interface IntegrationWorkflow {
  id: string;
  name: string;
  description: string;
  steps: WorkflowStep[];
  setup: {
    variables: { [key: string]: string };
    base_url: string;
    headers: { [key: string]: string };
    timeout_ms: number;
  };
}

const IntegrationTestBuilder: React.FC = () => {
  const [workflow, setWorkflow] = useState<IntegrationWorkflow>({
    id: '',
    name: 'New Integration Test',
    description: '',
    steps: [],
    setup: {
      variables: {},
      base_url: 'http://localhost:3000',
      headers: {},
      timeout_ms: 30000,
    },
  });

  const [currentStep, setCurrentStep] = useState<WorkflowStep | null>(null);
  const [stepDialogOpen, setStepDialogOpen] = useState(false);
  const [generatedCode, setGeneratedCode] = useState('');
  const [codeDialogOpen, setCodeDialogOpen] = useState(false);
  const [selectedFormat, setSelectedFormat] = useState('rust');

  const httpMethods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

  const handleAddStep = () => {
    setCurrentStep({
      id: `step-${Date.now()}`,
      name: '',
      description: '',
      request: {
        method: 'GET',
        path: '/',
        headers: {},
        query_params: {},
      },
      validation: {
        body_assertions: [],
        header_assertions: [],
      },
      extract: [],
    });
    setStepDialogOpen(true);
  };

  const handleEditStep = (step: WorkflowStep) => {
    setCurrentStep(step);
    setStepDialogOpen(true);
  };

  const handleSaveStep = () => {
    if (!currentStep) return;

    const existingIndex = workflow.steps.findIndex((s) => s.id === currentStep.id);
    if (existingIndex >= 0) {
      const newSteps = [...workflow.steps];
      newSteps[existingIndex] = currentStep;
      setWorkflow({ ...workflow, steps: newSteps });
    } else {
      setWorkflow({ ...workflow, steps: [...workflow.steps, currentStep] });
    }

    setStepDialogOpen(false);
    setCurrentStep(null);
  };

  const handleDeleteStep = (stepId: string) => {
    setWorkflow({
      ...workflow,
      steps: workflow.steps.filter((s) => s.id !== stepId),
    });
  };

  const handleGenerateCode = async (format: string) => {
    setSelectedFormat(format);

    try {
      const response = await fetch(`/api/recorder/workflows/${workflow.id}/generate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          workflow,
          format,
        }),
      });

      const data = await response.json();
      if (data.success) {
        setGeneratedCode(data.test_code);
        setCodeDialogOpen(true);
      }
    } catch (error) {
      console.error('Failed to generate integration test:', error);
    }
  };

  const getLanguage = (format: string): string => {
    if (format === 'rust') return 'rust';
    if (format === 'python') return 'python';
    return 'javascript';
  };

  return (
    <Container maxWidth="xl">
      <Box sx={{ my: 4 }}>
        <Typography variant="h4" gutterBottom>
          <CodeIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          Integration Test Builder
        </Typography>
        <Typography variant="body1" color="text.secondary" paragraph>
          Build multi-step integration tests with state management and variable extraction
        </Typography>

        <Grid container spacing={3}>
          {/* Workflow Configuration */}
          <Grid item xs={12} md={4}>
            <Paper sx={{ p: 3 }}>
              <Typography variant="h6" gutterBottom>
                Workflow Configuration
              </Typography>

              <TextField
                fullWidth
                label="Workflow Name"
                value={workflow.name}
                onChange={(e) => setWorkflow({ ...workflow, name: e.target.value })}
                sx={{ mb: 2 }}
              />

              <TextField
                fullWidth
                label="Description"
                value={workflow.description}
                onChange={(e) => setWorkflow({ ...workflow, description: e.target.value })}
                multiline
                rows={2}
                sx={{ mb: 2 }}
              />

              <TextField
                fullWidth
                label="Base URL"
                value={workflow.setup.base_url}
                onChange={(e) =>
                  setWorkflow({
                    ...workflow,
                    setup: { ...workflow.setup, base_url: e.target.value },
                  })
                }
                sx={{ mb: 2 }}
              />

              <TextField
                fullWidth
                label="Timeout (ms)"
                type="number"
                value={workflow.setup.timeout_ms}
                onChange={(e) =>
                  setWorkflow({
                    ...workflow,
                    setup: { ...workflow.setup, timeout_ms: parseInt(e.target.value) },
                  })
                }
                sx={{ mb: 3 }}
              />

              <Divider sx={{ my: 2 }} />

              <Typography variant="subtitle2" gutterBottom>
                Generate Code
              </Typography>

              <Button
                fullWidth
                variant="outlined"
                onClick={() => handleGenerateCode('rust')}
                disabled={workflow.steps.length === 0}
                sx={{ mb: 1 }}
              >
                Rust
              </Button>

              <Button
                fullWidth
                variant="outlined"
                onClick={() => handleGenerateCode('python')}
                disabled={workflow.steps.length === 0}
                sx={{ mb: 1 }}
              >
                Python
              </Button>

              <Button
                fullWidth
                variant="outlined"
                onClick={() => handleGenerateCode('javascript')}
                disabled={workflow.steps.length === 0}
              >
                JavaScript
              </Button>
            </Paper>
          </Grid>

          {/* Steps Builder */}
          <Grid item xs={12} md={8}>
            <Paper sx={{ p: 3 }}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
                <Typography variant="h6">Test Steps ({workflow.steps.length})</Typography>
                <Button variant="contained" startIcon={<AddIcon />} onClick={handleAddStep}>
                  Add Step
                </Button>
              </Box>

              {workflow.steps.length === 0 ? (
                <Box sx={{ py: 8, textAlign: 'center' }}>
                  <Typography variant="body1" color="text.secondary">
                    No steps added yet
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    Click "Add Step" to create your first test step
                  </Typography>
                </Box>
              ) : (
                <Stepper orientation="vertical">
                  {workflow.steps.map((step, index) => (
                    <Step key={step.id} active>
                      <StepLabel>
                        {step.name || `Step ${index + 1}`}
                        <Chip
                          label={step.request.method}
                          size="small"
                          color="primary"
                          sx={{ ml: 1 }}
                        />
                      </StepLabel>
                      <StepContent>
                        <Card variant="outlined">
                          <CardContent>
                            <Typography variant="body2" color="text.secondary">
                              {step.description}
                            </Typography>
                            <Typography variant="body2" sx={{ mt: 1 }}>
                              <strong>Endpoint:</strong> {step.request.path}
                            </Typography>
                            {step.extract.length > 0 && (
                              <Box sx={{ mt: 1 }}>
                                <Typography variant="caption" color="text.secondary">
                                  Extracts: {step.extract.map((e) => e.name).join(', ')}
                                </Typography>
                              </Box>
                            )}
                          </CardContent>
                          <CardActions>
                            <IconButton size="small" onClick={() => handleEditStep(step)}>
                              <EditIcon />
                            </IconButton>
                            <IconButton size="small" onClick={() => handleDeleteStep(step.id)} color="error">
                              <DeleteIcon />
                            </IconButton>
                          </CardActions>
                        </Card>
                      </StepContent>
                    </Step>
                  ))}
                </Stepper>
              )}
            </Paper>
          </Grid>
        </Grid>
      </Box>

      {/* Step Editor Dialog */}
      <Dialog open={stepDialogOpen} onClose={() => setStepDialogOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>{currentStep?.name ? 'Edit Step' : 'Add Step'}</DialogTitle>
        <DialogContent>
          {currentStep && (
            <Box sx={{ pt: 2 }}>
              <TextField
                fullWidth
                label="Step Name"
                value={currentStep.name}
                onChange={(e) => setCurrentStep({ ...currentStep, name: e.target.value })}
                sx={{ mb: 2 }}
              />

              <TextField
                fullWidth
                label="Description"
                value={currentStep.description}
                onChange={(e) => setCurrentStep({ ...currentStep, description: e.target.value })}
                multiline
                rows={2}
                sx={{ mb: 2 }}
              />

              <Grid container spacing={2}>
                <Grid item xs={4}>
                  <FormControl fullWidth>
                    <InputLabel>Method</InputLabel>
                    <Select
                      value={currentStep.request.method}
                      onChange={(e) =>
                        setCurrentStep({
                          ...currentStep,
                          request: { ...currentStep.request, method: e.target.value },
                        })
                      }
                      label="Method"
                    >
                      {httpMethods.map((method) => (
                        <MenuItem key={method} value={method}>
                          {method}
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={8}>
                  <TextField
                    fullWidth
                    label="Path (use {variable} for substitution)"
                    value={currentStep.request.path}
                    onChange={(e) =>
                      setCurrentStep({
                        ...currentStep,
                        request: { ...currentStep.request, path: e.target.value },
                      })
                    }
                  />
                </Grid>
              </Grid>

              <TextField
                fullWidth
                label="Request Body (JSON)"
                value={currentStep.request.body || ''}
                onChange={(e) =>
                  setCurrentStep({
                    ...currentStep,
                    request: { ...currentStep.request, body: e.target.value },
                  })
                }
                multiline
                rows={4}
                sx={{ mt: 2 }}
              />

              <TextField
                fullWidth
                label="Expected Status Code"
                type="number"
                value={currentStep.validation.status_code || ''}
                onChange={(e) =>
                  setCurrentStep({
                    ...currentStep,
                    validation: {
                      ...currentStep.validation,
                      status_code: parseInt(e.target.value) || undefined,
                    },
                  })
                }
                sx={{ mt: 2 }}
              />

              <Typography variant="subtitle2" sx={{ mt: 2, mb: 1 }}>
                Extract Variables
              </Typography>
              <Button
                size="small"
                startIcon={<AddIcon />}
                onClick={() =>
                  setCurrentStep({
                    ...currentStep,
                    extract: [
                      ...currentStep.extract,
                      { name: '', source: 'Body', pattern: '', default: '' },
                    ],
                  })
                }
              >
                Add Extraction
              </Button>

              {currentStep.extract.map((extraction, idx) => (
                <Grid container spacing={1} key={idx} sx={{ mt: 1 }}>
                  <Grid item xs={3}>
                    <TextField
                      size="small"
                      fullWidth
                      label="Variable Name"
                      value={extraction.name}
                      onChange={(e) => {
                        const newExtract = [...currentStep.extract];
                        newExtract[idx].name = e.target.value;
                        setCurrentStep({ ...currentStep, extract: newExtract });
                      }}
                    />
                  </Grid>
                  <Grid item xs={3}>
                    <FormControl size="small" fullWidth>
                      <InputLabel>Source</InputLabel>
                      <Select
                        value={extraction.source}
                        onChange={(e) => {
                          const newExtract = [...currentStep.extract];
                          newExtract[idx].source = e.target.value as any;
                          setCurrentStep({ ...currentStep, extract: newExtract });
                        }}
                        label="Source"
                      >
                        <MenuItem value="Body">Body</MenuItem>
                        <MenuItem value="Header">Header</MenuItem>
                        <MenuItem value="StatusCode">Status Code</MenuItem>
                      </Select>
                    </FormControl>
                  </Grid>
                  <Grid item xs={4}>
                    <TextField
                      size="small"
                      fullWidth
                      label="JSONPath/Pattern"
                      value={extraction.pattern}
                      onChange={(e) => {
                        const newExtract = [...currentStep.extract];
                        newExtract[idx].pattern = e.target.value;
                        setCurrentStep({ ...currentStep, extract: newExtract });
                      }}
                    />
                  </Grid>
                  <Grid item xs={2}>
                    <IconButton
                      size="small"
                      onClick={() => {
                        const newExtract = currentStep.extract.filter((_, i) => i !== idx);
                        setCurrentStep({ ...currentStep, extract: newExtract });
                      }}
                    >
                      <DeleteIcon />
                    </IconButton>
                  </Grid>
                </Grid>
              ))}
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setStepDialogOpen(false)}>Cancel</Button>
          <Button variant="contained" onClick={handleSaveStep}>
            Save
          </Button>
        </DialogActions>
      </Dialog>

      {/* Generated Code Dialog */}
      <Dialog open={codeDialogOpen} onClose={() => setCodeDialogOpen(false)} maxWidth="lg" fullWidth>
        <DialogTitle>Generated Integration Test ({selectedFormat})</DialogTitle>
        <DialogContent>
          <Box
            component="pre"
            sx={{
              maxHeight: '520px',
              overflow: 'auto',
              p: 2,
              m: 0,
              borderRadius: 1,
              backgroundColor: '#0f172a',
              color: '#e2e8f0',
              fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
              fontSize: '0.8rem',
              lineHeight: 1.5,
            }}
          >
            <Box component="code">{generatedCode}</Box>
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCodeDialogOpen(false)}>Close</Button>
          <Button
            variant="contained"
            onClick={() => {
              const blob = new Blob([generatedCode], { type: 'text/plain' });
              const url = URL.createObjectURL(blob);
              const a = document.createElement('a');
              a.href = url;
              a.download = `integration_test.${getLanguage(selectedFormat)}`;
              a.click();
              URL.revokeObjectURL(url);
            }}
          >
            Download
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
};

export default IntegrationTestBuilder;
