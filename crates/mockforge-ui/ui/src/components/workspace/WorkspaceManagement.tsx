import React, { useState, useEffect } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  CardHeader,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Grid,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
  Alert,
  CircularProgress,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  Refresh as RefreshIcon,
  ToggleOn as EnableIcon,
  ToggleOff as DisableIcon,
} from '@mui/icons-material';

interface Workspace {
  id: string;
  name: string;
  description?: string;
  enabled: boolean;
  stats: {
    total_requests: number;
    active_routes: number;
    avg_response_time_ms: number;
    last_request_at?: string;
  };
  created_at: string;
  updated_at: string;
}

interface WorkspaceFormData {
  id: string;
  name: string;
  description?: string;
}

const WorkspaceManagement: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [selectedWorkspace, setSelectedWorkspace] = useState<Workspace | null>(null);
  const [formData, setFormData] = useState<WorkspaceFormData>({
    id: '',
    name: '',
    description: '',
  });

  // Fetch workspaces
  const fetchWorkspaces = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch('/__mockforge/workspaces');
      if (!response.ok) {
        throw new Error(`Failed to fetch workspaces: ${response.statusText}`);
      }
      const result = await response.json();
      setWorkspaces(result.data || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  // Create workspace
  const handleCreate = async () => {
    try {
      const response = await fetch('/__mockforge/workspaces', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(formData),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to create workspace');
      }

      setCreateDialogOpen(false);
      setFormData({ id: '', name: '', description: '' });
      fetchWorkspaces();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  // Update workspace
  const handleUpdate = async () => {
    if (!selectedWorkspace) return;

    try {
      const response = await fetch(`/__mockforge/workspaces/${selectedWorkspace.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: formData.name,
          description: formData.description,
        }),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to update workspace');
      }

      setEditDialogOpen(false);
      setSelectedWorkspace(null);
      setFormData({ id: '', name: '', description: '' });
      fetchWorkspaces();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  // Delete workspace
  const handleDelete = async (workspaceId: string) => {
    if (!confirm(`Are you sure you want to delete workspace "${workspaceId}"?`)) {
      return;
    }

    try {
      const response = await fetch(`/__mockforge/workspaces/${workspaceId}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to delete workspace');
      }

      fetchWorkspaces();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  // Toggle workspace enabled state
  const handleToggleEnabled = async (workspace: Workspace) => {
    try {
      const response = await fetch(`/__mockforge/workspaces/${workspace.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: !workspace.enabled }),
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to update workspace');
      }

      fetchWorkspaces();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  // Open edit dialog
  const handleOpenEdit = (workspace: Workspace) => {
    setSelectedWorkspace(workspace);
    setFormData({
      id: workspace.id,
      name: workspace.name,
      description: workspace.description || '',
    });
    setEditDialogOpen(true);
  };

  useEffect(() => {
    fetchWorkspaces();
  }, []);

  if (loading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box>
      {/* Header */}
      <Box display="flex" justifyContent="space-between" alignItems="center" mb={3}>
        <Typography variant="h4">Workspace Management</Typography>
        <Box>
          <IconButton onClick={fetchWorkspaces} title="Refresh">
            <RefreshIcon />
          </IconButton>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={() => setCreateDialogOpen(true)}
          >
            Create Workspace
          </Button>
        </Box>
      </Box>

      {/* Error Alert */}
      {error && (
        <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      {/* Workspaces Table */}
      <TableContainer component={Paper}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>ID</TableCell>
              <TableCell>Name</TableCell>
              <TableCell>Status</TableCell>
              <TableCell align="right">Requests</TableCell>
              <TableCell align="right">Routes</TableCell>
              <TableCell align="right">Avg Response Time</TableCell>
              <TableCell align="right">Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {workspaces.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} align="center">
                  No workspaces found. Create one to get started!
                </TableCell>
              </TableRow>
            ) : (
              workspaces.map((workspace) => (
                <TableRow key={workspace.id}>
                  <TableCell>
                    <Typography variant="body2" fontWeight="bold">
                      {workspace.id}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    {workspace.name}
                    {workspace.description && (
                      <Typography variant="caption" display="block" color="text.secondary">
                        {workspace.description}
                      </Typography>
                    )}
                  </TableCell>
                  <TableCell>
                    <Chip
                      label={workspace.enabled ? 'Enabled' : 'Disabled'}
                      color={workspace.enabled ? 'success' : 'default'}
                      size="small"
                    />
                  </TableCell>
                  <TableCell align="right">
                    {workspace.stats.total_requests.toLocaleString()}
                  </TableCell>
                  <TableCell align="right">{workspace.stats.active_routes}</TableCell>
                  <TableCell align="right">
                    {workspace.stats.avg_response_time_ms.toFixed(2)} ms
                  </TableCell>
                  <TableCell align="right">
                    <IconButton
                      size="small"
                      onClick={() => handleToggleEnabled(workspace)}
                      title={workspace.enabled ? 'Disable' : 'Enable'}
                    >
                      {workspace.enabled ? <DisableIcon /> : <EnableIcon />}
                    </IconButton>
                    <IconButton
                      size="small"
                      onClick={() => handleOpenEdit(workspace)}
                      title="Edit"
                    >
                      <EditIcon />
                    </IconButton>
                    <IconButton
                      size="small"
                      onClick={() => handleDelete(workspace.id)}
                      title="Delete"
                      color="error"
                    >
                      <DeleteIcon />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </TableContainer>

      {/* Create Dialog */}
      <Dialog open={createDialogOpen} onClose={() => setCreateDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Create Workspace</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label="Workspace ID"
            fullWidth
            value={formData.id}
            onChange={(e) => setFormData({ ...formData, id: e.target.value })}
            helperText="Unique identifier for the workspace (e.g., frontend-dev)"
          />
          <TextField
            margin="dense"
            label="Name"
            fullWidth
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            helperText="Display name for the workspace"
          />
          <TextField
            margin="dense"
            label="Description"
            fullWidth
            multiline
            rows={3}
            value={formData.description}
            onChange={(e) => setFormData({ ...formData, description: e.target.value })}
            helperText="Optional description"
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCreateDialogOpen(false)}>Cancel</Button>
          <Button
            onClick={handleCreate}
            variant="contained"
            disabled={!formData.id || !formData.name}
          >
            Create
          </Button>
        </DialogActions>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={editDialogOpen} onClose={() => setEditDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Edit Workspace</DialogTitle>
        <DialogContent>
          <TextField
            margin="dense"
            label="Workspace ID"
            fullWidth
            value={formData.id}
            disabled
            helperText="Workspace ID cannot be changed"
          />
          <TextField
            margin="dense"
            label="Name"
            fullWidth
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
          />
          <TextField
            margin="dense"
            label="Description"
            fullWidth
            multiline
            rows={3}
            value={formData.description}
            onChange={(e) => setFormData({ ...formData, description: e.target.value })}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleUpdate} variant="contained" disabled={!formData.name}>
            Update
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default WorkspaceManagement;
