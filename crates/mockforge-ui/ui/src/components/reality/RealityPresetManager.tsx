/**
 * Reality Preset Manager Component
 *
 * Allows users to export current reality configurations as presets
 * and import previously saved presets. Provides a clean interface
 * for managing reality level configurations.
 */

import React, { useState } from 'react';
import { Download, Upload, FileText, X, Check } from 'lucide-react';
import { cn } from '../../utils/cn';
import {
  useRealityPresets,
  useImportRealityPreset,
  useExportRealityPreset,
} from '../../hooks/useApi';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from '../ui/Dialog';
import { Badge } from '../ui/Badge';
import { Alert } from '../ui/DesignSystem';
import { toast } from 'sonner';

interface RealityPresetManagerProps {
  className?: string;
}

export function RealityPresetManager({ className }: RealityPresetManagerProps) {
  const { data: presets, isLoading: presetsLoading } = useRealityPresets();
  const importMutation = useImportRealityPreset();
  const exportMutation = useExportRealityPreset();

  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [presetName, setPresetName] = useState('');
  const [presetDescription, setPresetDescription] = useState('');
  const [selectedPresetPath, setSelectedPresetPath] = useState<string>('');

  const handleExport = () => {
    if (!presetName.trim()) {
      toast.error('Preset name is required');
      return;
    }

    exportMutation.mutate(
      {
        name: presetName.trim(),
        description: presetDescription.trim() || undefined,
      },
      {
        onSuccess: (data) => {
          toast.success('Preset exported successfully', {
            description: `Saved to ${data.path}`,
          });
          setExportDialogOpen(false);
          setPresetName('');
          setPresetDescription('');
        },
        onError: (error) => {
          toast.error('Failed to export preset', {
            description: error instanceof Error ? error.message : 'Unknown error',
          });
        },
      }
    );
  };

  const handleImport = (path: string) => {
    importMutation.mutate(path, {
      onSuccess: (data) => {
        toast.success('Preset imported successfully', {
          description: `Applied ${data.name} (Level ${data.level}: ${data.level_name})`,
        });
        setImportDialogOpen(false);
        setSelectedPresetPath('');
      },
      onError: (error) => {
        toast.error('Failed to import preset', {
          description: error instanceof Error ? error.message : 'Unknown error',
        });
      },
    });
  };

  return (
    <Card className={cn('p-6', className)}>
      <CardHeader>
        <CardTitle className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Reality Presets
        </CardTitle>
        <CardDescription className="text-sm text-gray-600 dark:text-gray-400">
          Save and load reality level configurations for different testing scenarios
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Action Buttons */}
        <div className="flex items-center gap-3">
          <Dialog open={exportDialogOpen} onOpenChange={setExportDialogOpen}>
            <DialogTrigger asChild>
              <Button
                variant="default"
                className="flex items-center gap-2"
                disabled={exportMutation.isPending}
              >
                <Download className="h-4 w-4" />
                Export Current
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Export Reality Preset</DialogTitle>
                <DialogDescription>
                  Save the current reality level configuration as a preset for later use
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div>
                  <Label htmlFor="preset-name">Preset Name *</Label>
                  <Input
                    id="preset-name"
                    value={presetName}
                    onChange={(e) => setPresetName(e.target.value)}
                    placeholder="e.g., production-chaos, staging-realistic"
                    className="mt-1"
                  />
                </div>
                <div>
                  <Label htmlFor="preset-description">Description (Optional)</Label>
                  <Textarea
                    id="preset-description"
                    value={presetDescription}
                    onChange={(e) => setPresetDescription(e.target.value)}
                    placeholder="Describe when to use this preset..."
                    className="mt-1"
                    rows={3}
                  />
                </div>
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => {
                    setExportDialogOpen(false);
                    setPresetName('');
                    setPresetDescription('');
                  }}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleExport}
                  disabled={!presetName.trim() || exportMutation.isPending}
                >
                  {exportMutation.isPending ? 'Exporting...' : 'Export Preset'}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={importDialogOpen} onOpenChange={setImportDialogOpen}>
            <DialogTrigger asChild>
              <Button
                variant="outline"
                className="flex items-center gap-2"
                disabled={importMutation.isPending}
              >
                <Upload className="h-4 w-4" />
                Import Preset
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Import Reality Preset</DialogTitle>
                <DialogDescription>
                  Load a previously saved reality level configuration
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                {presetsLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <div className="h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" />
                  </div>
                ) : presets && presets.length > 0 ? (
                  <div className="space-y-2 max-h-64 overflow-y-auto">
                    {presets.map((preset) => (
                      <button
                        key={preset.id}
                        type="button"
                        onClick={() => {
                          setSelectedPresetPath(preset.path);
                          handleImport(preset.path);
                        }}
                        disabled={importMutation.isPending}
                        className={cn(
                          'w-full text-left p-3 rounded-lg border transition-all duration-200',
                          'hover:bg-gray-50 dark:hover:bg-gray-800',
                          'hover:border-gray-300 dark:hover:border-gray-600',
                          'disabled:opacity-50 disabled:cursor-not-allowed',
                          selectedPresetPath === preset.path
                            ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                            : 'border-gray-200 dark:border-gray-700'
                        )}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <FileText className="h-4 w-4 text-gray-500 dark:text-gray-400" />
                            <span className="font-medium text-gray-900 dark:text-gray-100">
                              {preset.name}
                            </span>
                          </div>
                          {selectedPresetPath === preset.path && importMutation.isPending && (
                            <div className="h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" />
                          )}
                          {selectedPresetPath === preset.path && !importMutation.isPending && (
                            <Check className="h-4 w-4 text-green-500" />
                          )}
                        </div>
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          {preset.path}
                        </p>
                      </button>
                    ))}
                  </div>
                ) : (
                  <Alert variant="info" className="mt-4">
                    <p className="text-sm">No presets available. Export a preset to get started.</p>
                  </Alert>
                )}
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => {
                    setImportDialogOpen(false);
                    setSelectedPresetPath('');
                  }}
                >
                  Close
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>

        {/* Presets List */}
        {presetsLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" />
          </div>
        ) : presets && presets.length > 0 ? (
          <div className="space-y-2">
            <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Available Presets ({presets.length})
            </h4>
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {presets.map((preset) => (
                <div
                  key={preset.id}
                  className="flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50"
                >
                  <div className="flex items-center gap-2 flex-1 min-w-0">
                    <FileText className="h-4 w-4 text-gray-500 dark:text-gray-400 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                        {preset.name}
                      </p>
                      <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                        {preset.path}
                      </p>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleImport(preset.path)}
                    disabled={importMutation.isPending}
                    className="flex-shrink-0"
                  >
                    <Upload className="h-4 w-4 mr-1" />
                    Load
                  </Button>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <Alert variant="info">
            <p className="text-sm">No presets saved yet. Export your current configuration to create one.</p>
          </Alert>
        )}

        {/* Loading State */}
        {(importMutation.isPending || exportMutation.isPending) && (
          <div className="flex items-center justify-center gap-2 text-sm text-gray-600 dark:text-gray-400 py-2">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" />
            <span>
              {importMutation.isPending ? 'Importing preset...' : 'Exporting preset...'}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
