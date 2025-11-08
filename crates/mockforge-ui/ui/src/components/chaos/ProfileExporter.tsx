/**
 * Profile Export/Import Component
 *
 * Allows users to export current chaos configuration as a profile template
 * and import profiles from JSON/YAML files.
 */

import React, { useState } from 'react';
import { ModernCard } from '../ui/DesignSystem';
import { Button } from '../ui/button';
import {
  useExportNetworkProfile,
  useImportNetworkProfile,
  useChaosConfig,
  useNetworkProfiles,
} from '../../hooks/useApi';
import { toast } from 'sonner';
import { Download, Upload, FileText, Loader2 } from 'lucide-react';

interface ProfileExporterProps {
  /** Whether the component is compact (for header/toolbar) */
  compact?: boolean;
}

export function ProfileExporter({ compact = false }: ProfileExporterProps) {
  const [exportFormat, setExportFormat] = useState<'json' | 'yaml'>('json');
  const [importFormat, setImportFormat] = useState<'json' | 'yaml'>('json');
  const [importContent, setImportContent] = useState('');
  const [showImport, setShowImport] = useState(false);

  const { data: currentConfig } = useChaosConfig();
  const { data: profiles } = useNetworkProfiles();
  const exportProfile = useExportNetworkProfile();
  const importProfile = useImportNetworkProfile();

  const handleExportCurrent = async () => {
    if (!currentConfig) {
      toast.error('No configuration to export');
      return;
    }

    try {
      // Create a temporary profile from current config
      const profileData = {
        name: `exported_${Date.now()}`,
        description: 'Exported chaos configuration',
        chaos_config: currentConfig,
        tags: ['exported'],
        builtin: false,
      };

      const content =
        exportFormat === 'yaml'
          ? JSON.stringify(profileData, null, 2) // Simplified - would need YAML library
          : JSON.stringify(profileData, null, 2);

      // Download file
      const blob = new Blob([content], {
        type: exportFormat === 'yaml' ? 'text/yaml' : 'application/json',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `chaos-profile.${exportFormat === 'yaml' ? 'yaml' : 'json'}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast.success(`Profile exported as ${exportFormat.toUpperCase()}`);
    } catch (error: any) {
      toast.error(`Failed to export profile: ${error.message || 'Unknown error'}`);
    }
  };

  const handleExportProfile = async (profileName: string) => {
    try {
      const data = await exportProfile.mutateAsync({
        name: profileName,
        format: exportFormat,
      });

      const content =
        typeof data === 'string' ? data : JSON.stringify(data, null, 2);

      // Download file
      const blob = new Blob([content], {
        type: exportFormat === 'yaml' ? 'text/yaml' : 'application/json',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${profileName}.${exportFormat === 'yaml' ? 'yaml' : 'json'}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast.success(`Profile "${profileName}" exported successfully`);
    } catch (error: any) {
      toast.error(`Failed to export profile: ${error.message || 'Unknown error'}`);
    }
  };

  const handleImport = async () => {
    if (!importContent.trim()) {
      toast.error('Please provide profile content to import');
      return;
    }

    try {
      await importProfile.mutateAsync({
        content: importContent,
        format: importFormat,
      });
      setImportContent('');
      setShowImport(false);
      toast.success('Profile imported successfully');
    } catch (error: any) {
      toast.error(`Failed to import profile: ${error.message || 'Unknown error'}`);
    }
  };

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      setImportContent(content);
      // Detect format from file extension
      if (file.name.endsWith('.yaml') || file.name.endsWith('.yml')) {
        setImportFormat('yaml');
      } else {
        setImportFormat('json');
      }
    };
    reader.readAsText(file);
  };

  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={handleExportCurrent}
          disabled={!currentConfig || exportProfile.isPending}
        >
          {exportProfile.isPending ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Download className="h-4 w-4 mr-2" />
          )}
          Export
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setShowImport(true)}
        >
          <Upload className="h-4 w-4 mr-2" />
          Import
        </Button>
        {showImport && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <ModernCard className="w-full max-w-2xl">
              <div className="space-y-4">
                <h3 className="text-lg font-semibold">Import Profile</h3>
                <div>
                  <label className="block text-sm font-medium mb-2">
                    Format
                  </label>
                  <select
                    value={importFormat}
                    onChange={(e) => setImportFormat(e.target.value as 'json' | 'yaml')}
                    className="w-full px-3 py-2 border rounded-lg"
                  >
                    <option value="json">JSON</option>
                    <option value="yaml">YAML</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-2">
                    Upload File
                  </label>
                  <input
                    type="file"
                    accept=".json,.yaml,.yml"
                    onChange={handleFileUpload}
                    className="w-full"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-2">
                    Or Paste Content
                  </label>
                  <textarea
                    value={importContent}
                    onChange={(e) => setImportContent(e.target.value)}
                    placeholder="Paste profile JSON or YAML here..."
                    className="w-full h-48 px-3 py-2 border rounded-lg font-mono text-sm"
                  />
                </div>
                <div className="flex justify-end gap-3">
                  <Button variant="outline" onClick={() => setShowImport(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleImport} disabled={importProfile.isPending}>
                    {importProfile.isPending ? (
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    ) : (
                      <Upload className="h-4 w-4 mr-2" />
                    )}
                    Import
                  </Button>
                </div>
              </div>
            </ModernCard>
          </div>
        )}
      </div>
    );
  }

  return (
    <ModernCard>
      <div className="space-y-6">
        <div className="flex items-center gap-2">
          <FileText className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Export/Import Profiles</h3>
        </div>

        {/* Export Section */}
        <div className="space-y-4">
          <div>
            <h4 className="text-sm font-medium mb-3">Export Current Configuration</h4>
            <div className="flex items-center gap-3">
              <select
                value={exportFormat}
                onChange={(e) => setExportFormat(e.target.value as 'json' | 'yaml')}
                className="px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900"
              >
                <option value="json">JSON</option>
                <option value="yaml">YAML</option>
              </select>
              <Button
                onClick={handleExportCurrent}
                disabled={!currentConfig || exportProfile.isPending}
              >
                {exportProfile.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Exporting...
                  </>
                ) : (
                  <>
                    <Download className="h-4 w-4 mr-2" />
                    Export Current Config
                  </>
                )}
              </Button>
            </div>
          </div>

          {/* Export Specific Profile */}
          {profiles && profiles.length > 0 && (
            <div>
              <h4 className="text-sm font-medium mb-3">Export Profile</h4>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
                {profiles.slice(0, 8).map((profile) => (
                  <Button
                    key={profile.name}
                    variant="outline"
                    size="sm"
                    onClick={() => handleExportProfile(profile.name)}
                    disabled={exportProfile.isPending}
                  >
                    <Download className="h-3 w-3 mr-1" />
                    {profile.name}
                  </Button>
                ))}
              </div>
            </div>
          )}

          {/* Import Section */}
          <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
            <h4 className="text-sm font-medium mb-3">Import Profile</h4>
            <div className="space-y-3">
              <div>
                <label className="block text-sm font-medium mb-2">
                  Format
                </label>
                <select
                  value={importFormat}
                  onChange={(e) => setImportFormat(e.target.value as 'json' | 'yaml')}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900"
                >
                  <option value="json">JSON</option>
                  <option value="yaml">YAML</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">
                  Upload File
                </label>
                <input
                  type="file"
                  accept=".json,.yaml,.yml"
                  onChange={handleFileUpload}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">
                  Or Paste Content
                </label>
                <textarea
                  value={importContent}
                  onChange={(e) => setImportContent(e.target.value)}
                  placeholder="Paste profile JSON or YAML here..."
                  className="w-full h-32 px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg font-mono text-sm bg-white dark:bg-gray-900"
                />
              </div>
              <Button
                onClick={handleImport}
                disabled={!importContent.trim() || importProfile.isPending}
                className="w-full"
              >
                {importProfile.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Importing...
                  </>
                ) : (
                  <>
                    <Upload className="h-4 w-4 mr-2" />
                    Import Profile
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </ModernCard>
  );
}
