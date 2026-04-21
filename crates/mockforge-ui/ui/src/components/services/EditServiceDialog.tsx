import React, { useEffect, useState } from 'react';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../ui/Dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import { useServiceStore } from '../../stores/useServiceStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import type { ServiceInfo } from '../../types';

interface EditServiceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  service: ServiceInfo | null;
}

const parseTags = (raw: string): string[] =>
  raw
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0);

export function EditServiceDialog({ open, onOpenChange, service }: EditServiceDialogProps) {
  const updateServiceDetails = useServiceStore((s) => s.updateServiceDetails);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const loadWorkspaces = useWorkspaceStore((s) => s.loadWorkspaces);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [workspaceId, setWorkspaceId] = useState<string>('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open && workspaces.length === 0) {
      void loadWorkspaces();
    }
  }, [open, workspaces.length, loadWorkspaces]);

  useEffect(() => {
    if (open && service) {
      setName(service.name);
      setDescription(service.description ?? '');
      setBaseUrl(service.baseUrl ?? '');
      setTagsInput((service.tags ?? []).join(', '));
      setWorkspaceId(service.workspace_id ?? '');
      setError(null);
      setSubmitting(false);
    }
  }, [open, service]);

  if (!service) return null;

  const canSubmit = name.trim().length > 0 && !submitting;

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    try {
      const originalWorkspace = service.workspace_id ?? '';
      const details: Parameters<typeof updateServiceDetails>[1] = {
        name: name.trim(),
        description: description.trim(),
        base_url: baseUrl.trim(),
        tags: parseTags(tagsInput),
      };
      if (workspaceId !== originalWorkspace) {
        // `""` → send explicit null to unassign; otherwise assign to picked id.
        details.workspace_id = workspaceId === '' ? null : workspaceId;
      }
      await updateServiceDetails(service.id, details);
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update service');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit Service</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4 pt-4">
          <div className="space-y-2">
            <Label htmlFor="edit-service-name" required>Name</Label>
            <Input
              id="edit-service-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              required
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="edit-service-description">Description</Label>
            <Textarea
              id="edit-service-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="edit-service-base-url">Base URL</Label>
            <Input
              id="edit-service-base-url"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://api.example.com"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="edit-service-tags">Tags</Label>
            <Input
              id="edit-service-tags"
              value={tagsInput}
              onChange={(e) => setTagsInput(e.target.value)}
              placeholder="api, users, internal"
            />
            <p className="text-xs text-muted-foreground">Comma-separated list.</p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="edit-service-workspace">Workspace</Label>
            <select
              id="edit-service-workspace"
              value={workspaceId}
              onChange={(e) => setWorkspaceId(e.target.value)}
              className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <option value="">No workspace</option>
              {workspaces.map((w) => (
                <option key={w.id} value={w.id}>{w.name}</option>
              ))}
            </select>
            {service.workspace_id && workspaceId === '' && (
              <p className="text-xs text-muted-foreground">
                Saving will unassign this service from its current workspace.
              </p>
            )}
          </div>
          {error && (
            <p role="alert" className="text-sm text-red-500">
              {error}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)} disabled={submitting}>
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit}>
              {submitting ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
