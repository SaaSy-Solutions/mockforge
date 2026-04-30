/**
 * Plugin Moderation (admin)
 *
 * Lists plugins that have been taken down via the admin "Take down" button
 * on `/plugin-registry`. Public search filters these out, so this page is
 * the only UI surface for finding them again and restoring them once the
 * post-takedown undo snackbar has expired.
 */

import React, { useEffect, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from '@mui/material';
import {
  ArrowBack as ArrowBackIcon,
  Refresh as RefreshIcon,
  Restore as RestoreIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { authenticatedFetch } from '../utils/apiClient';
import { useAuthStore } from '../stores/useAuthStore';

interface TakenDownPlugin {
  name: string;
  description: string;
  category: string;
  currentVersion: string;
  author: { id: string; username: string; email?: string | null };
  takenDownAt: string;
  reason?: string | null;
}

interface ListResponse {
  plugins: TakenDownPlugin[];
  total: number;
}

export const PluginModerationPage: React.FC = () => {
  const navigate = useNavigate();
  const currentUser = useAuthStore((s) => s.user);
  const isAdmin = currentUser?.role === 'admin';

  const [plugins, setPlugins] = useState<TakenDownPlugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const resp = await authenticatedFetch('/api/v1/admin/plugins/taken-down');
      if (!resp.ok) {
        const body = await resp.json().catch(() => ({}));
        throw new Error(body?.error || body?.message || `HTTP ${resp.status}`);
      }
      const data: ListResponse = await resp.json();
      setPlugins(data.plugins || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load taken-down plugins');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isAdmin) {
      load();
    } else {
      setLoading(false);
    }
  }, [isAdmin]);

  const handleRestore = async (name: string) => {
    if (!window.confirm(`Restore ${name}? It will reappear in search and detail views immediately.`)) {
      return;
    }
    setRestoring(name);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/admin/plugins/${encodeURIComponent(name)}/restore`,
        { method: 'POST' }
      );
      if (!resp.ok) {
        const body = await resp.json().catch(() => ({}));
        throw new Error(body?.error || body?.message || `HTTP ${resp.status}`);
      }
      setFeedback(`Restored ${name}`);
      setPlugins((prev) => prev.filter((p) => p.name !== name));
    } catch (err) {
      setFeedback(err instanceof Error ? err.message : 'Restore failed');
    } finally {
      setRestoring(null);
      setTimeout(() => setFeedback(null), 4000);
    }
  };

  if (!isAdmin) {
    return (
      <Box sx={{ p: 3 }}>
        <Alert severity="error">
          Plugin moderation is restricted to administrators.
        </Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3 }}>
      <Stack direction="row" alignItems="center" spacing={1} sx={{ mb: 1 }}>
        <Tooltip title="Back to plugin registry">
          <IconButton onClick={() => navigate('/plugin-registry')} size="small">
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Plugin Moderation
        </Typography>
        <Tooltip title="Reload">
          <span>
            <IconButton onClick={load} disabled={loading}>
              <RefreshIcon />
            </IconButton>
          </span>
        </Tooltip>
      </Stack>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
        Plugins currently hidden from public search. Restoring a plugin makes
        it discoverable and installable again immediately.
      </Typography>

      {feedback && (
        <Alert severity="info" sx={{ mb: 2 }} onClose={() => setFeedback(null)}>
          {feedback}
        </Alert>
      )}

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} action={
          <Button color="inherit" size="small" onClick={load}>Retry</Button>
        }>
          {error}
        </Alert>
      )}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
          <CircularProgress />
        </Box>
      ) : plugins.length === 0 ? (
        <Paper variant="outlined" sx={{ p: 4, textAlign: 'center' }}>
          <Typography color="text.secondary">
            No plugins are currently taken down.
          </Typography>
        </Paper>
      ) : (
        <TableContainer component={Paper} variant="outlined">
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Plugin</TableCell>
                <TableCell>Author</TableCell>
                <TableCell>Reason</TableCell>
                <TableCell>Taken down</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {plugins.map((plugin) => (
                <TableRow key={plugin.name} hover>
                  <TableCell>
                    <Stack spacing={0.5}>
                      <Stack direction="row" spacing={1} alignItems="center">
                        <Typography variant="subtitle2">{plugin.name}</Typography>
                        <Chip label={plugin.category} size="small" variant="outlined" />
                        <Chip label={`v${plugin.currentVersion}`} size="small" variant="outlined" />
                      </Stack>
                      <Typography variant="caption" color="text.secondary">
                        {plugin.description}
                      </Typography>
                    </Stack>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2">{plugin.author.username}</Typography>
                    {plugin.author.email && (
                      <Typography variant="caption" color="text.secondary">
                        {plugin.author.email}
                      </Typography>
                    )}
                  </TableCell>
                  <TableCell sx={{ maxWidth: 280 }}>
                    {plugin.reason ? (
                      <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>
                        {plugin.reason}
                      </Typography>
                    ) : (
                      <Typography variant="caption" color="text.secondary">
                        (no reason recorded)
                      </Typography>
                    )}
                  </TableCell>
                  <TableCell>
                    <Tooltip title={new Date(plugin.takenDownAt).toLocaleString()}>
                      <Typography variant="caption">
                        {new Date(plugin.takenDownAt).toLocaleDateString()}
                      </Typography>
                    </Tooltip>
                  </TableCell>
                  <TableCell align="right">
                    <Button
                      size="small"
                      variant="outlined"
                      startIcon={<RestoreIcon />}
                      disabled={restoring === plugin.name}
                      onClick={() => handleRestore(plugin.name)}
                    >
                      {restoring === plugin.name ? 'Restoring…' : 'Restore'}
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </Box>
  );
};

export default PluginModerationPage;
