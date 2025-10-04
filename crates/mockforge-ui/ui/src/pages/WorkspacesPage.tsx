import React, { useState, useEffect } from 'react';
import { apiService, importApi } from '../services/api';
import { useUpdateWorkspacesOrder } from '../hooks/useApi';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import type {
  WorkspaceSummary,
  WorkspaceDetail,
  FolderDetail,
  CreateWorkspaceRequest,
  CreateFolderRequest,
  CreateRequestRequest,
  ImportToWorkspaceRequest,
  ImportResponse,
  ImportHistoryEntry
} from '../types';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../components/ui/Card';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { Textarea } from '../components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from '../components/ui/Dialog';
import { Badge } from '../components/ui/Badge';
import { Alert } from '../components/ui/DesignSystem';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/Tabs';
import { Folder, FolderOpen, FileText, Plus, Upload, Settings, Trash2, History, Play, Shield, GripVertical, AlertTriangle } from 'lucide-react';
import { Checkbox } from '../components/ui/DesignSystem';
import { toast } from 'sonner';
import ResponseHistory from '../components/workspace/ResponseHistory';
import EncryptionSettings from '../components/workspace/EncryptionSettings';
import { getErrorDetails, logError, sanitizeInput, validateFile } from '../utils/errorHandling';

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface WorkspacesPageProps {}

const WorkspacesPage: React.FC<WorkspacesPageProps> = () => {
  const { workspaces, loading, error, setActiveWorkspaceById } = useWorkspaceStore();
  const [selectedWorkspace, setSelectedWorkspace] = useState<WorkspaceDetail | null>(null);
  const [selectedFolder, setSelectedFolder] = useState<FolderDetail | null>(null);

  // Dialog states
  const [createWorkspaceOpen, setCreateWorkspaceOpen] = useState(false);
  const [openFromDirectoryOpen, setOpenFromDirectoryOpen] = useState(false);
  const [createFolderOpen, setCreateFolderOpen] = useState(false);
  const [createRequestOpen, setCreateRequestOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [importPreviewOpen, setImportPreviewOpen] = useState(false);
  const [importPreviewData, setImportPreviewData] = useState<ImportResponse | null>(null);
  const [selectedRoutes, setSelectedRoutes] = useState<Set<number>>(new Set());
  const [importHistory, setImportHistory] = useState<ImportHistoryEntry[]>([]);

  const [historyDialogOpen, setHistoryDialogOpen] = useState(false);
  const [encryptionSettingsOpen, setEncryptionSettingsOpen] = useState(false);
  const [selectedRequestForHistory, setSelectedRequestForHistory] = useState<{ id: string; name: string } | null>(null);
  const [draggedWorkspace, setDraggedWorkspace] = useState<string | null>(null);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [workspaceToDelete, setWorkspaceToDelete] = useState<string | null>(null);

  // Form states
  const [newWorkspace, setNewWorkspace] = useState<CreateWorkspaceRequest>({
    name: '',
    description: '',
  });

  const [enableSync, setEnableSync] = useState(false);
  const [syncDirectory, setSyncDirectory] = useState('');

  const [openFromDirectory, setOpenFromDirectory] = useState({
    directory: '',
  });

  const [newFolder, setNewFolder] = useState<CreateFolderRequest>({
    name: '',
    description: '',
  });

  const [newRequest, setNewRequest] = useState<CreateRequestRequest>({
    name: '',
    method: 'GET',
    path: '',
    status_code: 200,
    response_body: '',
  });

  const [importData, setImportData] = useState<ImportToWorkspaceRequest>({
    format: 'postman',
    data: '',
    create_folders: true,
  });

  const updateWorkspacesOrder = useUpdateWorkspacesOrder();

  // Load import history when import dialog opens
  useEffect(() => {
    if (importOpen) {
      loadImportHistory();
    }
  }, [importOpen]);

  const handleCreateWorkspace = async () => {
    try {
      const response = await apiService.createWorkspace(newWorkspace);
      toast.success('Workspace created successfully');

      // Configure sync if enabled
      if (enableSync && syncDirectory.trim()) {
        try {
          await apiService.configureSync(response.data.id, {
            target_directory: syncDirectory,
            sync_direction: 'Bidirectional',
            realtime_monitoring: true,
          });
          toast.success('Sync configured successfully');
        } catch (syncErr) {
          const syncErrorDetails = getErrorDetails(syncErr);
          toast.error(`Workspace created but sync configuration failed: ${syncErrorDetails.message}`);
          logError(syncErr, 'Sync configuration');
        }
      }

      setCreateWorkspaceOpen(false);
      setNewWorkspace({ name: '', description: '' });
      setEnableSync(false);
      setSyncDirectory('');
      // Refresh workspaces from the store
      const { refreshWorkspaces } = useWorkspaceStore.getState();
      await refreshWorkspaces();
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to create workspace: ${errorDetails.message}`);
      logError(err, 'Create workspace');
    }
  };

  const handleOpenFromDirectory = async () => {
    try {
      // Sanitize directory path
      const sanitizedDir = sanitizeInput(openFromDirectory.directory);
      await apiService.openWorkspaceFromDirectory(sanitizedDir);
      toast.success('Workspace opened from directory successfully');
      setOpenFromDirectoryOpen(false);
      setOpenFromDirectory({ directory: '' });
      // Refresh workspaces from the store
      const { refreshWorkspaces } = useWorkspaceStore.getState();
      await refreshWorkspaces();
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to open workspace: ${errorDetails.message}`);
      logError(err, 'Open workspace from directory');
    }
  };

  const handleDeleteWorkspace = async () => {
    if (!workspaceToDelete) return;

    try {
      await apiService.deleteWorkspace(workspaceToDelete);
      toast.success('Workspace deleted successfully');
      // Refresh workspaces from the store
      const { refreshWorkspaces } = useWorkspaceStore.getState();
      await refreshWorkspaces();
      if (selectedWorkspace?.summary.id === workspaceToDelete) {
        setSelectedWorkspace(null);
      }
      setDeleteConfirmOpen(false);
      setWorkspaceToDelete(null);
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to delete workspace: ${errorDetails.message}`);
      logError(err, 'Delete workspace');
    }
  };

  const confirmDeleteWorkspace = (workspaceId: string) => {
    setWorkspaceToDelete(workspaceId);
    setDeleteConfirmOpen(true);
  };

  const handleSetActiveWorkspace = async (workspaceId: string) => {
    try {
      await setActiveWorkspaceById(workspaceId);
      toast.success('Active workspace updated');
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to set active workspace: ${errorDetails.message}`);
      logError(err, 'Set active workspace');
    }
  };

  const handleWorkspaceClick = async (workspace: WorkspaceSummary) => {
    try {
      const response = await apiService.getWorkspace(workspace.id);
      setSelectedWorkspace(response.workspace);
      setSelectedFolder(null);
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to load workspace: ${errorDetails.message}`);
      logError(err, 'Load workspace details');
    }
  };

  const handleFolderClick = async (folderId: string) => {
    if (!selectedWorkspace) return;

    try {
      const response = await apiService.getFolder(selectedWorkspace.summary.id, folderId);
      setSelectedFolder(response.folder);
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to load folder: ${errorDetails.message}`);
      logError(err, 'Load folder details');
    }
  };

  const handleCreateFolder = async () => {
    if (!selectedWorkspace) return;

    // Validate and sanitize inputs
    if (!newFolder.name.trim()) {
      toast.error('Folder name is required');
      return;
    }

    const sanitizedFolder = {
      name: sanitizeInput(newFolder.name),
      description: newFolder.description ? sanitizeInput(newFolder.description) : undefined,
    };

    try {
      await apiService.createFolder(selectedWorkspace.summary.id, sanitizedFolder);
      toast.success('Folder created successfully');
      setCreateFolderOpen(false);
      setNewFolder({ name: '', description: '' });
      // Reload workspace details
      const response = await apiService.getWorkspace(selectedWorkspace.summary.id);
      setSelectedWorkspace(response.workspace);
    } catch (err) {
      const errorDetails = getErrorDetails(err);
      toast.error(`Failed to create folder: ${errorDetails.message}`);
      logError(err, 'Create folder');
    }
  };

  const handleCreateRequest = async () => {
    if (!selectedWorkspace) return;

    const requestData = {
      ...newRequest,
      folder_id: selectedFolder?.summary.id,
    };

    try {
      await apiService.createRequest(selectedWorkspace.summary.id, requestData);
      toast.success('Request created successfully');
      setCreateRequestOpen(false);
      setNewRequest({
        name: '',
        method: 'GET',
        path: '',
        status_code: 200,
        response_body: '',
      });

      // Reload workspace details
      const response = await apiService.getWorkspace(selectedWorkspace.summary.id);
      setSelectedWorkspace(response.workspace);

      // Reload folder details if we're in a folder
      if (selectedFolder) {
        const folderResponse = await apiService.getFolder(selectedWorkspace.summary.id, selectedFolder.summary.id);
        setSelectedFolder(folderResponse.folder);
      }
    } catch (err) {
      toast.error('Failed to create request');
      console.error('Error creating request:', err);
    }
  };

  const handleViewHistory = (requestId: string, requestName: string) => {
    setSelectedRequestForHistory({ id: requestId, name: requestName });
    setHistoryDialogOpen(true);
  };

  const handleImport = async () => {
    if (!selectedWorkspace) return;

    const importRequest = {
      ...importData,
      folder_id: selectedFolder?.summary.id,
      selected_routes: Array.from(selectedRoutes),
    };

    try {
      await apiService.importToWorkspace(selectedWorkspace.summary.id, importRequest);
      toast.success('Import completed successfully');
      setImportOpen(false);
      setImportPreviewOpen(false);
      setImportData({
        format: 'postman',
        data: '',
        create_folders: true,
      });

      // Reload workspace details
      const workspaceResponse = await apiService.getWorkspace(selectedWorkspace.summary.id);
      setSelectedWorkspace(workspaceResponse.workspace);
    } catch (err) {
      toast.error('Failed to import data');
      console.error('Error importing data:', err);
    }
  };

  const handlePreviewImport = async () => {
    try {
      const previewRequest = {
        ...importData,
        folder_id: selectedFolder?.summary.id,
      };

      const response = await apiService.previewImport(previewRequest);
      setImportPreviewData(response);
      // Select all routes by default
      setSelectedRoutes(new Set(response.routes?.map((_: unknown, index: number) => index) || []));
      setImportOpen(false);
      setImportPreviewOpen(true);
    } catch (err) {
      toast.error('Failed to preview import');
      console.error('Error previewing import:', err);
    }
  };

  const loadImportHistory = async () => {
    try {
      const response = await importApi.getImportHistory();
      setImportHistory(response.imports || []);
    } catch (err) {
      console.error('Failed to load import history:', err);
      // Don't show error toast as this is a background operation
    }
  };

  const handleReimport = (historyEntry: ImportHistoryEntry) => {
    // Re-populate the import form with the data from history
    setImportData({
      format: historyEntry.format,
      data: '', // We don't store the original data, user will need to provide it again
      create_folders: true,
    });
    toast.info(`Format set to ${historyEntry.format}. Please paste or upload the data again.`);
  };

  const handleWorkspaceDragStart = (e: React.DragEvent, workspaceId: string) => {
    setDraggedWorkspace(workspaceId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleWorkspaceDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const handleWorkspaceDrop = async (e: React.DragEvent, targetWorkspaceId: string) => {
    e.preventDefault();

    if (!draggedWorkspace || draggedWorkspace === targetWorkspaceId || !workspaces) {
      setDraggedWorkspace(null);
      return;
    }

    try {
      // Reorder the workspaces array
      const draggedIndex = workspaces.findIndex(ws => ws.id === draggedWorkspace);
      const targetIndex = workspaces.findIndex(ws => ws.id === targetWorkspaceId);

      if (draggedIndex === -1 || targetIndex === -1) {
        setDraggedWorkspace(null);
        return;
      }

      const newWorkspaces = [...workspaces];
      const [draggedWs] = newWorkspaces.splice(draggedIndex, 1);
      newWorkspaces.splice(targetIndex, 0, draggedWs);

      // Update the local state immediately for better UX
      // Note: setWorkspaces is handled by the store

      // Update the order by sending the new order to the API
      const workspaceIds = newWorkspaces.map(ws => ws.id);

      try {
        await updateWorkspacesOrder.mutateAsync(workspaceIds);
        toast.success('Workspace order updated');
      } catch (error) {
        toast.error('Failed to update workspace order');
        throw error;
      }
    } catch {
      toast.error('Failed to update workspace order');
      // Reload workspaces to revert the optimistic update
      const { refreshWorkspaces } = useWorkspaceStore.getState();
      await refreshWorkspaces();
    } finally {
      setDraggedWorkspace(null);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-lg">Loading workspaces...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Workspaces</h1>
          <p className="text-muted-foreground">Manage your mock API workspaces</p>
        </div>
        <div className="flex gap-2">
          <Dialog open={createWorkspaceOpen} onOpenChange={setCreateWorkspaceOpen}>
            <DialogTrigger asChild>
              <Button>
                <Plus className="w-4 h-4 mr-2" />
                New Workspace
              </Button>
            </DialogTrigger>
            <DialogContent className="bg-white dark:bg-gray-900">
              <DialogHeader className="space-y-2">
                <DialogTitle className="text-xl font-semibold text-gray-900 dark:text-gray-100">Create New Workspace</DialogTitle>
                <DialogDescription className="text-sm text-gray-600 dark:text-gray-400 leading-relaxed">
                  Create a new workspace to organize your mock API endpoints.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                <div>
                  <Label htmlFor="workspace-name" className="text-gray-900 dark:text-gray-100">Name</Label>
                  <Input
                    id="workspace-name"
                    value={newWorkspace.name}
                    onChange={(e) => setNewWorkspace({ ...newWorkspace, name: e.target.value })}
                    placeholder="My Workspace"
                    className="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
                  />
                </div>
                <div>
                  <Label htmlFor="workspace-description" className="text-gray-900 dark:text-gray-100">Description</Label>
                  <Textarea
                    id="workspace-description"
                    value={newWorkspace.description}
                    onChange={(e) => setNewWorkspace({ ...newWorkspace, description: e.target.value })}
                    placeholder="Optional description..."
                    className="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
                  />
                </div>
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="enable-sync"
                    checked={enableSync}
                    onCheckedChange={setEnableSync}
                  />
                  <Label htmlFor="enable-sync" className="text-gray-900 dark:text-gray-100">Enable directory sync</Label>
                </div>
                {enableSync && (
                  <div>
                    <Label htmlFor="sync-directory" className="text-gray-900 dark:text-gray-100">Sync Directory</Label>
                    <Input
                      id="sync-directory"
                      value={syncDirectory}
                      onChange={(e) => setSyncDirectory(e.target.value)}
                      placeholder="/path/to/workspace"
                      className="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
                    />
                  </div>
                )}
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setCreateWorkspaceOpen(false)}>
                  Cancel
                </Button>
                <Button onClick={handleCreateWorkspace} disabled={!newWorkspace.name.trim()}>
                  Create Workspace
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={openFromDirectoryOpen} onOpenChange={setOpenFromDirectoryOpen}>
            <DialogTrigger asChild>
              <Button variant="outline">
                <FolderOpen className="w-4 h-4 mr-2" />
                Open from Directory
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Open Workspace from Directory</DialogTitle>
                <DialogDescription>
                  Open an existing workspace from a directory on your system.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                <div>
                  <Label htmlFor="directory-path">Directory Path</Label>
                  <Input
                    id="directory-path"
                    value={openFromDirectory.directory}
                    onChange={(e) => setOpenFromDirectory({ directory: e.target.value })}
                    placeholder="/path/to/workspace"
                  />
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setOpenFromDirectoryOpen(false)}>
                  Cancel
                </Button>
                <Button onClick={handleOpenFromDirectory} disabled={!openFromDirectory.directory.trim()}>
                  Open Workspace
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      {/* Workspaces Grid */}
      {workspaces && workspaces.length === 0 ? (
        <Card className="col-span-full">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <FolderOpen className="w-16 h-16 text-muted-foreground mb-4" />
            <h3 className="text-xl font-semibold mb-2">No Workspaces Yet</h3>
            <p className="text-muted-foreground mb-6 text-center max-w-md">
              Get started by creating a new workspace or opening an existing one from a directory.
            </p>
            <div className="flex gap-2">
              <Button onClick={() => setCreateWorkspaceOpen(true)}>
                <Plus className="w-4 h-4 mr-2" />
                Create Workspace
              </Button>
              <Button variant="outline" onClick={() => setOpenFromDirectoryOpen(true)}>
                <FolderOpen className="w-4 h-4 mr-2" />
                Open from Directory
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {workspaces?.map((workspace) => (
          <Card
            key={workspace.id}
            className={`cursor-pointer transition-all hover:shadow-md ${
              selectedWorkspace?.summary.id === workspace.id ? 'ring-2 ring-primary' : ''
            }`}
            draggable
            onDragStart={(e) => handleWorkspaceDragStart(e, workspace.id)}
            onDragOver={handleWorkspaceDragOver}
            onDrop={(e) => handleWorkspaceDrop(e, workspace.id)}
          >
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <div className="flex items-center space-x-2">
                <GripVertical className="w-4 h-4 text-muted-foreground" />
                <CardTitle className="text-lg">{workspace.name}</CardTitle>
                {workspace.is_active && (
                  <Badge variant="secondary">Active</Badge>
                )}
              </div>
              <div className="flex items-center space-x-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleSetActiveWorkspace(workspace.id);
                  }}
                  disabled={workspace.is_active}
                >
                  <Play className="w-4 h-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    confirmDeleteWorkspace(workspace.id);
                  }}
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent onClick={() => handleWorkspaceClick(workspace)}>
              <CardDescription className="mb-4">
                {workspace.description || 'No description'}
              </CardDescription>
              <div className="flex items-center justify-between text-sm text-muted-foreground">
                <span>{workspace.request_count} requests</span>
                <span>{workspace.folder_count} folders</span>
              </div>
            </CardContent>
          </Card>
        ))}
        </div>
      )}

      {/* Selected Workspace Details */}
      {selectedWorkspace && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>{selectedWorkspace.summary.name}</CardTitle>
                <CardDescription>{selectedWorkspace.summary.description}</CardDescription>
              </div>
              <div className="flex gap-2">
                <Dialog open={createFolderOpen} onOpenChange={setCreateFolderOpen}>
                  <DialogTrigger asChild>
                    <Button variant="outline" size="sm">
                      <Folder className="w-4 h-4 mr-2" />
                      New Folder
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Create New Folder</DialogTitle>
                      <DialogDescription>
                        Create a new folder to organize requests in this workspace.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                      <div>
                        <Label htmlFor="folder-name">Name</Label>
                        <Input
                          id="folder-name"
                          value={newFolder.name}
                          onChange={(e) => setNewFolder({ ...newFolder, name: e.target.value })}
                          placeholder="My Folder"
                        />
                      </div>
                      <div>
                        <Label htmlFor="folder-description">Description</Label>
                        <Textarea
                          id="folder-description"
                          value={newFolder.description}
                          onChange={(e) => setNewFolder({ ...newFolder, description: e.target.value })}
                          placeholder="Optional description..."
                        />
                      </div>
                    </div>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setCreateFolderOpen(false)}>
                        Cancel
                      </Button>
                      <Button onClick={handleCreateFolder} disabled={!newFolder.name.trim()}>
                        Create Folder
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>

                <Dialog open={createRequestOpen} onOpenChange={setCreateRequestOpen}>
                  <DialogTrigger asChild>
                    <Button variant="outline" size="sm">
                      <FileText className="w-4 h-4 mr-2" />
                      New Request
                    </Button>
                  </DialogTrigger>
                  <DialogContent className="max-w-2xl">
                    <DialogHeader>
                      <DialogTitle>Create New Request</DialogTitle>
                      <DialogDescription>
                        Create a new mock request in this workspace.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-4">
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <Label htmlFor="request-name">Name</Label>
                          <Input
                            id="request-name"
                            value={newRequest.name}
                            onChange={(e) => setNewRequest({ ...newRequest, name: e.target.value })}
                            placeholder="My Request"
                          />
                        </div>
                        <div>
                          <Label htmlFor="request-method">Method</Label>
                          <Select
                            value={newRequest.method}
                            onValueChange={(value) => setNewRequest({ ...newRequest, method: value })}
                          >
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="GET">GET</SelectItem>
                              <SelectItem value="POST">POST</SelectItem>
                              <SelectItem value="PUT">PUT</SelectItem>
                              <SelectItem value="DELETE">DELETE</SelectItem>
                              <SelectItem value="PATCH">PATCH</SelectItem>
                              <SelectItem value="HEAD">HEAD</SelectItem>
                              <SelectItem value="OPTIONS">OPTIONS</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                      </div>
                      <div>
                        <Label htmlFor="request-path">Path</Label>
                        <Input
                          id="request-path"
                          value={newRequest.path}
                          onChange={(e) => setNewRequest({ ...newRequest, path: e.target.value })}
                          placeholder="/api/users"
                        />
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <Label htmlFor="request-status">Status Code</Label>
                          <Input
                            id="request-status"
                            type="number"
                            value={newRequest.status_code}
                            onChange={(e) => setNewRequest({ ...newRequest, status_code: parseInt(e.target.value) || 200 })}
                          />
                        </div>
                      </div>
                      <div>
                        <Label htmlFor="request-body">Response Body</Label>
                        <Textarea
                          id="request-body"
                          value={newRequest.response_body}
                          onChange={(e) => setNewRequest({ ...newRequest, response_body: e.target.value })}
                          placeholder="Response body..."
                          rows={6}
                        />
                      </div>
                    </div>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setCreateRequestOpen(false)}>
                        Cancel
                      </Button>
                      <Button onClick={handleCreateRequest} disabled={!newRequest.name.trim() || !newRequest.path.trim()}>
                        Create Request
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>

                <Dialog open={importOpen} onOpenChange={setImportOpen}>
                  <DialogTrigger asChild>
                    <Button size="sm">
                      <Upload className="w-4 h-4 mr-2" />
                      Import
                    </Button>
                  </DialogTrigger>
                  <DialogContent className="max-w-4xl">
                    <DialogHeader>
                      <DialogTitle>Import Data</DialogTitle>
                      <DialogDescription>
                        Import API data from various formats to create mock endpoints.
                      </DialogDescription>
                    </DialogHeader>
                    <Tabs defaultValue="paste" className="w-full">
                      <TabsList className="grid w-full grid-cols-3">
                        <TabsTrigger value="paste">Paste Data</TabsTrigger>
                        <TabsTrigger value="upload">Upload File</TabsTrigger>
                        <TabsTrigger value="history">History</TabsTrigger>
                      </TabsList>
                      <TabsContent value="paste" className="space-y-4">
                        <div>
                          <Label htmlFor="import-format">Format</Label>
                          <Select
                            value={importData.format}
                            onValueChange={(value) => setImportData({ ...importData, format: value })}
                          >
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="postman">Postman Collection</SelectItem>
                              <SelectItem value="insomnia">Insomnia Export</SelectItem>
                              <SelectItem value="curl">cURL Commands</SelectItem>
                              <SelectItem value="openapi">OpenAPI/Swagger</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div>
                          <Label htmlFor="import-data">Data</Label>
                          <Textarea
                            id="import-data"
                            value={importData.data}
                            onChange={(e) => setImportData({ ...importData, data: e.target.value })}
                            placeholder="Paste your API data here..."
                            rows={12}
                          />
                        </div>
                        <div className="flex items-center space-x-2">
                          <Checkbox
                            id="create-folders"
                            checked={importData.create_folders}
                            onCheckedChange={(checked: boolean) => setImportData({ ...importData, create_folders: checked as boolean })}
                          />
                          <Label htmlFor="create-folders">Create folders for organization</Label>
                        </div>
                      </TabsContent>
                      <TabsContent value="upload" className="space-y-4">
                        <div className="border-2 border-dashed border-muted-foreground/25 rounded-lg p-8 text-center">
                          <Upload className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
                          <p className="text-lg font-medium mb-2">Drop your file here</p>
                          <p className="text-sm text-muted-foreground mb-4">
                            Supports Postman collections, Insomnia exports, OpenAPI specs, and more
                          </p>
                          <Input
                            type="file"
                            accept=".json,.yaml,.yml,.txt"
                            onChange={(e) => {
                              const file = e.target.files?.[0];
                              if (file) {
                                // Validate file
                                const validation = validateFile(file, {
                                  maxSize: 10 * 1024 * 1024, // 10MB
                                  allowedExtensions: ['json', 'yaml', 'yml', 'txt'],
                                });

                                if (!validation.valid) {
                                  toast.error(validation.error || 'Invalid file');
                                  e.target.value = '';
                                  return;
                                }

                                const reader = new FileReader();
                                reader.onload = (e) => {
                                  const content = e.target?.result as string;
                                  // Auto-detect format based on file content
                                  let format = 'postman';
                                  if (content.includes('swagger') || content.includes('openapi')) {
                                    format = 'openapi';
                                  } else if (content.includes('curl')) {
                                    format = 'curl';
                                  }
                                  setImportData({ ...importData, data: content, format });
                                };
                                reader.onerror = () => {
                                  toast.error('Failed to read file');
                                  logError(new Error('FileReader error'), 'File upload');
                                };
                                reader.readAsText(file);
                              }
                            }}
                            className="max-w-xs mx-auto"
                          />
                        </div>
                      </TabsContent>
                      <TabsContent value="history" className="space-y-4">
                        <div className="space-y-2">
                          {importHistory.length === 0 ? (
                            <p className="text-muted-foreground text-center py-8">No import history available</p>
                          ) : (
                            importHistory.map((item) => (
                              <div key={`${item.timestamp}-${item.format}`} className="flex items-center justify-between p-3 border rounded">
                                <div>
                                  <p className="font-medium">{item.format}</p>
                                  <p className="text-sm text-muted-foreground">
                                    {item.timestamp} • {item.routeCount} routes
                                  </p>
                                </div>
                                <div className="flex items-center gap-2">
                                  <Badge variant={item.success ? "default" : "destructive"}>
                                    {item.success ? "Success" : "Failed"}
                                  </Badge>
                                  <Button variant="outline" size="sm" onClick={() => handleReimport(item)}>
                                    Re-import
                                  </Button>
                                </div>
                              </div>
                            ))
                          )}
                        </div>
                      </TabsContent>
                    </Tabs>
                    <DialogFooter>
                      <Button variant="outline" onClick={() => setImportOpen(false)}>
                        Cancel
                      </Button>
                      <Button onClick={handlePreviewImport} disabled={!importData.data.trim()}>
                        Preview Import
                      </Button>
                    </DialogFooter>
                  </DialogContent>
                </Dialog>

                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setEncryptionSettingsOpen(true)}
                >
                  <Shield className="w-4 h-4 mr-2" />
                  Encryption
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {/* Folders */}
              {selectedWorkspace.folders.length > 0 && (
                <div>
                  <h3 className="text-lg font-semibold mb-2">Folders</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                    {selectedWorkspace.folders.map((folder) => (
                      <Card
                        key={folder.id}
                        className={`cursor-pointer transition-all hover:shadow-sm ${
                          selectedFolder?.summary.id === folder.id ? 'ring-2 ring-primary' : ''
                        }`}
                        onClick={() => handleFolderClick(folder.id)}
                      >
                        <CardContent className="p-4">
                          <div className="flex items-center space-x-2">
                            <Folder className="w-4 h-4" />
                            <span className="font-medium">{folder.name}</span>
                            <Badge variant="outline">{folder.request_count} requests</Badge>
                          </div>
                          {folder.description && (
                            <p className="text-sm text-muted-foreground mt-1">{folder.description}</p>
                          )}
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </div>
              )}

              {/* Requests */}
              <div>
                <h3 className="text-lg font-semibold mb-2">
                  Requests {selectedFolder && `in ${selectedFolder.summary.name}`}
                </h3>
                {selectedFolder ? (
                  selectedFolder.requests.length === 0 ? (
                    <p className="text-muted-foreground">No requests in this folder</p>
                  ) : (
                    <div className="space-y-2">
                      {selectedFolder.requests.map((request) => (
                        <Card key={request.id}>
                          <CardContent className="p-4">
                            <div className="flex items-center justify-between">
                              <div className="flex items-center space-x-3">
                                <Badge variant="outline">{request.method}</Badge>
                                <span className="font-medium">{request.name}</span>
                                <span className="text-sm text-muted-foreground">{request.path}</span>
                              </div>
                              <div className="flex items-center space-x-2">
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() => handleViewHistory(request.id, request.name)}
                                >
                                  <History className="w-4 h-4" />
                                </Button>
                                <Button variant="ghost" size="sm">
                                  <Settings className="w-4 h-4" />
                                </Button>
                              </div>
                            </div>
                          </CardContent>
                        </Card>
                      ))}
                    </div>
                  )
                ) : selectedWorkspace.requests.length === 0 ? (
                  <p className="text-muted-foreground">No requests in this workspace</p>
                ) : (
                  <div className="space-y-2">
                    {selectedWorkspace.requests.map((request) => (
                      <Card key={request.id}>
                        <CardContent className="p-4">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center space-x-3">
                              <Badge variant="outline">{request.method}</Badge>
                              <span className="font-medium">{request.name}</span>
                              <span className="text-sm text-muted-foreground">{request.path}</span>
                            </div>
                            <div className="flex items-center space-x-2">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => handleViewHistory(request.id, request.name)}
                              >
                                <History className="w-4 h-4" />
                              </Button>
                              <Button variant="ghost" size="sm">
                                <Settings className="w-4 h-4" />
                              </Button>
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Import Preview Dialog */}
      <Dialog open={importPreviewOpen} onOpenChange={setImportPreviewOpen}>
        <DialogContent className="max-w-6xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Import Preview</DialogTitle>
            <DialogDescription>
              Review the routes that will be imported. Select which routes to include.
            </DialogDescription>
          </DialogHeader>
          {importPreviewData && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-4">
                  <span className="text-sm text-muted-foreground">
                    {importPreviewData.routes?.length || 0} routes found
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      if (selectedRoutes.size === importPreviewData.routes?.length) {
                        setSelectedRoutes(new Set());
                      } else {
                        setSelectedRoutes(new Set(importPreviewData.routes?.map((_, index: number) => index) || []));
                      }
                    }}
                  >
                    {selectedRoutes.size === importPreviewData.routes?.length ? 'Deselect All' : 'Select All'}
                  </Button>
                </div>
                <Badge variant="secondary">
                  {selectedRoutes.size} selected
                </Badge>
              </div>

              {importPreviewData.warnings && importPreviewData.warnings.length > 0 && (
                <Alert>
                  <AlertTriangle className="h-4 w-4" />
                  <div>
                    <p className="font-medium">Warnings</p>
                    <ul className="list-disc list-inside mt-1">
                      {importPreviewData.warnings.map((warning: string) => (
                        <li key={warning} className="text-sm">{warning}</li>
                      ))}
                    </ul>
                  </div>
                </Alert>
              )}

              <div className="space-y-2 max-h-96 overflow-y-auto">
                {importPreviewData.routes?.map((route, index: number) => (
                  <Card key={`${route.method}-${route.path}-${index}`} className="p-4">
                    <div className="flex items-start space-x-3">
                      <Checkbox
                        checked={selectedRoutes.has(index)}
                        onCheckedChange={(checked: boolean) => {
                          const newSelected = new Set(selectedRoutes);
                          if (checked) {
                            newSelected.add(index);
                          } else {
                            newSelected.delete(index);
                          }
                          setSelectedRoutes(newSelected);
                        }}
                      />
                      <div className="flex-1 space-y-2">
                        <div className="flex items-center space-x-2">
                          <Badge variant="outline">{route.method}</Badge>
                          <span className="font-medium">{route.name || route.path}</span>
                          <span className="text-sm text-muted-foreground">{route.path}</span>
                        </div>
                        {route.description && (
                          <p className="text-sm text-muted-foreground">{route.description}</p>
                        )}
                        <div className="flex items-center space-x-4 text-xs text-muted-foreground">
                          <span>Status: {route.status_code || 200}</span>
                          {route.headers && Object.keys(route.headers).length > 0 && (
                            <span>{Object.keys(route.headers).length} headers</span>
                          )}
                        </div>
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setImportPreviewOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleImport} disabled={selectedRoutes.size === 0}>
              Import {selectedRoutes.size} Routes
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* History Dialog */}
      <Dialog open={historyDialogOpen} onOpenChange={setHistoryDialogOpen}>
        <DialogContent className="max-w-4xl">
          <DialogHeader>
            <DialogTitle>Request History - {selectedRequestForHistory?.name}</DialogTitle>
          </DialogHeader>
          <ResponseHistory
            workspaceId={selectedWorkspace?.summary.id || ''}
            requestId={selectedRequestForHistory?.id || ''}
            requestName={selectedRequestForHistory?.name || ''}
          />
        </DialogContent>
      </Dialog>

      {/* Encryption Settings Dialog */}
      <Dialog open={encryptionSettingsOpen} onOpenChange={setEncryptionSettingsOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Encryption Settings</DialogTitle>
            <DialogDescription>
              Configure encryption settings for this workspace.
            </DialogDescription>
          </DialogHeader>
          <EncryptionSettings
            workspaceId={selectedWorkspace?.summary.id || ''}
            workspaceName={selectedWorkspace?.summary.name || ''}
          />
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteConfirmOpen} onOpenChange={setDeleteConfirmOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Workspace</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this workspace? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteConfirmOpen(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleDeleteWorkspace}>
              Delete Workspace
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default WorkspacesPage;
