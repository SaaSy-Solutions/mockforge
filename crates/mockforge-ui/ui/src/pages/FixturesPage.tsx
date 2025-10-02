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


export function FixturesPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedMethod, setSelectedMethod] = useState<string>('all');
  const [selectedFixture, setSelectedFixture] = useState<FixtureInfo | null>(null);
  const [isViewingFixture, setIsViewingFixture] = useState(false);
  const [_fixturesToCompare, _setFixturesToCompare] = useState<FixtureInfo[]>([]);
  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
  const [fixtureToRename, setFixtureToRename] = useState<FixtureInfo | null>(null);
  const [newFixtureName, setNewFixtureName] = useState('');
  const [isMoveDialogOpen, setIsMoveDialogOpen] = useState(false);
  const [fixtureToMove, setFixtureToMove] = useState<FixtureInfo | null>(null);
  const [newFixturePath, setNewFixturePath] = useState('');
  const [_selectedBulkAction, _setSelectedBulkAction] = useState<string>('');

  const { data: fixtures, isLoading, error, refetch } = useFixtures();

  const filteredFixtures = fixtures?.filter(fixture => {
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

  const handleDownloadFixture = (fixture: FixtureInfo) => {
    // For now, we'll create a placeholder content since the API doesn't provide the actual fixture content
    const placeholderContent = {
      fixture_id: fixture.id,
      method: fixture.method,
      path: fixture.path,
      protocol: fixture.protocol,
      saved_at: fixture.saved_at,
      fingerprint: fixture.fingerprint,
      metadata: fixture.metadata
    };

    const blob = new Blob([JSON.stringify(placeholderContent, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${fixture.id}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
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
                    {formatFileSize(filteredFixtures.reduce((acc, f) => acc + (f.file_size || 0), 0))}
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
              {filteredFixtures.map((fixture) => (
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
                        <span className="truncate max-w-xs" title={fixture.path}>
                          {fixture.path}
                        </span>
                        <span className="truncate max-w-xs" title={fixture.path}>
                          {fixture.path}
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

      {/* Fixture Viewer Modal */}
      {isViewingFixture && selectedFixture && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="fixed inset-0 bg-black bg-opacity-50" onClick={() => setIsViewingFixture(false)} />
          <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-xl max-w-4xl w-full mx-4 max-h-[80vh] overflow-hidden">
            <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
              <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {selectedFixture.id}
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                    {selectedFixture.path} ({selectedFixture.method})
                  </p>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleDownloadFixture(selectedFixture)}
                  className="flex items-center gap-2"
                >
                  <Download className="h-4 w-4" />
                  Download
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsViewingFixture(false)}
                >
                  Close
                </Button>
              </div>
            </div>

            <div className="p-6 overflow-y-auto max-h-[calc(80vh-120px)]">
              <div className="space-y-4">
                  <div className="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
                    <div>
                      <span className="font-medium">Method:</span> {selectedFixture.method}
                    </div>
                    <div>
                      <span className="font-medium">Protocol:</span> {selectedFixture.protocol}
                    </div>
                    <div>
                      <span className="font-medium">Size:</span> {formatFileSize(selectedFixture.file_size ?? 0)}
                    </div>
                    <div>
                      <span className="font-medium">Saved:</span> {formatDate(selectedFixture.saved_at ?? '')}
                    </div>
                  </div>

                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2">
                    Metadata
                  </h4>
                  <pre className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 text-sm overflow-x-auto max-h-96 overflow-y-auto">
                    <code className="text-gray-900 dark:text-gray-100">
                      {JSON.stringify({
                        id: selectedFixture.id,
                        protocol: selectedFixture.protocol,
                        method: selectedFixture.method,
                        path: selectedFixture.path,
                        saved_at: selectedFixture.saved_at,
                        file_size: selectedFixture.file_size,
                        file_path: selectedFixture.file_path,
                        fingerprint: selectedFixture.fingerprint,
                        metadata: selectedFixture.metadata
                      }, null, 2)}
                    </code>
                  </pre>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Rename Dialog */}
      {isRenameDialogOpen && fixtureToRename && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="fixed inset-0 bg-black bg-opacity-50" onClick={() => setIsRenameDialogOpen(false)} />
          <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-xl max-w-md w-full mx-4">
            <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">Rename Fixture</h3>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setIsRenameDialogOpen(false)}
              >
                ×
              </Button>
            </div>
            <div className="p-6">
              <div className="space-y-4">
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  Current name: <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">{fixtureToRename.id}</code>
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100">New Name</label>
                  <Input
                    value={newFixtureName}
                    onChange={(e) => setNewFixtureName(e.target.value)}
                    placeholder="Enter new fixture name"
                  />
                </div>
                <div className="flex items-center justify-end gap-3">
                  <Button
                    variant="outline"
                    onClick={() => setIsRenameDialogOpen(false)}
                  >
                    Cancel
                  </Button>
    <Button
      onClick={async () => {
        try {
          await fetch(`/__mockforge/fixtures/${fixtureToRename.id}/rename`, {
            method: 'PUT',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({ new_name: newFixtureName }),
          });
          setIsRenameDialogOpen(false);
          refetch();
        } catch (error) {
          console.error('Error renaming fixture:', error);
        }
      }}
      disabled={!newFixtureName.trim() || newFixtureName === fixtureToRename.id}
    >
                    Rename
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Move Dialog */}
      {isMoveDialogOpen && fixtureToMove && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="fixed inset-0 bg-black bg-opacity-50" onClick={() => setIsMoveDialogOpen(false)} />
          <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-xl max-w-md w-full mx-4">
            <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">Move Fixture</h3>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setIsMoveDialogOpen(false)}
              >
                ×
              </Button>
            </div>
            <div className="p-6">
              <div className="space-y-4">
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  Moving: <code className="bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">{fixtureToMove.id}</code>
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100">New Path</label>
                  <Input
                    value={newFixturePath}
                    onChange={(e) => setNewFixturePath(e.target.value)}
                    placeholder="Enter new path"
                  />
                </div>
                <div className="flex items-center justify-end gap-3">
                  <Button
                    variant="outline"
                    onClick={() => setIsMoveDialogOpen(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    onClick={async () => {
                      try {
                        await fetch(`/__mockforge/fixtures/${fixtureToMove.id}/move`, {
                          method: 'PUT',
                          headers: {
                            'Content-Type': 'application/json',
                          },
                          body: JSON.stringify({ new_path: newFixturePath }),
                        });
                        setIsMoveDialogOpen(false);
                        setNewFixturePath('');
                        refetch();
                      } catch (error) {
                        console.error('Error moving fixture:', error);
                      }
                    }}
                    disabled={!newFixturePath.trim()}
                  >
                    Move
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
