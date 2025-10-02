import React, { useState, useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter, DialogClose } from '../ui/Dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Switch } from '../ui/switch';
import { FolderOpen, Settings, Save, X } from 'lucide-react';
import type { SyncConfig, SyncDirection, SyncDirectoryStructure } from '../../types';

interface WorkspaceSettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaceId: string;
  workspaceName: string;
  currentConfig?: SyncConfig;
  onSave: (config: SyncConfig) => Promise<void>;
  loading?: boolean;
}

export function WorkspaceSettingsDialog({
  open,
  onOpenChange,
  workspaceName,
  currentConfig,
  onSave,
  loading = false
}: WorkspaceSettingsDialogProps) {
  const [config, setConfig] = useState<SyncConfig>({
    enabled: false,
    target_directory: '',
    directory_structure: 'Nested',
    sync_direction: 'Manual',
    include_metadata: true,
    realtime_monitoring: false,
    filename_pattern: '{name}',
    exclude_pattern: '',
    force_overwrite: false,
    ...currentConfig
  });

  useEffect(() => {
    if (currentConfig) {
      setConfig(currentConfig);
    }
  }, [currentConfig]);

  const handleSave = async () => {
    try {
      await onSave(config);
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save workspace settings:', error);
    }
  };

  interface WindowWithDirectoryPicker extends Window {
    showDirectoryPicker?: () => Promise<DirectoryHandle>;
  }

  const handleDirectorySelect = async () => {
    try {
      // Use the File System Access API if available, fallback to input
      const windowWithPicker = window as WindowWithDirectoryPicker;
      if ('showDirectoryPicker' in window && windowWithPicker.showDirectoryPicker) {
        const dirHandle = await windowWithPicker.showDirectoryPicker();
        const path = await getDirectoryPath(dirHandle);
        setConfig(prev => ({ ...prev, target_directory: path }));
      }
    } catch (error) {
      console.error('Failed to select directory:', error);
    }
  };

  interface DirectoryHandle {
    name: string;
  }

  const getDirectoryPath = async (dirHandle: DirectoryHandle): Promise<string> => {
    // This is a simplified implementation - in a real app you'd need to handle permissions
    // and construct the full path. For now, we'll just use the directory name.
    return dirHandle.name;
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-center justify-between">
            <div>
              <DialogTitle className="flex items-center gap-2">
                <Settings className="h-5 w-5" />
                Workspace Settings - {workspaceName}
              </DialogTitle>
              <DialogDescription>
                Configure synchronization and other workspace settings.
              </DialogDescription>
            </div>
            <DialogClose onClick={() => onOpenChange(false)} />
          </div>
        </DialogHeader>

        <div className="space-y-6">
          {/* Sync Configuration Section */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">Directory Synchronization</h3>

            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <Label htmlFor="sync-enabled">Enable Directory Sync</Label>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Automatically sync workspace changes with a local directory
                </p>
              </div>
              <Switch
                id="sync-enabled"
                checked={config.enabled}
                onCheckedChange={(checked) => setConfig(prev => ({ ...prev, enabled: checked }))}
              />
            </div>

            {config.enabled && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="target-directory">Target Directory</Label>
                  <div className="flex gap-2">
                    <Input
                      id="target-directory"
                      value={config.target_directory || ''}
                      onChange={(e) => setConfig(prev => ({ ...prev, target_directory: e.target.value }))}
                      placeholder="/path/to/sync/directory"
                      className="flex-1"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={handleDirectorySelect}
                      className="px-3"
                    >
                      <FolderOpen className="h-4 w-4" />
                    </Button>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Directory where workspace files will be synchronized
                  </p>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="sync-direction">Sync Direction</Label>
                    <Select
                      value={config.sync_direction}
                      onValueChange={(value: string) => setConfig(prev => ({ ...prev, sync_direction: value as SyncDirection }))}
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="Manual">Manual</SelectItem>
                        <SelectItem value="WorkspaceToDirectory">Workspace → Directory</SelectItem>
                        <SelectItem value="Bidirectional">Bidirectional</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="directory-structure">Directory Structure</Label>
                    <Select
                      value={config.directory_structure}
                      onValueChange={(value: string) => setConfig(prev => ({ ...prev, directory_structure: value as SyncDirectoryStructure }))}
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="Flat">Flat</SelectItem>
                        <SelectItem value="Nested">Nested</SelectItem>
                        <SelectItem value="Grouped">Grouped</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="filename-pattern">Filename Pattern</Label>
                  <Input
                    id="filename-pattern"
                    value={config.filename_pattern}
                    onChange={(e) => setConfig(prev => ({ ...prev, filename_pattern: e.target.value }))}
                    placeholder="{name}"
                  />
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Pattern for exported files. Use {'{name}'} for workspace name, {'{id}'} for workspace ID
                  </p>
                </div>

                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label htmlFor="realtime-monitoring">Real-time Monitoring</Label>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Automatically sync changes as they happen
                      </p>
                    </div>
                    <Switch
                      id="realtime-monitoring"
                      checked={config.realtime_monitoring}
                      onCheckedChange={(checked) => setConfig(prev => ({ ...prev, realtime_monitoring: checked }))}
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label htmlFor="include-metadata">Include Metadata</Label>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Include workspace metadata files in sync
                      </p>
                    </div>
                    <Switch
                      id="include-metadata"
                      checked={config.include_metadata}
                      onCheckedChange={(checked) => setConfig(prev => ({ ...prev, include_metadata: checked }))}
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label htmlFor="force-overwrite">Force Overwrite</Label>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Overwrite existing files without confirmation
                      </p>
                    </div>
                    <Switch
                      id="force-overwrite"
                      checked={config.force_overwrite}
                      onCheckedChange={(checked) => setConfig(prev => ({ ...prev, force_overwrite: checked }))}
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="exclude-pattern">Exclude Pattern</Label>
                  <Input
                    id="exclude-pattern"
                    value={config.exclude_pattern || ''}
                    onChange={(e) => setConfig(prev => ({ ...prev, exclude_pattern: e.target.value }))}
                    placeholder="*.tmp,*.log"
                  />
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Regex pattern for files to exclude from sync
                  </p>
                </div>
              </>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={loading}
          >
            <X className="h-4 w-4 mr-2" />
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={loading}
          >
            <Save className="h-4 w-4 mr-2" />
            {loading ? 'Saving...' : 'Save Settings'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}