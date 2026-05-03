import React, { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Settings, Search, FolderSync } from 'lucide-react';
import { apiService } from '../../services/api';
import type { SyncStatus, SyncChange, SyncConfig, ConfigureSyncRequest } from '../../types';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/Card';
import { SyncStatusIndicator } from './SyncStatusIndicator';
import { SyncConfirmationDialog } from './SyncConfirmationDialog';
import { WorkspaceSettingsDialog } from './WorkspaceSettingsDialog';
import { logger } from '@/utils/logger';

interface Props {
  workspaceId: string;
  workspaceName: string;
}

const statusToConfig = (status: SyncStatus | null): SyncConfig => ({
  enabled: status?.enabled ?? false,
  target_directory: status?.target_directory ?? '',
  directory_structure: status?.directory_structure ?? 'Nested',
  sync_direction: status?.sync_direction ?? 'Manual',
  include_metadata: status?.include_metadata ?? true,
  realtime_monitoring: status?.realtime_monitoring ?? false,
  filename_pattern: status?.filename_pattern ?? '{name}',
  exclude_pattern: status?.exclude_pattern,
  force_overwrite: status?.force_overwrite ?? false,
});

const WorkspaceSyncPanel: React.FC<Props> = ({ workspaceId, workspaceName }) => {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [changes, setChanges] = useState<SyncChange[]>([]);
  const [statusLoading, setStatusLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const loadStatus = useCallback(async () => {
    setStatusLoading(true);
    try {
      const response = await apiService.getSyncStatus(workspaceId);
      setStatus(response);
    } catch (error) {
      logger.error('Failed to load sync status', error);
    } finally {
      setStatusLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleSaveConfig = async (config: SyncConfig) => {
    if (!config.target_directory) {
      toast.error('Target directory is required');
      return;
    }
    setActionLoading(true);
    try {
      const request: ConfigureSyncRequest = {
        target_directory: config.target_directory,
        sync_direction: config.sync_direction,
        realtime_monitoring: config.realtime_monitoring,
        directory_structure: config.directory_structure,
        filename_pattern: config.filename_pattern,
        include_metadata: config.include_metadata,
        exclude_pattern: config.exclude_pattern,
        force_overwrite: config.force_overwrite,
      };
      if (config.enabled) {
        await apiService.configureSync(workspaceId, request);
        toast.success('Sync configuration saved');
      } else {
        await apiService.disableSync(workspaceId);
        toast.success('Sync disabled');
      }
      await loadStatus();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to save sync settings');
      throw error;
    } finally {
      setActionLoading(false);
    }
  };

  const handleTriggerSync = async () => {
    setActionLoading(true);
    try {
      await apiService.triggerSync(workspaceId);
      toast.success('Sync triggered');
      await loadStatus();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to trigger sync');
    } finally {
      setActionLoading(false);
    }
  };

  const handleDisableSync = async () => {
    if (!confirm('Disable directory sync for this workspace?')) return;
    setActionLoading(true);
    try {
      await apiService.disableSync(workspaceId);
      toast.success('Sync disabled');
      await loadStatus();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to disable sync');
    } finally {
      setActionLoading(false);
    }
  };

  const handleCheckChanges = async () => {
    setActionLoading(true);
    try {
      const detected = await apiService.getSyncChanges(workspaceId);
      setChanges(detected);
      if (detected.length === 0) {
        toast.success('No pending changes detected');
      } else {
        setConfirmOpen(true);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to check for changes');
    } finally {
      setActionLoading(false);
    }
  };

  const handleConfirmChanges = async (applyAll: boolean) => {
    setActionLoading(true);
    try {
      await apiService.confirmSyncChanges(workspaceId, {
        workspace_id: workspaceId,
        changes,
        apply_all: applyAll,
      });
      toast.success('Changes applied');
      setChanges([]);
      await loadStatus();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to apply changes');
      throw error;
    } finally {
      setActionLoading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between gap-4">
          <div>
            <CardTitle className="flex items-center gap-2">
              <FolderSync className="w-4 h-4" />
              Directory Sync
            </CardTitle>
            <CardDescription>
              Mirror this workspace to a local directory for git tracking and editing on disk.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setSettingsOpen(true)}
            disabled={statusLoading}
          >
            <Settings className="w-4 h-4 mr-2" />
            {status?.enabled ? 'Edit Settings' : 'Configure Sync'}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {status ? (
          <>
            <SyncStatusIndicator
              status={status}
              onSyncNow={status.enabled ? handleTriggerSync : undefined}
              loading={actionLoading}
            />
            {status.enabled && (
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCheckChanges}
                  disabled={actionLoading}
                >
                  <Search className="w-4 h-4 mr-2" />
                  Check for Directory Changes
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDisableSync}
                  disabled={actionLoading}
                >
                  Disable Sync
                </Button>
              </div>
            )}
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            {statusLoading ? 'Loading sync status...' : 'No sync status available'}
          </p>
        )}
      </CardContent>

      <WorkspaceSettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        workspaceId={workspaceId}
        workspaceName={workspaceName}
        currentConfig={statusToConfig(status)}
        onSave={handleSaveConfig}
        loading={actionLoading}
      />

      <SyncConfirmationDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        workspaceId={workspaceId}
        changes={changes}
        onConfirm={handleConfirmChanges}
        loading={actionLoading}
      />
    </Card>
  );
};

export default WorkspaceSyncPanel;
