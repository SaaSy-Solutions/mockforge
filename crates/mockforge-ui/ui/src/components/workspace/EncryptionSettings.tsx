import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import { Switch } from '../ui/switch';
import { Badge } from '../ui/Badge';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from '../ui/Dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/Tabs';
import { Alert, AlertDescription, AlertTitle } from '../ui/alert';
import { apiService } from '../../services/api';
import type {
  EncryptionStatus,
  AutoEncryptionConfig,
  SecurityCheckResult,
} from '../../types';
import {
  Shield,
  Key,
  Download,
  Upload,
  AlertTriangle,
  CheckCircle,
  Settings,
  Eye,
  EyeOff,
  Copy,
  RefreshCw,
  Lock,
  Unlock,
  Database,
} from 'lucide-react';
import { toast } from 'sonner';

interface EncryptionSettingsProps {
  workspaceId: string;
  workspaceName: string;
}


const EncryptionSettings: React.FC<EncryptionSettingsProps> = ({
  workspaceId,
  workspaceName,
}) => {
  const [status, setStatus] = useState<EncryptionStatus>({
    enabled: false,
    masterKeySet: false,
    workspaceKeySet: false,
  });

  const [config, setConfig] = useState<AutoEncryptionConfig>({
    enabled: false,
    sensitiveHeaders: ['authorization', 'x-api-key', 'x-auth-token', 'cookie'],
    sensitiveFields: ['password', 'token', 'secret', 'key', 'credentials'],
    sensitiveEnvVars: ['API_KEY', 'SECRET_KEY', 'PASSWORD', 'TOKEN', 'DATABASE_URL'],
    customPatterns: [],
  });

  const [securityCheck, setSecurityCheck] = useState<SecurityCheckResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [showBackupKey, setShowBackupKey] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [exportPath, setExportPath] = useState('');
  const [importPath, setImportPath] = useState('');
  const [importBackupKey, setImportBackupKey] = useState('');

  useEffect(() => {
    loadEncryptionStatus();
    loadEncryptionConfig();
  }, [workspaceId]);

  const loadEncryptionStatus = async () => {
    try {
      const response = await apiService.getWorkspaceEncryptionStatus(workspaceId);
      setStatus(response);
    } catch (error) {
      console.error('Failed to load encryption status:', error);
    }
  };

  const loadEncryptionConfig = async () => {
    try {
      const response = await apiService.getWorkspaceEncryptionConfig(workspaceId);
      setConfig(response);
    } catch (error) {
      console.error('Failed to load encryption config:', error);
    }
  };

  const enableEncryption = async () => {
    setLoading(true);
    try {
      await apiService.enableWorkspaceEncryption(workspaceId);
      toast.success('Encryption enabled for workspace');
      await loadEncryptionStatus();
    } catch (error) {
      toast.error('Failed to enable encryption');
      console.error('Error enabling encryption:', error);
    } finally {
      setLoading(false);
    }
  };

  const disableEncryption = async () => {
    if (!confirm('Are you sure you want to disable encryption? This will remove all encrypted data.')) {
      return;
    }

    setLoading(true);
    try {
      await apiService.disableWorkspaceEncryption(workspaceId);
      toast.success('Encryption disabled for workspace');
      await loadEncryptionStatus();
    } catch (error) {
      toast.error('Failed to disable encryption');
      console.error('Error disabling encryption:', error);
    } finally {
      setLoading(false);
    }
  };

  const runSecurityCheck = async () => {
    setLoading(true);
    try {
      const response = await apiService.checkWorkspaceSecurity(workspaceId);
      setSecurityCheck(response);
      toast.success('Security check completed');
    } catch (error) {
      toast.error('Failed to run security check');
      console.error('Error running security check:', error);
    } finally {
      setLoading(false);
    }
  };

  const exportEncrypted = async () => {
    setLoading(true);
    try {
      await apiService.exportWorkspaceEncrypted(workspaceId, exportPath);
      toast.success(`Workspace exported successfully to ${exportPath}`);
      setExportDialogOpen(false);
      setExportPath('');
    } catch (error) {
      toast.error('Failed to export workspace');
      console.error('Error exporting workspace:', error);
    } finally {
      setLoading(false);
    }
  };

  const importEncrypted = async () => {
    setLoading(true);
    try {
      await apiService.importWorkspaceEncrypted(importPath, workspaceId, importBackupKey);
      toast.success('Workspace imported successfully');
      setImportDialogOpen(false);
      setImportPath('');
      setImportBackupKey('');
      await loadEncryptionStatus();
    } catch (error) {
      toast.error('Failed to import workspace');
      console.error('Error importing workspace:', error);
    } finally {
      setLoading(false);
    }
  };

  const updateConfig = async () => {
    setLoading(true);
    try {
      await apiService.updateWorkspaceEncryptionConfig(workspaceId, config);
      toast.success('Encryption configuration updated');
    } catch (error) {
      toast.error('Failed to update configuration');
      console.error('Error updating configuration:', error);
    } finally {
      setLoading(false);
    }
  };

  const copyBackupKey = () => {
    if (status.backupKey) {
      navigator.clipboard.writeText(status.backupKey);
      toast.success('Backup key copied to clipboard');
    }
  };

  interface SecurityItem {
    severity: string;
    message: string;
    location: string;
    suggestion: string;
  }

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return 'text-red-600 bg-red-50 border-red-200';
      case 'high': return 'text-orange-600 bg-orange-50 border-orange-200';
      case 'medium': return 'text-yellow-600 bg-yellow-50 border-yellow-200';
      case 'low': return 'text-blue-600 bg-blue-50 border-blue-200';
      default: return 'text-gray-600 bg-gray-50 border-gray-200';
    }
  };

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'critical': return <AlertTriangle className="w-4 h-4" />;
      case 'high': return <AlertTriangle className="w-4 h-4" />;
      case 'medium': return <AlertTriangle className="w-4 h-4" />;
      case 'low': return <Eye className="w-4 h-4" />;
      default: return <Eye className="w-4 h-4" />;
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Shield className="w-6 h-6 text-gray-900 dark:text-gray-100" />
          <div>
            <h2 className="text-xl font-semibold">Encryption Settings</h2>
            <p className="text-sm text-muted-foreground">
              Secure your workspace with end-to-end encryption
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant={status.enabled ? "default" : "secondary"}>
            {status.enabled ? <Lock className="w-3 h-3 mr-1" /> : <Unlock className="w-3 h-3 mr-1" />}
            {status.enabled ? 'Encrypted' : 'Not Encrypted'}
          </Badge>
        </div>
      </div>

      {/* Status Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Key className="w-4 h-4" />
              Master Key
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              {status.masterKeySet ? (
                <CheckCircle className="w-4 h-4 text-green-500" />
              ) : (
                <AlertTriangle className="w-4 h-4 text-yellow-500" />
              )}
              <span className="text-sm">
                {status.masterKeySet ? 'Configured' : 'Not Set'}
              </span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Database className="w-4 h-4" />
              Workspace Key
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              {status.workspaceKeySet ? (
                <CheckCircle className="w-4 h-4 text-green-500" />
              ) : (
                <AlertTriangle className="w-4 h-4 text-yellow-500" />
              )}
              <span className="text-sm">
                {status.workspaceKeySet ? 'Generated' : 'Not Generated'}
              </span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Settings className="w-4 h-4" />
              Auto Encryption
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              {config.enabled ? (
                <CheckCircle className="w-4 h-4 text-green-500" />
              ) : (
                <AlertTriangle className="w-4 h-4 text-yellow-500" />
              )}
              <span className="text-sm">
                {config.enabled ? 'Enabled' : 'Disabled'}
              </span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main Content Tabs */}
      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="export">Export/Import</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Workspace Encryption Overview</CardTitle>
              <CardDescription>
                Manage encryption settings for workspace "{workspaceName}"
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {!status.enabled ? (
                <Alert>
                  <AlertTriangle className="h-4 w-4" />
                  <AlertTitle>Encryption Not Enabled</AlertTitle>
                  <AlertDescription>
                    This workspace is not encrypted. Enable encryption to protect sensitive data
                    like API keys, passwords, and tokens.
                  </AlertDescription>
                </Alert>
              ) : (
                <Alert className="border-green-200 bg-green-50">
                  <CheckCircle className="h-4 w-4 text-green-600" />
                  <AlertTitle className="text-green-800">Encryption Enabled</AlertTitle>
                  <AlertDescription className="text-green-700">
                    This workspace is protected with end-to-end encryption.
                    Sensitive data is automatically encrypted.
                  </AlertDescription>
                </Alert>
              )}

              <div className="flex gap-2">
                {!status.enabled ? (
                  <Button onClick={enableEncryption} disabled={loading}>
                    <Lock className="w-4 h-4 mr-2" />
                    {loading ? 'Enabling...' : 'Enable Encryption'}
                  </Button>
                ) : (
                  <Button variant="outline" onClick={disableEncryption} disabled={loading}>
                    <Unlock className="w-4 h-4 mr-2" />
                    {loading ? 'Disabling...' : 'Disable Encryption'}
                  </Button>
                )}

                <Button variant="outline" onClick={runSecurityCheck} disabled={loading}>
                  <Shield className="w-4 h-4 mr-2" />
                  {loading ? 'Checking...' : 'Security Check'}
                </Button>
              </div>

              {status.backupKey && (
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Backup Key</Label>
                  <div className="flex items-center gap-2 p-3 bg-muted rounded-lg">
                    <code className="flex-1 font-mono text-sm">
                      {showBackupKey ? status.backupKey : '••••••••-••••••••-••••••••-••••••••-••••••••-••••••••-••••••••-••••••••'}
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setShowBackupKey(!showBackupKey)}
                    >
                      {showBackupKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                    </Button>
                    <Button variant="ghost" size="sm" onClick={copyBackupKey}>
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    ⚠️ Keep this backup key safe! You'll need it to access encrypted data on other devices.
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="security" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Security Analysis</CardTitle>
              <CardDescription>
                Check for unencrypted sensitive data in your workspace
              </CardDescription>
            </CardHeader>
            <CardContent>
              {securityCheck ? (
                <div className="space-y-4">
                  {securityCheck.isSecure ? (
                    <Alert className="border-green-200 bg-green-50">
                      <CheckCircle className="h-4 w-4 text-green-600" />
                      <AlertTitle className="text-green-800">Security Check Passed</AlertTitle>
                      <AlertDescription className="text-green-700">
                        No security issues found in this workspace.
                      </AlertDescription>
                    </Alert>
                  ) : (
                    <>
                      {/* Warnings */}
                      {securityCheck.warnings && securityCheck.warnings.length > 0 && (
                        <div className="space-y-2">
                          <h4 className="font-medium text-orange-800">Warnings ({securityCheck.warnings.length})</h4>
                          <div className="space-y-2">
                            {(securityCheck.warnings as SecurityItem[]).map((warning, index: number) => (
                              <div key={index} className={`p-3 border rounded-lg ${getSeverityColor(warning.severity)}`}>
                                <div className="flex items-start gap-2">
                                  {getSeverityIcon(warning.severity)}
                                  <div className="flex-1">
                                    <div className="font-medium">{warning.message}</div>
                                    <div className="text-sm opacity-75">Location: {warning.location}</div>
                                    <div className="text-sm mt-1">💡 {warning.suggestion}</div>
                                  </div>
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {/* Errors */}
                      {securityCheck.errors && securityCheck.errors.length > 0 && (
                        <div className="space-y-2">
                          <h4 className="font-medium text-red-800">Errors ({securityCheck.errors.length})</h4>
                          <div className="space-y-2">
                            {(securityCheck.errors as SecurityItem[]).map((error, index: number) => (
                              <div key={index} className={`p-3 border rounded-lg ${getSeverityColor(error.severity)}`}>
                                <div className="flex items-start gap-2">
                                  {getSeverityIcon(error.severity)}
                                  <div className="flex-1">
                                    <div className="font-medium">{error.message}</div>
                                    <div className="text-sm opacity-75">Location: {error.location}</div>
                                    <div className="text-sm mt-1">💡 {error.suggestion}</div>
                                  </div>
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {/* Recommendations */}
                      {securityCheck.recommendations && securityCheck.recommendations.length > 0 && (
                        <div className="space-y-2">
                          <h4 className="font-medium">Recommendations</h4>
                          <ul className="list-disc list-inside space-y-1 text-sm">
                            {(securityCheck.recommendations as string[]).map((rec, index: number) => (
                              <li key={index}>{rec}</li>
                            ))}
                          </ul>
                        </div>
                      )}
                    </>
                  )}

                  <Button variant="outline" onClick={runSecurityCheck} disabled={loading}>
                    <RefreshCw className="w-4 h-4 mr-2" />
                    {loading ? 'Running...' : 'Re-run Security Check'}
                  </Button>
                </div>
              ) : (
                <div className="text-center py-8">
                  <Shield className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
                  <h3 className="text-lg font-medium mb-2">No Security Check Run</h3>
                  <p className="text-muted-foreground mb-4">
                    Run a security check to identify unencrypted sensitive data in your workspace.
                  </p>
                  <Button onClick={runSecurityCheck} disabled={loading}>
                    <Shield className="w-4 h-4 mr-2" />
                    {loading ? 'Running...' : 'Run Security Check'}
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="export" className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Export */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Download className="w-4 h-4" />
                  Export Encrypted
                </CardTitle>
                <CardDescription>
                  Export workspace with encryption for secure sharing
                </CardDescription>
              </CardHeader>
              <CardContent>
                <Dialog open={exportDialogOpen} onOpenChange={setExportDialogOpen}>
                  <DialogTrigger asChild>
                    <Button asChild className="w-full" disabled={!status.enabled}>
                      <div>
                        <Download className="w-4 h-4 mr-2" />
                        Export Encrypted Workspace
                      </div>
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Export Encrypted Workspace</DialogTitle>
                      <DialogDescription>
                        Export workspace "{workspaceName}" with encryption applied.
                        You'll need the backup key to import on other devices.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                      <div>
                        <Label htmlFor="export-path">Export Path</Label>
                        <Input
                          id="export-path"
                          value={exportPath}
                          onChange={(e) => setExportPath(e.target.value)}
                          placeholder="/path/to/workspace.enc"
                        />
                      </div>
                    </div>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setExportDialogOpen(false)}>
                        Cancel
                      </Button>
                      <Button onClick={exportEncrypted} disabled={!exportPath.trim() || loading}>
                        {loading ? 'Exporting...' : 'Export'}
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>
              </CardContent>
            </Card>

            {/* Import */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Upload className="w-4 h-4" />
                  Import Encrypted
                </CardTitle>
                <CardDescription>
                  Import encrypted workspace from another device
                </CardDescription>
              </CardHeader>
              <CardContent>
                <Dialog open={importDialogOpen} onOpenChange={setImportDialogOpen}>
                  <DialogTrigger asChild>
                    <Button asChild variant="outline" className="w-full">
                      <div>
                        <Upload className="w-4 h-4 mr-2" />
                        Import Encrypted Workspace
                      </div>
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Import Encrypted Workspace</DialogTitle>
                      <DialogDescription>
                        Import an encrypted workspace. You'll need the backup key from the original device.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                      <div>
                        <Label htmlFor="import-path">Import Path</Label>
                        <Input
                          id="import-path"
                          value={importPath}
                          onChange={(e) => setImportPath(e.target.value)}
                          placeholder="/path/to/workspace.enc"
                        />
                      </div>
                      <div>
                        <Label htmlFor="backup-key">Backup Key</Label>
                        <Input
                          id="backup-key"
                          value={importBackupKey}
                          onChange={(e) => setImportBackupKey(e.target.value)}
                          placeholder="YKV2DK-HT1MD0-8EB48W-..."
                        />
                      </div>
                    </div>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setImportDialogOpen(false)}>
                        Cancel
                      </Button>
                      <Button
                        onClick={importEncrypted}
                        disabled={!importPath.trim() || !importBackupKey.trim() || loading}
                      >
                        {loading ? 'Importing...' : 'Import'}
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="settings" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Auto Encryption Settings</CardTitle>
              <CardDescription>
                Configure automatic encryption for sensitive data
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Enable Auto Encryption */}
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label className="text-base">Enable Auto Encryption</Label>
                  <div className="text-sm text-muted-foreground">
                    Automatically encrypt sensitive data when detected
                  </div>
                </div>
                <Switch
                  checked={config.enabled}
                  onCheckedChange={(enabled) => setConfig({ ...config, enabled })}
                  disabled={!status.enabled}
                />
              </div>

              {/* Sensitive Headers */}
              <div className="space-y-2">
                <Label className="text-base">Sensitive Headers</Label>
                <div className="text-sm text-muted-foreground mb-2">
                  Headers that contain sensitive data and should be automatically encrypted
                </div>
                <div className="flex flex-wrap gap-2">
                  {config.sensitiveHeaders?.map((header, index: number) => (
                    <Badge key={index} variant="secondary">
                      {header}
                    </Badge>
                  ))}
                </div>
                <Textarea
                  value={config.sensitiveHeaders?.join('\n') || ''}
                  onChange={(e) => setConfig({
                    ...config,
                    sensitiveHeaders: e.target.value.split('\n').filter(h => h.trim())
                  })}
                  placeholder="authorization&#10;x-api-key&#10;x-auth-token&#10;cookie"
                  className="font-mono text-sm"
                />
              </div>

              {/* Sensitive Fields */}
              <div className="space-y-2">
                <Label className="text-base">Sensitive JSON Fields</Label>
                <div className="text-sm text-muted-foreground mb-2">
                  JSON field names that contain sensitive data
                </div>
                <div className="flex flex-wrap gap-2">
                  {config.sensitiveFields?.map((field, index: number) => (
                    <Badge key={index} variant="secondary">
                      {field}
                    </Badge>
                  ))}
                </div>
                <Textarea
                  value={config.sensitiveFields?.join('\n') || ''}
                  onChange={(e) => setConfig({
                    ...config,
                    sensitiveFields: e.target.value.split('\n').filter(f => f.trim())
                  })}
                  placeholder="password&#10;token&#10;secret&#10;key&#10;credentials"
                  className="font-mono text-sm"
                />
              </div>

              {/* Sensitive Environment Variables */}
              <div className="space-y-2">
                <Label className="text-base">Sensitive Environment Variables</Label>
                <div className="text-sm text-muted-foreground mb-2">
                  Environment variable names that contain sensitive data
                </div>
                <div className="flex flex-wrap gap-2">
                  {config.sensitiveEnvVars?.map((env, index: number) => (
                    <Badge key={index} variant="secondary">
                      {env}
                    </Badge>
                  ))}
                </div>
                <Textarea
                  value={config.sensitiveEnvVars?.join('\n') || ''}
                  onChange={(e) => setConfig({
                    ...config,
                    sensitiveEnvVars: e.target.value.split('\n').filter(e => e.trim())
                  })}
                  placeholder="API_KEY&#10;SECRET_KEY&#10;PASSWORD&#10;TOKEN&#10;DATABASE_URL"
                  className="font-mono text-sm"
                />
              </div>

              {/* Custom Patterns */}
              <div className="space-y-2">
                <Label className="text-base">Custom Patterns (Regex)</Label>
                <div className="text-sm text-muted-foreground mb-2">
                  Regular expressions to detect additional sensitive patterns
                </div>
                <Textarea
                  value={config.customPatterns?.join('\n') || ''}
                  onChange={(e) => setConfig({
                    ...config,
                    customPatterns: e.target.value.split('\n').filter(p => p.trim())
                  })}
                  placeholder="\\b\\d{4}[\\-\\s]?\\d{4}[\\-\\s]?\\d{4}[\\-\\s]?\\d{4}\\b&#10;\\b\\d{3}[\\-\\s]?\\d{2}[\\-\\s]?\\d{4}\\b"
                  className="font-mono text-sm"
                />
              </div>

              <div className="flex justify-end">
                <Button onClick={updateConfig} disabled={loading}>
                  <Settings className="w-4 h-4 mr-2" />
                  {loading ? 'Saving...' : 'Save Settings'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default EncryptionSettings;
