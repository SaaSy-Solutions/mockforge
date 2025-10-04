import React, { useState } from 'react';
import { Upload, FileText, Link, Loader2, AlertTriangle, CheckCircle } from 'lucide-react';
import {
  Modal,
  Button,
  Input,
  Label,
  Alert
} from '../ui/DesignSystem';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger
} from '../ui/Tabs';

interface InstallPluginModalProps {
  onClose: () => void;
}

export function InstallPluginModal({ onClose }: InstallPluginModalProps) {
  const [installMethod, setInstallMethod] = useState<'file' | 'url'>('file');
  const [filePath, setFilePath] = useState('');
  const [url, setUrl] = useState('');
  const [pluginId, setPluginId] = useState('');
  const [skipValidation, setSkipValidation] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const source = installMethod === 'file' ? filePath : url;

  const handleInstall = async () => {
    if (!source.trim()) {
      setError('Please provide a plugin source');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const response = await fetch('/__mockforge/plugins/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source: source.trim(),
          id: pluginId.trim() || undefined,
          force: false, // For now, don't force
          skip_validation: skipValidation,
        }),
      });

      const data = await response.json();

      if (data.success) {
        setSuccess('Plugin installed successfully!');
        setTimeout(() => {
          onClose();
          window.location.reload(); // Refresh to show new plugin
        }, 2000);
      } else {
        setError(data.error);
      }
    } catch {
      setError('Failed to install plugin');
    } finally {
      setLoading(false);
    }
  };

  const handleValidate = async () => {
    if (!source.trim()) {
      setError('Please provide a plugin source to validate');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const response = await fetch('/__mockforge/plugins/validate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source: source.trim(),
          verbose: true,
        }),
      });

      const data = await response.json();

      if (data.success && data.data.valid) {
        setSuccess(`Plugin is valid: ${data.data.name} v${data.data.version}`);
      } else {
        setError(data.error || 'Plugin validation failed');
      }
    } catch {
      setError('Failed to validate plugin');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal open={true} onOpenChange={onClose} className="max-w-2xl">
      <div className="p-6">
        <div className="flex items-center gap-3 mb-6">
          <Upload className="w-6 h-6 text-blue-500" />
          <h2 className="text-xl font-bold">Install Plugin</h2>
        </div>

        <Tabs value={installMethod} onValueChange={(value) => setInstallMethod(value as 'file' | 'url')}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="file" className="flex items-center gap-2">
              <FileText className="w-4 h-4" />
              Local File
            </TabsTrigger>
            <TabsTrigger value="url" className="flex items-center gap-2">
              <Link className="w-4 h-4" />
              URL
            </TabsTrigger>
          </TabsList>

          <TabsContent value="file" className="space-y-4 mt-4">
            <div>
              <Label htmlFor="file-path">Plugin Directory Path</Label>
              <Input
                id="file-path"
                placeholder="/path/to/plugin-directory"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
              />
              <p className="text-xs text-gray-500 mt-1">
                Path to a directory containing plugin.yaml and WebAssembly file
              </p>
            </div>
          </TabsContent>

          <TabsContent value="url" className="space-y-4 mt-4">
            <div>
              <Label htmlFor="url">Plugin URL</Label>
              <Input
                id="url"
                placeholder="https://example.com/plugin.zip"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
              />
              <p className="text-xs text-gray-500 mt-1">
                URL to a plugin archive (.zip, .tar.gz) or Git repository
              </p>
            </div>
          </TabsContent>
        </Tabs>

        <div className="space-y-4 mt-6">
          <div>
            <Label htmlFor="plugin-id">Plugin ID (Optional)</Label>
            <Input
              id="plugin-id"
              placeholder="auto-generated"
              value={pluginId}
              onChange={(e) => setPluginId(e.target.value)}
            />
            <p className="text-xs text-gray-500 mt-1">
              Leave empty to auto-detect from plugin manifest
            </p>
          </div>

          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="skip-validation"
              checked={skipValidation}
              onChange={(e) => setSkipValidation(e.target.checked)}
              className="rounded border-gray-300"
            />
            <Label htmlFor="skip-validation" className="text-sm">
              Skip validation (not recommended)
            </Label>
          </div>
        </div>

        {error && (
          <Alert variant="destructive" className="mt-4">
            <AlertTriangle className="h-4 w-4" />
            <div>{error}</div>
          </Alert>
        )}

        {success && (
          <Alert variant="default" className="mt-4 border-green-200 bg-green-50">
            <CheckCircle className="h-4 w-4 text-green-600" />
            <div className="text-green-800">{success}</div>
          </Alert>
        )}

        <div className="flex justify-between mt-6">
          <Button variant="outline" onClick={handleValidate} disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Validating...
              </>
            ) : (
              'Validate'
            )}
          </Button>

          <div className="flex gap-3">
            <Button variant="outline" onClick={onClose} disabled={loading}>
              Cancel
            </Button>
            <Button onClick={handleInstall} disabled={loading}>
              {loading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Installing...
                </>
              ) : (
                'Install Plugin'
              )}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
