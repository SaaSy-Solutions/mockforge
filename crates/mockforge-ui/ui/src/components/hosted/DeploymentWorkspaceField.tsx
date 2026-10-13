import { useEffect, useId, useState } from 'react';
import { Alert, Box, Button, FormControl, InputLabel, MenuItem, Select, Typography } from '@mui/material';
import { fetchJsonWithErrorBody } from '@/services/api/client';

interface WorkspaceOption { id: string; name: string }

/** Explicit selection only: never infer a workspace from the user's defaults. */
export function DeploymentWorkspaceSelect({ value, onChange, disabled = false, orgId }: {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  orgId?: string;
}) {
  const labelId = useId();
  const [workspaces, setWorkspaces] = useState<WorkspaceOption[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(null);
    setWorkspaces([]);
    async function load() {
      try {
        const data = await fetchJsonWithErrorBody('/api/v1/workspaces', {
          credentials: 'include', signal: controller.signal,
          headers: orgId ? { 'X-Organization-Id': orgId } : undefined,
        });
        if (!Array.isArray(data) || !data.every(item =>
          typeof item?.id === 'string' && typeof item?.name === 'string')) {
          throw new Error('Could not read the workspace list');
        }
        if (!controller.signal.aborted) setWorkspaces(data);
      } catch (err) {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : 'Could not load workspaces');
        }
      } finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }
    void load();
    return () => controller.abort();
  }, [orgId, attempt]);

  return (
    <Box>
      <FormControl fullWidth>
        <InputLabel id={labelId}>Workspace (optional)</InputLabel>
        <Select aria-labelledby={labelId} label="Workspace (optional)" value={value}
          disabled={disabled || loading} onChange={event => onChange(event.target.value as string)}>
          <MenuItem value="">No workspace</MenuItem>
          {value && !workspaces.some(workspace => workspace.id === value) && (
            <MenuItem value={value}>Current workspace</MenuItem>
          )}
          {workspaces.map(workspace => (
            <MenuItem key={workspace.id} value={workspace.id}>{workspace.name}</MenuItem>
          ))}
        </Select>
      </FormControl>
      <Typography variant="caption" color="text.secondary" component="p">
        {loading ? 'Loading workspaces…' : 'Group future captures in this workspace for verification and alerts. Existing captures keep their workspace.'}
      </Typography>
      {error && (
        <Alert severity="warning">
          {error}
          <Button size="small" disabled={disabled} onClick={() => setAttempt(previous => previous + 1)}>
            Retry workspaces
          </Button>
        </Alert>
      )}
    </Box>
  );
}

export function DeploymentWorkspaceEditor({ deploymentId, orgId, workspaceId, onSaved }: {
  deploymentId: string;
  orgId: string;
  workspaceId?: string | null;
  onSaved: (workspaceId: string | null) => void;
}) {
  const [value, setValue] = useState(workspaceId ?? '');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const save = async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      const result = await fetchJsonWithErrorBody(
        `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/workspace`, {
          method: 'PATCH', credentials: 'include',
          headers: { 'Content-Type': 'application/json', 'X-Organization-Id': orgId },
          body: JSON.stringify({ workspace_id: value || null }),
        },
      ) as { id: string; workspace_id: string | null };
      if (result.id !== deploymentId ||
          (result.workspace_id !== null && typeof result.workspace_id !== 'string')) {
        throw new Error('Could not confirm the saved workspace');
      }
      setValue(result.workspace_id ?? '');
      onSaved(result.workspace_id);
      setSaved(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save the workspace');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Box sx={{ my: 2 }}>
      <DeploymentWorkspaceSelect value={value} orgId={orgId} disabled={saving}
        onChange={next => { setValue(next); setSaved(false); setError(null); }} />
      <Button disabled={saving || value === (workspaceId ?? '')} onClick={save}>
        {saving ? 'Saving workspace…' : 'Save workspace'}
      </Button>
      {error && <Alert severity="error">{error}</Alert>}
      {saved && <Alert severity="success">Workspace saved</Alert>}
    </Box>
  );
}
