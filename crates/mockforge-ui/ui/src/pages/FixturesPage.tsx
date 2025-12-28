import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { FileText, Download, Trash2, Search, Eye, Plus, Edit3, Move } from 'lucide-react';
import { useFixtures } from '../hooks/useApi';
import type { FixtureInfo } from '../services/api';
import {
  PageHeader,
  ModernCard,
  Alert,
  EmptyState,
  Section
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose
} from '../components/ui/Dialog';
import { toast } from 'sonner';


export function FixturesPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedMethod, setSelectedMethod] = useState<string>('all');
  const [selectedFixture, setSelectedFixture] = useState<FixtureInfo | null>(null);
  const [isViewingFixture, setIsViewingFixture] = useState(false);
  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
  const [fixtureToRename, setFixtureToRename] = useState<FixtureInfo | null>(null);
  const [newFixtureName, setNewFixtureName] = useState('');
  const [isMoveDialogOpen, setIsMoveDialogOpen] = useState(false);
  const [fixtureToMove, setFixtureToMove] = useState<FixtureInfo | null>(null);
  const [newFixturePath, setNewFixturePath] = useState('');
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [fixtureToDelete, setFixtureToDelete] = useState<FixtureInfo | null>(null);

  const { data: fixtures, isLoading, error, refetch } = useFixtures();

  const filteredFixtures = fixtures?.filter((fixture: FixtureInfo) => {
    const matchesSearch = searchTerm === '' ||
      fixture.path.toLowerCase().includes(searchTerm.toLowerCase()) ||
      fixture.method?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      fixture.protocol?.toLowerCase().includes(searchTerm.toLowerCase());

    const matchesMethod = selectedMethod === 'all' || fixture.method === selectedMethod;

    return matchesSearch && matchesMethod;
  }) || [];

  const handleViewFixture = (fixture: FixtureInfo) => {
    setSelectedFixture(fixture);
    setIsViewingFixture(true);
  };

  const handleDownloadFixture = async (fixture: FixtureInfo) => {
    try {
      const response = await fetch(`/__mockforge/fixtures/${fixture.id}/download`);
      if (!response.ok) {
        throw new Error(`Failed to download fixture: ${response.statusText}`);
      }

      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      // Use content-disposition header if available, otherwise default to fixture id
      const contentDisposition = response.headers.get('Content-Disposition');
      const filenameMatch = contentDisposition?.match(/filename="?([^"]+)"?/);
      a.download = filenameMatch?.[1] || `${fixture.id}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success('Fixture downloaded successfully');
    } catch (error) {
      logger.error('Error downloading fixture', error);
      toast.error(error instanceof Error ? error.message : 'Failed to download fixture');
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const formatDate = (dateString: string): string => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const getMethodBadgeColor = (method?: string): string => {
    switch (method?.toUpperCase()) {
      case 'GET': return 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400';
      case 'POST': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400';
      case 'PUT': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400';
      case 'DELETE': return 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400';
      case 'PATCH': return 'bg-purple-100 text-purple-800 dark:bg-purple-900/20 dark:text-purple-400';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400';
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
          message={error instanceof Error ? error.message : 'Unable to fetch fixture data. Please try again.'}
        />
      </div>
    );
  }

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
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="default"
              size="sm"
              className="flex items-center gap-2"
            >
              <Plus className="h-4 w-4" />
              New Fixture
            </Button>
          </div>
        }
      />

      {/* Filters and Search */}
      <Section
        title="Filter & Search"
        subtitle="Find and organize your fixtures"
      >
        <ModernCard>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {/* Search Input */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Search Fixtures
              </label>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  placeholder="Search by name, path, or route..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="pl-10"
                />
              </div>
            </div>

            {/* Method Filter */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                HTTP Method
              </label>
              <select
                value={selectedMethod}
                onChange={(e) => setSelectedMethod(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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

            {/* Stats */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Summary
              </label>
              <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                <div className="text-center">
                  <div className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {filteredFixtures.length}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {filteredFixtures.length === 1 ? 'Fixture' : 'Fixtures'}
                  </div>
                </div>
                <div className="text-center">
                  <div className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {formatFileSize(filteredFixtures.reduce((acc: number, f: FixtureInfo) => acc + (f.file_size || 0), 0))}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    Total Size
                  </div>
                </div>
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
                  ? "No fixtures have been created yet. Create your first fixture to get started."
                  : "No fixtures match your current search criteria. Try adjusting your filters."
              }
              action={
                <Button className="flex items-center gap-2">
                  <Plus className="h-4 w-4" />
                  Create First Fixture
                </Button>
              }
            />
          ) : (
            <div className="space-y-4">
              {filteredFixtures.map((fixture: FixtureInfo) => (
                <div
                  key={fixture.id}
                  className="flex items-center justify-between p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                >
                  <div className="flex items-center gap-4 flex-1 min-w-0">
                    <div className="p-3 rounded-xl bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 flex-shrink-0">
                      <FileText className="h-5 w-5" />
                    </div>

                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <h3 className="font-semibold text-gray-900 dark:text-gray-100 truncate">
                          {fixture.id}
                        </h3>
                        {fixture.method && (
                          <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${getMethodBadgeColor(fixture.method)}`}>
                            {fixture.method}
                          </span>
                        )}
                      </div>

                      <div className="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
                        <span className="truncate" title={fixture.path}>
                          Path: {fixture.path}
                        </span>
                      </div>

                      <div className="flex items-center gap-4 mt-2 text-xs text-gray-500 dark:text-gray-400">
                        <span>{formatFileSize(fixture.file_size || 0)}</span>
                        <span>•</span>
                        <span>{fixture.protocol}</span>
                        <span>•</span>
                        <span>{formatDate(fixture.saved_at || '')}</span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 flex-shrink-0">
                    {/* More Actions Menu */}
                    <div className="flex items-center gap-1">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setFixtureToRename(fixture);
                          setNewFixtureName(fixture.id);
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
                        className="text-red-600 hover:text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:text-red-300 dark:hover:bg-red-900/20"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </ModernCard>
      </Section>

      {/* Fixture Viewer Dialog */}
      <Dialog open={isViewingFixture} onOpenChange={setIsViewingFixture}>
        <DialogContent className="max-w-4xl">
          <DialogHeader>
            <DialogTitle>{selectedFixture?.id}</DialogTitle>
            <DialogClose onClick={() => setIsViewingFixture(false)} />
          </DialogHeader>
          <DialogDescription>
            {selectedFixture?.path} ({selectedFixture?.method})
          </DialogDescription>

          <div className="py-4 overflow-y-auto max-h-[60vh]">
            <div className="space-y-4">
              <div className="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
                <div>
                  <span className="font-medium">Method:</span> {selectedFixture?.method}
                </div>
                <div>
                  <span className="font-medium">Protocol:</span> {selectedFixture?.protocol}
                </div>
                <div>
                  <span className="font-medium">Size:</span> {formatFileSize(selectedFixture?.file_size ?? 0)}
                </div>
                <div>
                  <span className="font-medium">Saved:</span> {formatDate(selectedFixture?.saved_at ?? '')}
                </div>
              </div>

              <div>
                <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2">
                  Metadata
                </h4>
                <pre className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 text-sm overflow-x-auto max-h-96 overflow-y-auto">
                  <code className="text-gray-900 dark:text-gray-100">
                    {JSON.stringify({
                      id: selectedFixture?.id,
                      protocol: selectedFixture?.protocol,
                      method: selectedFixture?.method,
                      path: selectedFixture?.path,
                      saved_at: selectedFixture?.saved_at,
                      file_size: selectedFixture?.file_size,
                      file_path: selectedFixture?.file_path,
                      fingerprint: selectedFixture?.fingerprint,
                      metadata: selectedFixture?.metadata
                    }, null, 2)}
                  </code>
                </pre>
              </div>
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
            <Button onClick={() => setIsViewingFixture(false)}>
              Close
            </Button>
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
            Current name: <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">{fixtureToRename?.id}</code>
          </DialogDescription>

          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-900 dark:text-gray-100">New Name</label>
              <Input
                value={newFixtureName}
                onChange={(e) => setNewFixtureName(e.target.value)}
                placeholder="Enter new fixture name"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsRenameDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={async () => {
                if (!fixtureToRename) return;
                try {
                  await fetch(`/__mockforge/fixtures/${fixtureToRename.id}/rename`, {
                    method: 'PUT',
                    headers: {
                      'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ new_name: newFixtureName }),
                  });
                  toast.success('Fixture renamed successfully');
                  setIsRenameDialogOpen(false);
                  refetch();
                } catch (error) {
                  logger.error('Error renaming fixture',error);
                  toast.error('Failed to rename fixture');
                }
              }}
              disabled={!newFixtureName.trim() || newFixtureName === fixtureToRename?.id}
            >
              Rename
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
            Moving: <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">{fixtureToMove?.id}</code>
          </DialogDescription>

          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-900 dark:text-gray-100">New Path</label>
              <Input
                value={newFixturePath}
                onChange={(e) => setNewFixturePath(e.target.value)}
                placeholder="Enter new path"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsMoveDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={async () => {
                if (!fixtureToMove) return;
                try {
                  await fetch(`/__mockforge/fixtures/${fixtureToMove.id}/move`, {
                    method: 'PUT',
                    headers: {
                      'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ new_path: newFixturePath }),
                  });
                  toast.success('Fixture moved successfully');
                  setIsMoveDialogOpen(false);
                  setNewFixturePath('');
                  refetch();
                } catch (error) {
                  logger.error('Error moving fixture',error);
                  toast.error('Failed to move fixture');
                }
              }}
              disabled={!newFixturePath.trim()}
            >
              Move
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
            <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
              <div className="text-sm">
                <div className="font-medium text-gray-900 dark:text-gray-100 mb-1">
                  {fixtureToDelete?.id}
                </div>
                <div className="text-gray-600 dark:text-gray-400">
                  {fixtureToDelete?.path} ({fixtureToDelete?.method})
                </div>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setIsDeleteDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              variant="default"
              onClick={async () => {
                if (!fixtureToDelete) return;
                try {
                  await fetch(`/__mockforge/fixtures/${fixtureToDelete.id}`, {
                    method: 'DELETE',
                  });
                  toast.success('Fixture deleted successfully');
                  setIsDeleteDialogOpen(false);
                  setFixtureToDelete(null);
                  refetch();
                } catch (error) {
                  logger.error('Error deleting fixture',error);
                  toast.error('Failed to delete fixture');
                }
              }}
              className="bg-red-600 hover:bg-red-700 text-white"
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
