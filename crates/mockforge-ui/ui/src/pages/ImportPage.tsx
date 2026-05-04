import { logger } from '@/utils/logger';
import React, { useState, useCallback } from 'react';
import { Upload, FileText, Code, Globe, AlertTriangle, CheckCircle, XCircle, Eye, Download, History, Trash2, Clock, File } from 'lucide-react';
import {
  useImportPostman,
  useImportInsomnia,
  useImportCurl,
  usePreviewImport,
  useImportHistory,
  useClearImportHistory,
} from '../hooks/useApi';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import { authenticatedFetch } from '../utils/apiClient';
import {
  PageHeader,
  Section,
  Alert,
  Button,
  Card,
  Badge,
  EmptyState,
} from '../components/ui/DesignSystem';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '../components/ui/Tabs';
import { toast } from 'sonner';
import type { ImportRequest, ImportResponse, ImportHistoryEntry } from '../services/api';
import type { ImportRoute } from '../types';

// Import format types
type ImportFormat = 'postman' | 'insomnia' | 'curl';
type TabType = ImportFormat | 'history';

/**
 * Cloud-mode preview dispatcher. Translates the local ImportRequest
 * shape into the cloud `{format, data, base_url, environment}` shape
 * and POSTs to /api/v1/import/preview. Cloud's PreviewResponse is
 * already shape-compatible with ImportResponse so no response mapping
 * needed.
 */
async function cloudPreview(
  format: ImportFormat,
  request: ImportRequest,
): Promise<ImportResponse> {
  const body = {
    format,
    data: request.content,
    base_url: request.base_url,
    environment: request.environment,
  };
  const resp = await authenticatedFetch('/api/v1/import/preview', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(`Cloud preview failed: ${resp.status} ${text}`);
  }
  return (await resp.json()) as ImportResponse;
}

/**
 * Cloud-mode import dispatcher. Cloud route is workspace-scoped, so
 * the caller must provide an active workspace id. Cloud's response
 * shape is {success, imported, warnings} (no per-route detail) — we
 * adapt to ImportResponse by leaving routes empty.
 */
async function cloudImport(
  workspaceId: string,
  format: ImportFormat,
  request: ImportRequest,
  selectedRoutes: number[] | undefined,
): Promise<ImportResponse> {
  const body = {
    format,
    data: request.content,
    base_url: request.base_url,
    environment: request.environment,
    selected_routes: selectedRoutes,
    create_folders: false,
  };
  const resp = await authenticatedFetch(
    `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/import`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    },
  );
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(`Cloud import failed: ${resp.status} ${text}`);
  }
  const cloud = (await resp.json()) as { success: boolean; imported: number; warnings: string[] };
  return {
    success: cloud.success,
    routes: [],
    warnings: cloud.warnings,
  };
}

interface FileUploadProps {
  onFileSelect: (content: string, filename: string) => void;
  format: ImportFormat;
}

function FileUpload({ onFileSelect, format }: FileUploadProps) {
  const [isDragOver, setIsDragOver] = useState(false);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleFile = useCallback((file: File) => {
    if (file.size > 10 * 1024 * 1024) {
      toast.error('File too large (max 10 MB)');
      return;
    }
    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      onFileSelect(content, file.name);
      setSelectedFile(file.name);
    };
    reader.readAsText(file);
  }, [onFileSelect]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);

    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) {
      handleFile(files[0]);
    }
  }, [handleFile]);

  const handleFileInput = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      handleFile(files[0]);
    }
  }, [handleFile]);


  const getFormatIcon = () => {
    switch (format) {
      case 'postman':
        return <FileText className="h-8 w-8 text-orange-500" />;
      case 'insomnia':
        return <Globe className="h-8 w-8 text-purple-500" />;
      case 'curl':
        return <Code className="h-8 w-8 text-success-500" />;
      default:
        return <Upload className="h-8 w-8 text-muted-foreground" />;
    }
  };

  const getFormatName = () => {
    switch (format) {
      case 'postman':
        return 'Postman Collection';
      case 'insomnia':
        return 'Insomnia Export';
      case 'curl':
        return 'cURL Commands';
      default:
        return 'File';
    }
  };

  const getAcceptedTypes = () => {
    switch (format) {
      case 'postman':
        return '.json,.postman_collection';
      case 'insomnia':
        return '.json,.insomnia';
      case 'curl':
        return '.txt,.sh,.curl';
      default:
        return '*';
    }
  };

  return (
    <div
      className={`border-2 border-dashed rounded-lg p-8 text-center transition-colors ${
        isDragOver
          ? 'border-primary bg-primary/5'
          : 'border-border hover:border-primary/50'
      }`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="flex flex-col items-center space-y-4">
        {getFormatIcon()}
        <div>
          <p className="text-lg font-medium text-foreground">
            {selectedFile ? `Selected: ${selectedFile}` : `Drop ${getFormatName()} here`}
          </p>
          <p className="text-sm text-muted-foreground mt-1">
            or click to browse files
          </p>
        </div>
        <input
          type="file"
          accept={getAcceptedTypes()}
          onChange={handleFileInput}
          className="hidden"
          id={`file-upload-${format}`}
        />
        <label htmlFor={`file-upload-${format}`} style={{ cursor: 'pointer' }}>
          <Button variant="outline" type="button">
            Choose File
          </Button>
        </label>
      </div>
    </div>
  );
}

interface RoutePreviewProps {
  routes: ImportRoute[];
  onToggleRoute: (index: number) => void;
  selectedRoutes: Set<number>;
}

function RoutePreview({ routes, onToggleRoute, selectedRoutes }: RoutePreviewProps) {
  // Defensive: upstream API/parsers may hand us a non-array under
  // degraded conditions — don't crash the whole page.
  if (!Array.isArray(routes) || routes.length === 0) return null;
  if (routes.length === 0) {
    return (
      <div className="text-center py-8">
        <p className="text-muted-foreground">No routes found to preview</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-medium">Generated Routes ({routes.length})</h3>
        <div className="flex space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              // Select all routes that aren't already selected
              routes.forEach((_, index) => {
                if (!selectedRoutes.has(index)) {
                  onToggleRoute(index);
                }
              });
            }}
          >
            Select All
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              // Deselect all
              selectedRoutes.forEach(index => onToggleRoute(index));
            }}
          >
            Deselect All
          </Button>
        </div>
      </div>

      <div className="space-y-3 max-h-96 overflow-y-auto">
        {routes.map((route, index) => (
          <Card key={index} className="p-4">
            <div className="flex items-start space-x-4">
              <input
                type="checkbox"
                checked={selectedRoutes.has(index)}
                onChange={() => onToggleRoute(index)}
                className="mt-1"
              />
              <div className="flex-1">
                <div className="flex items-center space-x-2 mb-2">
                  <Badge variant={
                    route.method === 'GET' ? 'success' :
                    route.method === 'POST' ? 'info' :
                    route.method === 'PUT' ? 'warning' :
                    route.method === 'DELETE' ? 'error' : 'default'
                  }>
                    {route.method}
                  </Badge>
                  <code className="text-sm bg-muted px-2 py-1 rounded">
                    {route.path}
                  </code>
                </div>

                {route.response && (
                  <div className="text-sm text-muted-foreground">
                    Response: {route.response.status} {
                      route.response.status >= 200 && route.response.status < 300 ? '✅' :
                      route.response.status >= 400 ? '❌' : '⚠️'
                    }
                  </div>
                )}

                {route.body && (
                  <div className="mt-2">
                    <details className="text-sm">
                      <summary className="cursor-pointer text-muted-foreground">
                        Request Body ({route.body.length} chars)
                      </summary>
                      <pre className="mt-2 bg-muted p-2 rounded text-xs overflow-x-auto">
                        {route.body}
                      </pre>
                    </details>
                  </div>
                )}
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}

function ImportHistory({ onHistoryEntryClick }: { onHistoryEntryClick?: (entry: ImportHistoryEntry) => void }) {
  const { data: history, isLoading, error } = useImportHistory();
  const clearHistory = useClearImportHistory();

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleString();
  };

  const getFormatIcon = (format: string) => {
    switch (format.toLowerCase()) {
      case 'postman':
        return <FileText className="h-4 w-4 text-orange-500" />;
      case 'insomnia':
        return <Globe className="h-4 w-4 text-purple-500" />;
      case 'curl':
        return <Code className="h-4 w-4 text-success-500" />;
      default:
        return <File className="h-4 w-4 text-muted-foreground" />;
    }
  };

  if (isLoading) {
    return (
      <div className="text-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
        <p className="mt-2 text-muted-foreground">Loading import history...</p>
      </div>
    );
  }

  if (error) {
    return (
      <Alert type="error">
        <XCircle className="h-4 w-4" />
        Failed to load import history: {error.message}
      </Alert>
    );
  }

  // The history endpoint can return a degraded payload (e.g. `{}` or
  // `{ imports: null }`) under rate limiting, so normalize defensively
  // before rendering — otherwise `.length` / `.map` crash the page and
  // trip the global ErrorBoundary.
  const imports = Array.isArray(history?.imports) ? history!.imports : [];
  const total = typeof history?.total === 'number' ? history!.total : imports.length;

  if (!history || imports.length === 0) {
    return (
      <EmptyState
        icon={<History className="h-8 w-8" />}
        title="No Import History"
        description="Your import history will appear here after you import collections."
      />
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-medium">Import History ({total})</h3>
        <Button
          variant="outline"
          size="sm"
          onClick={() => clearHistory.mutate()}
          loading={clearHistory.isPending}
        >
          <Trash2 className="h-4 w-4 mr-2" />
          Clear History
        </Button>
      </div>

      <div className="space-y-3 max-h-96 overflow-y-auto">
        {imports.map((entry) => (
          <Card key={entry.id} className="p-4">
            <div className="flex items-start justify-between">
              <div className="flex items-start space-x-3 flex-1">
                <div className="mt-1">
                  {entry.success ? (
                    <CheckCircle className="h-5 w-5 text-success-500" />
                  ) : (
                    <XCircle className="h-5 w-5 text-danger-500" />
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center space-x-2 mb-2">
                    {getFormatIcon(entry.format)}
                    <span className="font-medium capitalize">{entry.format}</span>
                    <Badge variant={entry.success ? 'success' : 'error'}>
                      {entry.success ? 'Success' : 'Failed'}
                    </Badge>
                  </div>

                  <div className="flex items-center space-x-2 text-sm text-muted-foreground mb-2">
                    <File className="h-3 w-3" />
                    <span className="truncate">{entry.filename}</span>
                    <Clock className="h-3 w-3 ml-2" />
                    <span>{formatTimestamp(entry.timestamp)}</span>
                  </div>

                  <div className="flex items-center space-x-4 text-sm">
                    <span>Routes: {entry.routes_count}</span>
                    {(entry.variables_count ?? 0) > 0 && (
                      <span>Variables: {entry.variables_count}</span>
                    )}
                    {(entry.warnings_count ?? 0) > 0 && (
                      <span className="text-warning-600 dark:text-warning-400">
                        Warnings: {entry.warnings_count}
                      </span>
                    )}
                  </div>

                  {entry.environment && (
                    <div className="text-sm text-muted-foreground mt-1">
                      Environment: {entry.environment}
                    </div>
                  )}

                  {entry.base_url && (
                    <div className="text-sm text-muted-foreground mt-1">
                      Base URL: {entry.base_url}
                    </div>
                  )}

                  {entry.error_message && (
                    <div className="text-sm text-danger-600 dark:text-danger-400 mt-2 bg-danger-50 dark:bg-danger-900/20 p-2 rounded">
                      Error: {entry.error_message}
                    </div>
                  )}
                </div>
              </div>

              {onHistoryEntryClick && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onHistoryEntryClick(entry)}
                >
                  View Details
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}

export function ImportPage() {
  const [activeTab, setActiveTab] = useState<TabType>('postman');
  const [fileContent, setFileContent] = useState<string>('');
  const [filename, setFilename] = useState<string>('');
  const [environment, setEnvironment] = useState<string>('');
  const [baseUrl, setBaseUrl] = useState<string>('');
  const [previewResult, setPreviewResult] = useState<ImportResponse | null>(null);
  const [selectedRoutes, setSelectedRoutes] = useState<Set<number>>(new Set());

  const previewImport = usePreviewImport();
  const importPostman = useImportPostman();
  const importInsomnia = useImportInsomnia();
  const importCurl = useImportCurl();
  const activeWorkspace = useWorkspaceStore(state => state.activeWorkspace);
  const cloud = isCloudMode();

  const handleFileSelect = useCallback((content: string, fileName: string) => {
    setFileContent(content);
    setFilename(fileName);
    setPreviewResult(null);
    setSelectedRoutes(new Set());
  }, []);

  const handlePreview = async () => {
    if (!fileContent) return;

    const request: ImportRequest = {
      content: fileContent,
      filename,
      environment: environment || undefined,
      base_url: baseUrl || undefined,
    };

    try {
      const result = cloud
        ? await cloudPreview(activeTab as ImportFormat, request)
        : await previewImport.mutateAsync(request);
      setPreviewResult(result);

      // Auto-select all routes by default
      if (result.routes) {
        setSelectedRoutes(new Set(result.routes.map((_, index) => index)));
      }
    } catch (error) {
      logger.error('Preview failed', error);
      toast.error(error instanceof Error ? error.message : 'Preview failed');
    }
  };

  const handleImport = async () => {
    if (!fileContent || !previewResult?.routes) return;

    const request: ImportRequest = {
      content: fileContent,
      filename,
      environment: environment || undefined,
      base_url: baseUrl || undefined,
    };

    try {
      let result: ImportResponse;

      if (cloud) {
        if (!activeWorkspace) {
          toast.error('Select an active workspace before importing in cloud mode');
          return;
        }
        result = await cloudImport(
          activeWorkspace.id,
          activeTab as ImportFormat,
          request,
          Array.from(selectedRoutes),
        );
      } else {
        switch (activeTab) {
          case 'postman':
            result = await importPostman.mutateAsync(request);
            break;
          case 'insomnia':
            result = await importInsomnia.mutateAsync(request);
            break;
          case 'curl':
            result = await importCurl.mutateAsync(request);
            break;
          default:
            return;
        }
      }

      if (result.success) {
        toast?.success(`Successfully imported ${selectedRoutes.size} routes!`);
        // Reset form
        setFileContent('');
        setFilename('');
        setPreviewResult(null);
        setSelectedRoutes(new Set());
      } else {
        toast?.error(`Import failed: ${result.error}`);
      }
    } catch (error) {
      logger.error('Import failed',error);
      toast?.error('Import failed. Please check the console for details.');
    }
  };

  const handleToggleRoute = (index: number) => {
    const newSelected = new Set(selectedRoutes);
    if (newSelected.has(index)) {
      newSelected.delete(index);
    } else {
      newSelected.add(index);
    }
    setSelectedRoutes(newSelected);
  };

  const isPreviewDisabled = !fileContent || previewImport.isPending;
  const isImportDisabled = !previewResult?.success || selectedRoutes.size === 0 ||
    importPostman.isPending || importInsomnia.isPending || importCurl.isPending;

  return (
    <div className="space-y-8">
      <PageHeader
        title="Import API Collections"
        subtitle="Import routes from Postman, Insomnia, or cURL commands"
      />

      <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as TabType)}>
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="postman" className="flex items-center space-x-2">
            <FileText className="h-4 w-4" />
            <span>Postman</span>
          </TabsTrigger>
          <TabsTrigger value="insomnia" className="flex items-center space-x-2">
            <Globe className="h-4 w-4" />
            <span>Insomnia</span>
          </TabsTrigger>
          <TabsTrigger value="curl" className="flex items-center space-x-2">
            <Code className="h-4 w-4" />
            <span>cURL</span>
          </TabsTrigger>
          <TabsTrigger value="history" className="flex items-center space-x-2">
            <History className="h-4 w-4" />
            <span>History</span>
          </TabsTrigger>
        </TabsList>

        {(activeTab === 'postman' || activeTab === 'insomnia' || activeTab === 'curl') && (
          <TabsContent value={activeTab} className="space-y-6">
            {/* File Upload Section */}
            <Section
              title="Upload File"
              subtitle={`Upload your ${activeTab} collection or export file`}
            >
              <FileUpload onFileSelect={handleFileSelect} format={activeTab} />
            </Section>

            {/* Configuration Section */}
            {(activeTab === 'insomnia' || activeTab === 'postman') && (
            <Section
              title="Configuration"
              subtitle="Optional settings for import processing"
            >
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {activeTab === 'insomnia' && (
                  <div>
                    <label className="block text-sm font-medium mb-2">
                      Environment (optional)
                    </label>
                    <input
                      type="text"
                      value={environment}
                      onChange={(e) => setEnvironment(e.target.value)}
                      placeholder="e.g., dev, staging, prod"
                      className="w-full px-3 py-2 border border-border rounded-md bg-card"
                    />
                  </div>
                )}
                <div>
                  <label className="block text-sm font-medium mb-2">
                    Base URL Override (optional)
                  </label>
                  <input
                    type="text"
                    value={baseUrl}
                    onChange={(e) => setBaseUrl(e.target.value)}
                    placeholder="e.g., https://api.example.com"
                    className="w-full px-3 py-2 border border-border rounded-md bg-card"
                  />
                </div>
              </div>
            </Section>
          )}

          {/* Preview Section */}
          <Section
            title="Preview Import"
            subtitle="Review the routes that will be generated before importing"
          >
            <div className="space-y-4">
              <Button
                onClick={handlePreview}
                disabled={isPreviewDisabled}
                loading={previewImport.isPending}
              >
                <Eye className="h-4 w-4 mr-2" />
                Preview Routes
              </Button>

              {previewResult && (
                <div className="space-y-4">
                  {previewResult.success ? (
                    <Alert type="success">
                      <CheckCircle className="h-4 w-4" />
                      Successfully parsed {previewResult.routes?.length || 0} routes
                    </Alert>
                  ) : (
                    <Alert type="error">
                      <XCircle className="h-4 w-4" />
                      Preview failed: {previewResult.error}
                    </Alert>
                  )}

                  {previewResult.warnings && previewResult.warnings.length > 0 && (
                    <Alert type="warning">
                      <AlertTriangle className="h-4 w-4" />
                      <div>
                        <p className="font-medium">Warnings:</p>
                        <ul className="list-disc list-inside mt-1">
                          {previewResult.warnings.map((warning, index) => (
                            <li key={index} className="text-sm">{warning}</li>
                          ))}
                        </ul>
                      </div>
                    </Alert>
                  )}

                  {previewResult.routes && (
                    <RoutePreview
                      routes={previewResult.routes}
                      onToggleRoute={handleToggleRoute}
                      selectedRoutes={selectedRoutes}
                    />
                  )}
                </div>
              )}
            </div>
          </Section>

          {/* Import Section */}
          <Section
            title="Import Routes"
            subtitle="Import the selected routes into MockForge"
          >
            <div className="flex items-center space-x-4">
              <Button
                onClick={handleImport}
                disabled={isImportDisabled}
                loading={importPostman.isPending || importInsomnia.isPending || importCurl.isPending}
              >
                <Download className="h-4 w-4 mr-2" />
                Import {selectedRoutes.size} Route{selectedRoutes.size !== 1 ? 's' : ''}
              </Button>

              {selectedRoutes.size > 0 && (
                <p className="text-sm text-muted-foreground">
                  {selectedRoutes.size} of {previewResult?.routes?.length || 0} routes selected
                </p>
              )}
            </div>
          </Section>
        </TabsContent>
        )}

        <TabsContent value="history" className="space-y-6">
          <Section
            title="Import History"
            subtitle="View and manage your previous import activities"
          >
            <ImportHistory />
          </Section>
        </TabsContent>
      </Tabs>
    </div>
  );
}
