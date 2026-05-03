import { logger } from '@/utils/logger';
import React, { useMemo, useState } from 'react';
import {
  FileText,
  Download,
  Trash2,
  Search,
  Eye,
  Plus,
  Edit3,
  Move,
  RefreshCw,
  Tag as TagIcon,
} from 'lucide-react';
import {
  useFixtures,
  useCreateFixture,
  useUpdateFixture,
  useDeleteFixture,
  useRenameFixture,
  useMoveFixture,
  useDownloadFixture,
} from '../hooks/api';
import type { FixtureInfo } from '../services/api';
import {
  PageHeader,
  ModernCard,
  Alert,
  EmptyState,
  Section,
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '../components/ui/Dialog';
import { toast } from 'sonner';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';

const isCloud = isCloudMode();

interface FixtureFormState {
  name: string;
  path: string;
  method: string;
  description: string;
  protocol: string;
  tagsInput: string;
  contentText: string;
  routePath: string;
}

const EMPTY_FORM: FixtureFormState = {
  name: '',
  path: '',
  method: 'GET',
  description: '',
  protocol: '',
  tagsInput: '',
  contentText: '',
  routePath: '',
};

function parseTagsInput(input: string): string[] {
  return input
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0);
}

function stringifyTags(tags: FixtureInfo['tags']): string[] {
  if (Array.isArray(tags)) {
    return tags.filter((t): t is string => typeof t === 'string' && t.length > 0);
  }
  return [];
}

function parseContentText(
  text: string
): { ok: true; value: unknown } | { ok: false; error: string } {
  const trimmed = text.trim();
  if (!trimmed) return { ok: true, value: null };
  try {
    return { ok: true, value: JSON.parse(trimmed) };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Invalid JSON' };
  }
}

function fixtureContentToString(content: FixtureInfo['content']): string {
  if (content === undefined || content === null) return '';
  if (typeof content === 'string') return content;
  try {
    return JSON.stringify(content, null, 2);
  } catch {
    return String(content);
  }
}

function fixtureDisplayName(fixture: FixtureInfo): string {
  return fixture.name || fixture.id;
}

function formFromFixture(fixture: FixtureInfo): FixtureFormState {
  return {
    name: fixture.name || '',
    path: fixture.path || '',
    method: fixture.method || 'GET',
    description: fixture.description || '',
    protocol: fixture.protocol || '',
    tagsInput: stringifyTags(fixture.tags).join(', '),
    contentText: fixtureContentToString(fixture.content),
    routePath: fixture.route_path || '',
  };
}

export function FixturesPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedMethod, setSelectedMethod] = useState<string>('all');
  const [selectedTag, setSelectedTag] = useState<string>('all');
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspace?.id ?? null);

  const [selectedFixture, setSelectedFixture] = useState<FixtureInfo | null>(null);
  const [isViewingFixture, setIsViewingFixture] = useState(false);

  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [createForm, setCreateForm] = useState<FixtureFormState>(EMPTY_FORM);
  const [createContentError, setCreateContentError] = useState<string | null>(null);

  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [fixtureToEdit, setFixtureToEdit] = useState<FixtureInfo | null>(null);
  const [editForm, setEditForm] = useState<FixtureFormState>(EMPTY_FORM);
  const [editContentError, setEditContentError] = useState<string | null>(null);

  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
  const [fixtureToRename, setFixtureToRename] = useState<FixtureInfo | null>(null);
  const [newFixtureName, setNewFixtureName] = useState('');

  const [isMoveDialogOpen, setIsMoveDialogOpen] = useState(false);
  const [fixtureToMove, setFixtureToMove] = useState<FixtureInfo | null>(null);
  const [newFixturePath, setNewFixturePath] = useState('');

  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [fixtureToDelete, setFixtureToDelete] = useState<FixtureInfo | null>(null);

  const { data: fixtures, isLoading, error, refetch, isFetching } = useFixtures();
  const createFixtureMutation = useCreateFixture();
  const updateFixtureMutation = useUpdateFixture();
  const deleteFixtureMutation = useDeleteFixture();
  const renameFixtureMutation = useRenameFixture();
  const moveFixtureMutation = useMoveFixture();
  const downloadFixtureMutation = useDownloadFixture();

  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    (fixtures ?? []).forEach((f) => {
      stringifyTags(f.tags).forEach((t) => tagSet.add(t));
    });
    return Array.from(tagSet).sort();
  }, [fixtures]);

  const filteredFixtures = useMemo(() => {
    const term = searchTerm.toLowerCase();
    return (fixtures ?? []).filter((fixture: FixtureInfo) => {
      const tags = stringifyTags(fixture.tags);
      const matchesSearch =
        term === '' ||
        fixture.path?.toLowerCase().includes(term) ||
        fixture.name?.toLowerCase().includes(term) ||
        fixture.description?.toLowerCase().includes(term) ||
        fixture.method?.toLowerCase().includes(term) ||
        fixture.protocol?.toLowerCase().includes(term) ||
        tags.some((t) => t.toLowerCase().includes(term));

      const matchesMethod = selectedMethod === 'all' || fixture.method === selectedMethod;
      const matchesTag = selectedTag === 'all' || tags.includes(selectedTag);
      return matchesSearch && matchesMethod && matchesTag;
    });
  }, [fixtures, searchTerm, selectedMethod, selectedTag]);

  const handleCreateFixture = async () => {
    if (!createForm.name.trim()) return;

    const contentResult = parseContentText(createForm.contentText);
    if (!contentResult.ok) {
      setCreateContentError(contentResult.error);
      return;
    }
    setCreateContentError(null);

    try {
      await createFixtureMutation.mutateAsync({
        name: createForm.name.trim(),
        path: createForm.path.trim(),
        method: createForm.method,
        description: createForm.description,
        protocol: createForm.protocol || undefined,
        tags: parseTagsInput(createForm.tagsInput),
        content: contentResult.value ?? undefined,
        route_path: createForm.routePath.trim() || undefined,
        // Cloud only: attach the active workspace if there is one. Local mode
        // ignores this field.
        workspace_id: isCloud ? activeWorkspaceId : undefined,
      });
      toast.success('Fixture created successfully');
      setIsCreateDialogOpen(false);
      setCreateForm(EMPTY_FORM);
    } catch (err) {
      logger.error('Error creating fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to create fixture');
    }
  };

  const handleEditFixture = async () => {
    if (!fixtureToEdit) return;
    const contentResult = parseContentText(editForm.contentText);
    if (!contentResult.ok) {
      setEditContentError(contentResult.error);
      return;
    }
    setEditContentError(null);

    try {
      await updateFixtureMutation.mutateAsync({
        fixtureId: fixtureToEdit.id,
        payload: {
          name: editForm.name.trim(),
          path: editForm.path,
          method: editForm.method,
          description: editForm.description,
          protocol: editForm.protocol || undefined,
          tags: parseTagsInput(editForm.tagsInput),
          content: contentResult.value ?? null,
          route_path: editForm.routePath.trim() || undefined,
        },
      });
      toast.success('Fixture updated successfully');
      setIsEditDialogOpen(false);
      setFixtureToEdit(null);
    } catch (err) {
      logger.error('Error updating fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to update fixture');
    }
  };

  const handleRenameFixture = async () => {
    if (!fixtureToRename) return;
    try {
      await renameFixtureMutation.mutateAsync({
        fixtureId: fixtureToRename.id,
        newName: newFixtureName,
      });
      toast.success('Fixture renamed successfully');
      setIsRenameDialogOpen(false);
    } catch (err) {
      logger.error('Error renaming fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to rename fixture');
    }
  };

  const handleMoveFixture = async () => {
    if (!fixtureToMove) return;
    try {
      await moveFixtureMutation.mutateAsync({
        fixtureId: fixtureToMove.id,
        newPath: newFixturePath,
      });
      toast.success('Fixture moved successfully');
      setIsMoveDialogOpen(false);
      setNewFixturePath('');
    } catch (err) {
      logger.error('Error moving fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to move fixture');
    }
  };

  const handleDeleteFixture = async () => {
    if (!fixtureToDelete) return;
    try {
      await deleteFixtureMutation.mutateAsync(fixtureToDelete.id);
      toast.success('Fixture deleted successfully');
      setIsDeleteDialogOpen(false);
      setFixtureToDelete(null);
    } catch (err) {
      logger.error('Error deleting fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to delete fixture');
    }
  };

  const handleViewFixture = (fixture: FixtureInfo) => {
    setSelectedFixture(fixture);
    setIsViewingFixture(true);
  };

  const handleOpenEdit = (fixture: FixtureInfo) => {
    setFixtureToEdit(fixture);
    setEditForm(formFromFixture(fixture));
    setEditContentError(null);
    setIsEditDialogOpen(true);
  };

  const handleDownloadFixture = async (fixture: FixtureInfo) => {
    try {
      const { blob, filename } = await downloadFixtureMutation.mutateAsync(fixture);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success('Fixture downloaded successfully');
    } catch (err) {
      logger.error('Error downloading fixture', err);
      toast.error(err instanceof Error ? err.message : 'Failed to download fixture');
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (!bytes) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const formatDate = (dateString?: string): string => {
    if (!dateString) return '—';
    const d = new Date(dateString);
    if (Number.isNaN(d.getTime())) return '—';
    return d.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const getMethodBadgeColor = (method?: string): string => {
    switch (method?.toUpperCase()) {
      case 'GET':
        return 'bg-success-100 text-success-700 dark:bg-success-900/20 dark:text-success-400';
      case 'POST':
        return 'bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-400';
      case 'PUT':
        return 'bg-warning-100 text-warning-700 dark:bg-warning-900/20 dark:text-warning-400';
      case 'DELETE':
        return 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400';
      case 'PATCH':
        return 'bg-purple-100 text-purple-800 dark:bg-purple-900/20 dark:text-purple-400';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400';
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Mock Fixtures"
          subtitle="Manage and organize your API response fixtures"
        />
        <EmptyState
          icon={<FileText className="h-12 w-12" />}
          title="Loading fixtures..."
          description="Fetching fixture data from the server."
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Mock Fixtures"
          subtitle="Manage and organize your API response fixtures"
        />
        <Alert
          type="error"
          title="Failed to load fixtures"
          message={
            error instanceof Error ? error.message : 'Unable to fetch fixture data. Please try again.'
          }
        />
      </div>
    );
  }

  const totalSize = filteredFixtures.reduce(
    (acc: number, f: FixtureInfo) => acc + (f.file_size || f.size_bytes || 0),
    0
  );

  return (
    <div className="space-y-8">
      <PageHeader
        title="Mock Fixtures"
        subtitle="Manage and organize your API response fixtures"
        action={
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => refetch()}
              disabled={isFetching}
              className="flex items-center gap-2"
            >
              <RefreshCw className={`h-4 w-4 ${isFetching ? 'animate-spin' : ''}`} />
              Refresh
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={() => {
                setCreateForm(EMPTY_FORM);
                setCreateContentError(null);
                setIsCreateDialogOpen(true);
              }}
              className="flex items-center gap-2"
            >
              <Plus className="h-4 w-4" />
              New Fixture
            </Button>
          </div>
        }
      />

      {/* Filters and Search */}
      <Section title="Filter & Search" subtitle="Find and organize your fixtures">
        <ModernCard>
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Search Fixtures
              </label>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search by name, path, tag, description…"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="pl-10"
                />
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                HTTP Method
              </label>
              <select
                value={selectedMethod}
                onChange={(e) => setSelectedMethod(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="all">All Methods</option>
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="DELETE">DELETE</option>
                <option value="PATCH">PATCH</option>
                <option value="HEAD">HEAD</option>
              </select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Tag
              </label>
              <select
                value={selectedTag}
                onChange={(e) => setSelectedTag(e.target.value)}
                disabled={allTags.length === 0}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:opacity-50"
              >
                <option value="all">All Tags</option>
                {allTags.map((tag) => (
                  <option key={tag} value={tag}>
                    {tag}
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Summary
              </label>
              <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
                <div className="text-center">
                  <div className="text-lg font-semibold text-foreground">
                    {filteredFixtures.length}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {filteredFixtures.length === 1 ? 'Fixture' : 'Fixtures'}
                  </div>
                </div>
                {!isCloud && (
                  <div className="text-center">
                    <div className="text-lg font-semibold text-foreground">
                      {formatFileSize(totalSize)}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Total Size
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </ModernCard>
      </Section>

      {/* Fixtures List */}
      <Section
        title={`Fixtures (${filteredFixtures.length})`}
        subtitle="Your mock response fixtures and templates"
      >
        <ModernCard>
          {filteredFixtures.length === 0 ? (
            <EmptyState
              icon={<FileText className="h-12 w-12" />}
              title="No fixtures found"
              description={
                fixtures?.length === 0
                  ? 'No fixtures have been created yet. Create your first fixture to get started.'
                  : 'No fixtures match your current search criteria. Try adjusting your filters.'
              }
              action={
                <Button
                  onClick={() => {
                    setCreateForm(EMPTY_FORM);
                    setIsCreateDialogOpen(true);
                  }}
                  className="flex items-center gap-2"
                >
                  <Plus className="h-4 w-4" />
                  Create First Fixture
                </Button>
              }
            />
          ) : (
            <div className="space-y-4">
              {filteredFixtures.map((fixture: FixtureInfo) => {
                const tags = stringifyTags(fixture.tags);
                const dateStr =
                  fixture.updated_at ||
                  fixture.updatedAt ||
                  fixture.saved_at ||
                  fixture.created_at ||
                  fixture.createdAt;
                const sizeBytes = fixture.file_size || fixture.size_bytes;
                return (
                  <div
                    key={fixture.id}
                    className="flex items-center justify-between p-4 rounded-lg border border-border hover:bg-accent hover:text-accent-foreground/50 transition-colors"
                  >
                    <div className="flex items-center gap-4 flex-1 min-w-0">
                      <div className="p-3 rounded-xl bg-info-50 dark:bg-info-900/20 text-info-600 dark:text-info-400 flex-shrink-0">
                        <FileText className="h-5 w-5" />
                      </div>

                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2 mb-1 flex-wrap">
                          <h3 className="font-semibold text-foreground truncate">
                            {fixtureDisplayName(fixture)}
                          </h3>
                          {fixture.method && (
                            <span
                              className={`px-2 py-0.5 rounded-full text-xs font-medium ${getMethodBadgeColor(fixture.method)}`}
                            >
                              {fixture.method}
                            </span>
                          )}
                          {fixture.protocol && (
                            <span className="px-2 py-0.5 rounded-full text-xs font-medium bg-indigo-100 text-indigo-800 dark:bg-indigo-900/20 dark:text-indigo-400">
                              {fixture.protocol}
                            </span>
                          )}
                        </div>

                        {fixture.description && (
                          <p className="text-sm text-muted-foreground truncate mb-1">
                            {fixture.description}
                          </p>
                        )}

                        <div className="flex items-center gap-4 text-sm text-muted-foreground">
                          <span className="truncate" title={fixture.path}>
                            Path: {fixture.path || '—'}
                          </span>
                        </div>

                        <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground flex-wrap">
                          {sizeBytes ? <span>{formatFileSize(sizeBytes)}</span> : null}
                          {sizeBytes ? <span>•</span> : null}
                          <span>{formatDate(dateStr)}</span>
                          {tags.length > 0 && (
                            <>
                              <span>•</span>
                              <span className="flex items-center gap-1 flex-wrap">
                                <TagIcon className="h-3 w-3" />
                                {tags.map((t) => (
                                  <span
                                    key={t}
                                    className="px-1.5 py-0.5 rounded bg-muted text-foreground"
                                  >
                                    {t}
                                  </span>
                                ))}
                              </span>
                            </>
                          )}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 flex-shrink-0">
                      <div className="flex items-center gap-1">
                        {isCloud && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleOpenEdit(fixture)}
                            className="flex items-center gap-2"
                          >
                            <Edit3 className="h-4 w-4" />
                            Edit
                          </Button>
                        )}

                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setFixtureToRename(fixture);
                            setNewFixtureName(fixtureDisplayName(fixture));
                            setIsRenameDialogOpen(true);
                          }}
                          className="flex items-center gap-2"
                        >
                          <Edit3 className="h-4 w-4" />
                          Rename
                        </Button>

                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setFixtureToMove(fixture);
                            setNewFixturePath(fixture.path || '');
                            setIsMoveDialogOpen(true);
                          }}
                          className="flex items-center gap-2"
                        >
                          <Move className="h-4 w-4" />
                          Move
                        </Button>

                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleViewFixture(fixture)}
                        >
                          <Eye className="h-4 w-4" />
                        </Button>

                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleDownloadFixture(fixture)}
                        >
                          <Download className="h-4 w-4" />
                        </Button>

                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            setFixtureToDelete(fixture);
                            setIsDeleteDialogOpen(true);
                          }}
                          className="text-danger-600 hover:text-danger-700 hover:bg-danger-50 dark:text-danger-400 dark:hover:text-danger-300 dark:hover:bg-danger-900/20"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </ModernCard>
      </Section>

      {/* Create Fixture Dialog */}
      <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Create New Fixture</DialogTitle>
            <DialogClose onClick={() => setIsCreateDialogOpen(false)} />
          </DialogHeader>
          <DialogDescription>
            Create a new mock response fixture for your API endpoints.
          </DialogDescription>

          <div className="py-4 space-y-4 overflow-y-auto max-h-[60vh]">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Fixture Name *
                </label>
                <Input
                  value={createForm.name}
                  onChange={(e) => setCreateForm({ ...createForm, name: e.target.value })}
                  placeholder="e.g., Get Users Response"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  HTTP Method
                </label>
                <select
                  value={createForm.method}
                  onChange={(e) => setCreateForm({ ...createForm, method: e.target.value })}
                  className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="GET">GET</option>
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                  <option value="DELETE">DELETE</option>
                  <option value="PATCH">PATCH</option>
                  <option value="HEAD">HEAD</option>
                </select>
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">Path</label>
              <Input
                value={createForm.path}
                onChange={(e) => setCreateForm({ ...createForm, path: e.target.value })}
                placeholder="e.g., /api/users"
              />
            </div>

            {isCloud && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Route Path (optional)
                </label>
                <Input
                  value={createForm.routePath}
                  onChange={(e) =>
                    setCreateForm({ ...createForm, routePath: e.target.value })
                  }
                  placeholder="e.g., /api/users/{id} — canonical path with placeholders"
                />
              </div>
            )}

            {isCloud && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Protocol
                </label>
                <select
                  value={createForm.protocol}
                  onChange={(e) => setCreateForm({ ...createForm, protocol: e.target.value })}
                  className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="">— unspecified —</option>
                  <option value="http">http</option>
                  <option value="grpc">grpc</option>
                  <option value="websocket">websocket</option>
                  <option value="graphql">graphql</option>
                  <option value="mqtt">mqtt</option>
                  <option value="kafka">kafka</option>
                  <option value="amqp">amqp</option>
                  <option value="smtp">smtp</option>
                  <option value="ftp">ftp</option>
                  <option value="tcp">tcp</option>
                </select>
              </div>
            )}

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Description
              </label>
              <Input
                value={createForm.description}
                onChange={(e) =>
                  setCreateForm({ ...createForm, description: e.target.value })
                }
                placeholder="Optional description"
              />
            </div>

            {isCloud && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Tags
                </label>
                <Input
                  value={createForm.tagsInput}
                  onChange={(e) =>
                    setCreateForm({ ...createForm, tagsInput: e.target.value })
                  }
                  placeholder="comma-separated, e.g. auth, users, billing"
                />
              </div>
            )}

            {isCloud && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Response Content (JSON)
                </label>
                <Textarea
                  value={createForm.contentText}
                  onChange={(e) =>
                    setCreateForm({ ...createForm, contentText: e.target.value })
                  }
                  placeholder={'{\n  "users": []\n}'}
                  className="font-mono text-xs min-h-[180px]"
                  error={createContentError ?? undefined}
                />
                {createContentError && (
                  <p className="text-xs text-danger-600 dark:text-danger-400">
                    Invalid JSON: {createContentError}
                  </p>
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsCreateDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreateFixture}
              disabled={!createForm.name.trim() || createFixtureMutation.isPending}
            >
              {createFixtureMutation.isPending ? 'Creating…' : 'Create Fixture'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Fixture Dialog (cloud only) */}
      <Dialog open={isEditDialogOpen} onOpenChange={setIsEditDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Edit Fixture</DialogTitle>
            <DialogClose onClick={() => setIsEditDialogOpen(false)} />
          </DialogHeader>
          <DialogDescription>
            Update fixture metadata, tags, and response content.
          </DialogDescription>

          <div className="py-4 space-y-4 overflow-y-auto max-h-[60vh]">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  Name
                </label>
                <Input
                  value={editForm.name}
                  onChange={(e) => setEditForm({ ...editForm, name: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">
                  HTTP Method
                </label>
                <select
                  value={editForm.method}
                  onChange={(e) => setEditForm({ ...editForm, method: e.target.value })}
                  className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="GET">GET</option>
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                  <option value="DELETE">DELETE</option>
                  <option value="PATCH">PATCH</option>
                  <option value="HEAD">HEAD</option>
                </select>
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">Path</label>
              <Input
                value={editForm.path}
                onChange={(e) => setEditForm({ ...editForm, path: e.target.value })}
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Protocol
              </label>
              <select
                value={editForm.protocol}
                onChange={(e) => setEditForm({ ...editForm, protocol: e.target.value })}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="">— unspecified —</option>
                <option value="http">http</option>
                <option value="grpc">grpc</option>
                <option value="websocket">websocket</option>
                <option value="graphql">graphql</option>
                <option value="mqtt">mqtt</option>
                <option value="kafka">kafka</option>
                <option value="amqp">amqp</option>
                <option value="smtp">smtp</option>
                <option value="ftp">ftp</option>
                <option value="tcp">tcp</option>
              </select>
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Route Path (optional)
              </label>
              <Input
                value={editForm.routePath}
                onChange={(e) => setEditForm({ ...editForm, routePath: e.target.value })}
                placeholder="e.g., /api/users/{id} — canonical path with placeholders"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Description
              </label>
              <Input
                value={editForm.description}
                onChange={(e) =>
                  setEditForm({ ...editForm, description: e.target.value })
                }
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Tags (comma-separated)
              </label>
              <Input
                value={editForm.tagsInput}
                onChange={(e) => setEditForm({ ...editForm, tagsInput: e.target.value })}
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Response Content (JSON)
              </label>
              <Textarea
                value={editForm.contentText}
                onChange={(e) =>
                  setEditForm({ ...editForm, contentText: e.target.value })
                }
                className="font-mono text-xs min-h-[220px]"
                error={editContentError ?? undefined}
              />
              {editContentError && (
                <p className="text-xs text-danger-600 dark:text-danger-400">
                  Invalid JSON: {editContentError}
                </p>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsEditDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleEditFixture}
              disabled={!editForm.name.trim() || updateFixtureMutation.isPending}
            >
              {updateFixtureMutation.isPending ? 'Saving…' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Fixture Viewer Dialog */}
      <Dialog open={isViewingFixture} onOpenChange={setIsViewingFixture}>
        <DialogContent className="max-w-4xl">
          <DialogHeader>
            <DialogTitle>{selectedFixture ? fixtureDisplayName(selectedFixture) : ''}</DialogTitle>
            <DialogClose onClick={() => setIsViewingFixture(false)} />
          </DialogHeader>
          <DialogDescription>
            {selectedFixture?.path} ({selectedFixture?.method})
          </DialogDescription>

          <div className="py-4 overflow-y-auto max-h-[60vh]">
            <div className="space-y-4">
              <div className="flex items-center gap-4 text-sm text-muted-foreground flex-wrap">
                <div>
                  <span className="font-medium">Method:</span> {selectedFixture?.method || '—'}
                </div>
                <div>
                  <span className="font-medium">Protocol:</span>{' '}
                  {selectedFixture?.protocol || '—'}
                </div>
                {(selectedFixture?.file_size || selectedFixture?.size_bytes) && (
                  <div>
                    <span className="font-medium">Size:</span>{' '}
                    {formatFileSize(
                      selectedFixture?.file_size ?? selectedFixture?.size_bytes ?? 0
                    )}
                  </div>
                )}
                {(selectedFixture?.created_at || selectedFixture?.createdAt) && (
                  <div>
                    <span className="font-medium">Created:</span>{' '}
                    {formatDate(
                      selectedFixture?.created_at || selectedFixture?.createdAt
                    )}
                  </div>
                )}
                <div>
                  <span className="font-medium">Updated:</span>{' '}
                  {formatDate(
                    selectedFixture?.updated_at ||
                      selectedFixture?.updatedAt ||
                      selectedFixture?.saved_at ||
                      selectedFixture?.created_at ||
                      selectedFixture?.createdAt
                  )}
                </div>
                {selectedFixture?.created_by_username && (
                  <div>
                    <span className="font-medium">Created by:</span>{' '}
                    {selectedFixture.created_by_username}
                  </div>
                )}
                {selectedFixture?.route_path && (
                  <div>
                    <span className="font-medium">Route path:</span>{' '}
                    <code className="text-xs bg-muted px-1 rounded">
                      {selectedFixture.route_path}
                    </code>
                  </div>
                )}
                {selectedFixture?.workspace_id && (
                  <div>
                    <span className="font-medium">Workspace:</span>{' '}
                    <code className="text-xs bg-muted px-1 rounded">
                      {selectedFixture.workspace_id}
                    </code>
                  </div>
                )}
              </div>

              {selectedFixture?.description && (
                <div>
                  <h4 className="text-sm font-medium text-foreground mb-1">
                    Description
                  </h4>
                  <p className="text-sm text-foreground">
                    {selectedFixture.description}
                  </p>
                </div>
              )}

              {selectedFixture && stringifyTags(selectedFixture.tags).length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-foreground mb-2">
                    Tags
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {stringifyTags(selectedFixture.tags).map((t) => (
                      <span
                        key={t}
                        className="px-2 py-0.5 rounded bg-muted text-xs text-foreground"
                      >
                        {t}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              <div>
                <h4 className="text-sm font-medium text-foreground mb-2">
                  Response Content
                </h4>
                <pre className="bg-muted rounded-lg p-4 text-sm overflow-x-auto max-h-96 overflow-y-auto">
                  <code className="text-foreground">
                    {selectedFixture
                      ? fixtureContentToString(selectedFixture.content) ||
                        '(no content stored)'
                      : ''}
                  </code>
                </pre>
              </div>

              {!isCloud && selectedFixture?.metadata && (
                <div>
                  <h4 className="text-sm font-medium text-foreground mb-2">
                    Metadata
                  </h4>
                  <pre className="bg-muted rounded-lg p-4 text-sm overflow-x-auto max-h-96 overflow-y-auto">
                    <code className="text-foreground">
                      {JSON.stringify(selectedFixture.metadata, null, 2)}
                    </code>
                  </pre>
                </div>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => selectedFixture && handleDownloadFixture(selectedFixture)}
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              Download
            </Button>
            {isCloud && selectedFixture && (
              <Button
                variant="outline"
                onClick={() => {
                  setIsViewingFixture(false);
                  handleOpenEdit(selectedFixture);
                }}
                className="flex items-center gap-2"
              >
                <Edit3 className="h-4 w-4" />
                Edit
              </Button>
            )}
            <Button onClick={() => setIsViewingFixture(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Rename Dialog */}
      <Dialog open={isRenameDialogOpen} onOpenChange={setIsRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Fixture</DialogTitle>
            <DialogClose onClick={() => setIsRenameDialogOpen(false)} />
          </DialogHeader>
          <DialogDescription>
            Current name:{' '}
            <code className="bg-muted px-2 py-1 rounded">
              {fixtureToRename ? fixtureDisplayName(fixtureToRename) : ''}
            </code>
          </DialogDescription>

          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                New Name
              </label>
              <Input
                value={newFixtureName}
                onChange={(e) => setNewFixtureName(e.target.value)}
                placeholder="Enter new fixture name"
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsRenameDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleRenameFixture}
              disabled={
                !newFixtureName.trim() ||
                newFixtureName === (fixtureToRename ? fixtureDisplayName(fixtureToRename) : '') ||
                renameFixtureMutation.isPending
              }
            >
              {renameFixtureMutation.isPending ? 'Renaming…' : 'Rename'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Move Dialog */}
      <Dialog open={isMoveDialogOpen} onOpenChange={setIsMoveDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Move Fixture</DialogTitle>
            <DialogClose onClick={() => setIsMoveDialogOpen(false)} />
          </DialogHeader>
          <DialogDescription>
            Moving:{' '}
            <code className="bg-muted px-2 py-1 rounded">
              {fixtureToMove ? fixtureDisplayName(fixtureToMove) : ''}
            </code>
          </DialogDescription>

          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                New Path
              </label>
              <Input
                value={newFixturePath}
                onChange={(e) => setNewFixturePath(e.target.value)}
                placeholder="Enter new path"
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsMoveDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleMoveFixture}
              disabled={!newFixturePath.trim() || moveFixtureMutation.isPending}
            >
              {moveFixtureMutation.isPending ? 'Moving…' : 'Move'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Fixture</DialogTitle>
            <DialogClose onClick={() => setIsDeleteDialogOpen(false)} />
          </DialogHeader>
          <DialogDescription>
            Are you sure you want to delete this fixture? This action cannot be undone.
          </DialogDescription>

          <div className="py-4">
            <div className="bg-muted rounded-lg p-4">
              <div className="text-sm">
                <div className="font-medium text-foreground mb-1">
                  {fixtureToDelete ? fixtureDisplayName(fixtureToDelete) : ''}
                </div>
                <div className="text-muted-foreground">
                  {fixtureToDelete?.path} ({fixtureToDelete?.method})
                </div>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsDeleteDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="default"
              onClick={handleDeleteFixture}
              disabled={deleteFixtureMutation.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteFixtureMutation.isPending ? 'Deleting…' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
