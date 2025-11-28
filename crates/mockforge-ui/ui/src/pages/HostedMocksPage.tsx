/**
 * Hosted Mocks Deployment Page
 *
 * Manage cloud-hosted mock service deployments
 */

import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  CardActions,
  Grid,
  Typography,
  Button,
  Chip,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Alert,
  LinearProgress,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Tabs,
  Tab,
  List,
  ListItem,
  ListItemText,
  Divider,
  Tooltip,
  CircularProgress,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Refresh as RefreshIcon,
  OpenInNew as OpenInNewIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
  Pending as PendingIcon,
  CloudUpload as CloudUploadIcon,
  Stop as StopIcon,
  Visibility as ViewIcon,
  Code as CodeIcon,
  Assessment as AssessmentIcon,
} from '@mui/icons-material';

interface Deployment {
  id: string;
  org_id: string;
  project_id?: string;
  name: string;
  slug: string;
  description?: string;
  status: 'pending' | 'deploying' | 'active' | 'stopped' | 'failed' | 'deleting';
  deployment_url?: string;
  health_status: 'healthy' | 'unhealthy' | 'unknown';
  error_message?: string;
  created_at: string;
  updated_at: string;
}

interface DeploymentLog {
  id: string;
  level: 'info' | 'warning' | 'error' | 'debug';
  message: string;
  metadata: Record<string, any>;
  created_at: string;
}

interface DeploymentMetrics {
  requests: number;
  requests_2xx: number;
  requests_4xx: number;
  requests_5xx: number;
  egress_bytes: number;
  avg_response_time_ms: number;
  period_start: string;
}

export const HostedMocksPage: React.FC = () => {
  const [deployments, setDeployments] = useState<Deployment[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [selectedDeployment, setSelectedDeployment] = useState<Deployment | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [detailsTab, setDetailsTab] = useState(0);
  const [logs, setLogs] = useState<DeploymentLog[]>([]);
  const [metrics, setMetrics] = useState<DeploymentMetrics | null>(null);
  const [logsLoading, setLogsLoading] = useState(false);
  const [metricsLoading, setMetricsLoading] = useState(false);

  // Create deployment form state
  const [formData, setFormData] = useState({
    name: '',
    slug: '',
    description: '',
    config_json: '{}',
    openapi_spec_url: '',
  });
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    loadDeployments();
  }, []);

  const loadDeployments = async () => {
    setLoading(true);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch('/api/v1/deployments', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error('Failed to load deployments');
      }

      const data = await response.json();
      setDeployments(data.deployments || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load deployments');
    } finally {
      setLoading(false);
    }
  };

  const handleCreateDeployment = async () => {
    setCreating(true);
    setError(null);

    try {
      // Validate form
      if (!formData.name || !formData.config_json) {
        throw new Error('Name and config are required');
      }

      // Validate JSON
      let configJson;
      try {
        configJson = JSON.parse(formData.config_json);
      } catch {
        throw new Error('Invalid JSON in config field');
      }

      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const request = {
        name: formData.name,
        slug: formData.slug || undefined,
        description: formData.description || undefined,
        config_json: configJson,
        openapi_spec_url: formData.openapi_spec_url || undefined,
      };

      const response = await fetch('/api/v1/deployments', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(errorData.error || 'Failed to create deployment');
      }

      // Success
      setCreateModalOpen(false);
      setFormData({
        name: '',
        slug: '',
        description: '',
        config_json: '{}',
        openapi_spec_url: '',
      });
      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create deployment');
    } finally {
      setCreating(false);
    }
  };

  const handleDeleteDeployment = async (id: string) => {
    if (!confirm('Are you sure you want to delete this deployment?')) {
      return;
    }

    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch(`/api/v1/deployments/${id}`, {
        method: 'DELETE',
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error('Failed to delete deployment');
      }

      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete deployment');
    }
  };

  const handleViewDetails = async (deployment: Deployment) => {
    setSelectedDeployment(deployment);
    setDetailsOpen(true);
    setDetailsTab(0);
    setLogs([]);
    setMetrics(null);

    // Load logs and metrics
    await Promise.all([
      loadDeploymentLogs(deployment.id),
      loadDeploymentMetrics(deployment.id),
    ]);
  };

  const loadDeploymentLogs = async (id: string) => {
    setLogsLoading(true);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        return;
      }

      const response = await fetch(`/api/v1/deployments/${id}/logs`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        setLogs(data || []);
      }
    } catch (err) {
      console.error('Failed to load logs:', err);
    } finally {
      setLogsLoading(false);
    }
  };

  const loadDeploymentMetrics = async (id: string) => {
    setMetricsLoading(true);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        return;
      }

      const response = await fetch(`/api/v1/deployments/${id}/metrics`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        setMetrics(data);
      }
    } catch (err) {
      console.error('Failed to load metrics:', err);
    } finally {
      setMetricsLoading(false);
    }
  };

  const getStatusColor = (status: Deployment['status']) => {
    switch (status) {
      case 'active':
        return 'success';
      case 'failed':
        return 'error';
      case 'deploying':
      case 'pending':
        return 'warning';
      case 'stopped':
        return 'default';
      default:
        return 'default';
    }
  };

  const getStatusIcon = (status: Deployment['status']) => {
    switch (status) {
      case 'active':
        return <CheckCircleIcon />;
      case 'failed':
        return <ErrorIcon />;
      case 'deploying':
      case 'pending':
        return <PendingIcon />;
      default:
        return <StopIcon />;
    }
  };

  const getHealthColor = (health: Deployment['health_status']) => {
    switch (health) {
      case 'healthy':
        return 'success';
      case 'unhealthy':
        return 'error';
      default:
        return 'default';
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Box sx={{ mb: 4, display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
        <Box>
          <Typography variant="h4" gutterBottom>
            Hosted Mocks
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Deploy and manage cloud-hosted mock services
          </Typography>
        </Box>
        <Box sx={{ display: 'flex', gap: 2 }}>
          <Button
            variant="outlined"
            startIcon={<RefreshIcon />}
            onClick={loadDeployments}
            disabled={loading}
          >
            Refresh
          </Button>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={() => setCreateModalOpen(true)}
          >
            Deploy Mock
          </Button>
        </Box>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 3 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {loading && <LinearProgress sx={{ mb: 3 }} />}

      {/* Deployments Table */}
      <Card>
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Name</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Health</TableCell>
                <TableCell>URL</TableCell>
                <TableCell>Created</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {deployments.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4 }}>
                    <Typography variant="body2" color="text.secondary">
                      No deployments yet. Create your first deployment to get started.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                deployments.map((deployment) => (
                  <TableRow key={deployment.id}>
                    <TableCell>
                      <Box>
                        <Typography variant="subtitle2">{deployment.name}</Typography>
                        {deployment.description && (
                          <Typography variant="caption" color="text.secondary">
                            {deployment.description}
                          </Typography>
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Chip
                        icon={getStatusIcon(deployment.status)}
                        label={deployment.status}
                        color={getStatusColor(deployment.status) as any}
                        size="small"
                      />
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={deployment.health_status}
                        color={getHealthColor(deployment.health_status) as any}
                        size="small"
                      />
                    </TableCell>
                    <TableCell>
                      {deployment.deployment_url ? (
                        <Button
                          size="small"
                          startIcon={<OpenInNewIcon />}
                          href={deployment.deployment_url}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          Open
                        </Button>
                      ) : (
                        <Typography variant="caption" color="text.secondary">
                          Not available
                        </Typography>
                      )}
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption">
                        {new Date(deployment.created_at).toLocaleDateString()}
                      </Typography>
                    </TableCell>
                    <TableCell align="right">
                      <Tooltip title="View Details">
                        <IconButton
                          size="small"
                          onClick={() => handleViewDetails(deployment)}
                        >
                          <ViewIcon />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title="Delete">
                        <IconButton
                          size="small"
                          color="error"
                          onClick={() => handleDeleteDeployment(deployment.id)}
                        >
                          <DeleteIcon />
                        </IconButton>
                      </Tooltip>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Card>

      {/* Create Deployment Modal */}
      <Dialog open={createModalOpen} onClose={() => setCreateModalOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>Deploy New Mock Service</DialogTitle>
        <DialogContent>
          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}
          {creating && <LinearProgress sx={{ mb: 2 }} />}

          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
            <TextField
              label="Name"
              required
              fullWidth
              value={formData.name}
              onChange={(e) => {
                setFormData({ ...formData, name: e.target.value });
                // Auto-generate slug
                if (!formData.slug) {
                  const slug = e.target.value
                    .toLowerCase()
                    .replace(/[^a-z0-9]+/g, '-')
                    .replace(/^-+|-+$/g, '');
                  setFormData((prev) => ({ ...prev, slug }));
                }
              }}
              placeholder="My Mock Service"
            />

            <TextField
              label="Slug"
              fullWidth
              value={formData.slug}
              onChange={(e) => setFormData({ ...formData, slug: e.target.value })}
              placeholder="my-mock-service"
              helperText="URL-friendly identifier (auto-generated from name)"
            />

            <TextField
              label="Description"
              fullWidth
              multiline
              rows={2}
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              placeholder="Description of the mock service"
            />

            <TextField
              label="OpenAPI Spec URL (optional)"
              fullWidth
              value={formData.openapi_spec_url}
              onChange={(e) => setFormData({ ...formData, openapi_spec_url: e.target.value })}
              placeholder="https://example.com/openapi.json"
            />

            <TextField
              label="Configuration (JSON)"
              required
              fullWidth
              multiline
              rows={8}
              value={formData.config_json}
              onChange={(e) => setFormData({ ...formData, config_json: e.target.value })}
              placeholder='{"services": [], "plugins": []}'
              helperText="MockForge configuration in JSON format"
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCreateModalOpen(false)} disabled={creating}>
            Cancel
          </Button>
          <Button
            variant="contained"
            onClick={handleCreateDeployment}
            disabled={creating || !formData.name || !formData.config_json}
            startIcon={<CloudUploadIcon />}
          >
            {creating ? 'Deploying...' : 'Deploy'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Deployment Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="lg"
        fullWidth
      >
        {selectedDeployment && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Box>
                  <Typography variant="h6">{selectedDeployment.name}</Typography>
                  <Typography variant="caption" color="text.secondary">
                    {selectedDeployment.slug}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', gap: 1 }}>
                  <Chip
                    icon={getStatusIcon(selectedDeployment.status)}
                    label={selectedDeployment.status}
                    color={getStatusColor(selectedDeployment.status) as any}
                  />
                  <Chip
                    label={selectedDeployment.health_status}
                    color={getHealthColor(selectedDeployment.health_status) as any}
                  />
                </Box>
              </Box>
            </DialogTitle>

            <DialogContent>
              <Tabs value={detailsTab} onChange={(_, v) => setDetailsTab(v)} sx={{ mb: 3 }}>
                <Tab icon={<CodeIcon />} label="Overview" />
                <Tab icon={<ViewIcon />} label="Logs" />
                <Tab icon={<AssessmentIcon />} label="Metrics" />
              </Tabs>

              {detailsTab === 0 && (
                <Box>
                  {selectedDeployment.description && (
                    <Typography variant="body1" paragraph>
                      {selectedDeployment.description}
                    </Typography>
                  )}

                  {selectedDeployment.error_message && (
                    <Alert severity="error" sx={{ mb: 2 }}>
                      {selectedDeployment.error_message}
                    </Alert>
                  )}

                  <Grid container spacing={2}>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Deployment URL
                      </Typography>
                      <Typography variant="body2">
                        {selectedDeployment.deployment_url ? (
                          <Button
                            size="small"
                            startIcon={<OpenInNewIcon />}
                            href={selectedDeployment.deployment_url}
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {selectedDeployment.deployment_url}
                          </Button>
                        ) : (
                          'Not available'
                        )}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Created
                      </Typography>
                      <Typography variant="body2">
                        {new Date(selectedDeployment.created_at).toLocaleString()}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Last Updated
                      </Typography>
                      <Typography variant="body2">
                        {new Date(selectedDeployment.updated_at).toLocaleString()}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        ID
                      </Typography>
                      <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                        {selectedDeployment.id}
                      </Typography>
                    </Grid>
                  </Grid>
                </Box>
              )}

              {detailsTab === 1 && (
                <Box>
                  {logsLoading ? (
                    <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
                      <CircularProgress />
                    </Box>
                  ) : logs.length === 0 ? (
                    <Alert severity="info">No logs available</Alert>
                  ) : (
                    <List>
                      {logs.map((log, index) => (
                        <React.Fragment key={log.id}>
                          <ListItem>
                            <ListItemText
                              primary={
                                <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                                  <Chip
                                    label={log.level}
                                    size="small"
                                    color={
                                      log.level === 'error'
                                        ? 'error'
                                        : log.level === 'warning'
                                        ? 'warning'
                                        : 'default'
                                    }
                                  />
                                  <Typography variant="body2">{log.message}</Typography>
                                </Box>
                              }
                              secondary={new Date(log.created_at).toLocaleString()}
                            />
                          </ListItem>
                          {index < logs.length - 1 && <Divider />}
                        </React.Fragment>
                      ))}
                    </List>
                  )}
                </Box>
              )}

              {detailsTab === 2 && (
                <Box>
                  {metricsLoading ? (
                    <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
                      <CircularProgress />
                    </Box>
                  ) : !metrics ? (
                    <Alert severity="info">No metrics available</Alert>
                  ) : (
                    <Grid container spacing={2}>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            Total Requests
                          </Typography>
                          <Typography variant="h6">{metrics.requests.toLocaleString()}</Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            2xx Responses
                          </Typography>
                          <Typography variant="h6" color="success.main">
                            {metrics.requests_2xx.toLocaleString()}
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            4xx Responses
                          </Typography>
                          <Typography variant="h6" color="warning.main">
                            {metrics.requests_4xx.toLocaleString()}
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            5xx Responses
                          </Typography>
                          <Typography variant="h6" color="error.main">
                            {metrics.requests_5xx.toLocaleString()}
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            Egress (bytes)
                          </Typography>
                          <Typography variant="h6">
                            {(metrics.egress_bytes / 1024 / 1024).toFixed(2)} MB
                          </Typography>
                        </Paper>
                      </Grid>
                      <Grid item xs={6} md={3}>
                        <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                          <Typography variant="caption" color="text.secondary">
                            Avg Response Time
                          </Typography>
                          <Typography variant="h6">{metrics.avg_response_time_ms} ms</Typography>
                        </Paper>
                      </Grid>
                    </Grid>
                  )}
                </Box>
              )}
            </DialogContent>

            <DialogActions>
              <Button onClick={() => setDetailsOpen(false)}>Close</Button>
              {selectedDeployment.deployment_url && (
                <Button
                  variant="contained"
                  startIcon={<OpenInNewIcon />}
                  href={selectedDeployment.deployment_url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Open Deployment
                </Button>
              )}
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};
