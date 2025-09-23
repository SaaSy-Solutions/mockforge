import React, { useState } from 'react';
import { FixtureTree } from './FixtureTree';
import { FixtureEditor } from './FixtureEditor';
import { FixtureDiffViewer } from './FixtureDiffViewer';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { useFixtureStore } from '../../stores/useFixtureStore';
import type { FixtureDiff } from '../../types';

export function FixturesPanel() {
  const {
    fixtures,
    selectedFixture,
    diffHistory,
    selectFixture,
    updateFixture,
    renameFixture,
    moveFixture,
    deleteFixture,
    clearSelection,
    generateDiff
  } = useFixtureStore();

  const [searchTerm, setSearchTerm] = useState('');
  const [showDiff, setShowDiff] = useState<FixtureDiff | null>(null);
  const [previewMode, setPreviewMode] = useState(false);

  // Filter fixtures based on search term
  const filteredFixtures = fixtures.filter(fixture =>
    fixture.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    fixture.path.toLowerCase().includes(searchTerm.toLowerCase()) ||
    (fixture.content as string).toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleSaveFixture = (fixtureId: string, content: string) => {
    const fixture = fixtures.find(f => f.id === fixtureId);
    if (fixture && content !== (fixture.content as string)) {
      // Show diff before saving
      const diff = generateDiff(fixtureId, content);
      if (diff.changes.length > 0) {
        setShowDiff(diff);
      } else {
        updateFixture(fixtureId, content);
      }
    }
  };

  const handleApplyDiff = (diff: FixtureDiff) => {
    const fixture = fixtures.find(f => f.name === diff.name);
    if (fixture && diff.new_content) {
      updateFixture(fixture.id, diff.new_content);
    }
    setShowDiff(null);
  };

  const handleDeleteFixture = (fixtureId: string) => {
    if (window.confirm('Are you sure you want to delete this fixture?')) {
      deleteFixture(fixtureId);
    }
  };

  const getTotalSize = () => {
    return filteredFixtures.reduce((total, fixture) => total + (fixture.size_bytes || 0), 0);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex flex-col gap-4 p-6 border-b">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold">Fixtures</h2>
            <p className="text-muted-foreground">
              {filteredFixtures.length} files • {formatFileSize(getTotalSize())}
            </p>
          </div>

          <div className="flex items-center space-x-2">
            <Button
              variant={previewMode ? "default" : "outline"}
              size="sm"
              onClick={() => setPreviewMode(!previewMode)}
            >
              {previewMode ? 'Edit Mode' : 'Preview Mode'}
            </Button>
            {diffHistory.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowDiff(diffHistory[0])}
              >
                View Last Diff
              </Button>
            )}
          </div>
        </div>

        {/* Search */}
        <div className="flex items-center space-x-4">
          <Input
            placeholder="Search fixtures..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="max-w-md"
          />
          <span className="text-sm text-muted-foreground">
            {searchTerm && `${filteredFixtures.length} results`}
          </span>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 flex">
        {/* Left Panel - File Tree */}
        <div className="w-80 border-r bg-muted/30 p-4">
          <FixtureTree
            fixtures={filteredFixtures}
            onSelectFixture={selectFixture}
            onRenameFixture={renameFixture}
            onMoveFixture={moveFixture}
            onDeleteFixture={handleDeleteFixture}
            selectedFixtureId={selectedFixture?.id}
          />

          {/* Recent Changes */}
          {diffHistory.length > 0 && (
            <div className="mt-4 border rounded-lg bg-card">
              <div className="p-3 border-b">
                <h4 className="font-semibold text-sm">Recent Changes</h4>
              </div>
              <div className="p-2 space-y-1">
                {diffHistory.slice(0, 5).map((diff) => (
                  <button
                    key={diff.id}
                    onClick={() => setShowDiff(diff)}
                    className="w-full text-left p-2 text-xs hover:bg-accent rounded"
                  >
                    <div className="font-medium truncate">{diff.name}</div>
                    <div className="text-muted-foreground">
                      {diff.changes.length} changes • {new Date(diff.timestamp).toLocaleTimeString()}
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Right Panel - Editor/Viewer */}
        <div className="flex-1">
          {selectedFixture ? (
            <FixtureEditor
              fixture={selectedFixture}
              onSave={handleSaveFixture}
              onClose={clearSelection}
              readOnly={previewMode}
            />
          ) : (
            <div className="h-full flex items-center justify-center text-center">
              <div className="space-y-4">
                <div className="text-6xl">📄</div>
                <div>
                  <h3 className="text-lg font-semibold">No fixture selected</h3>
                  <p className="text-muted-foreground">
                    Select a fixture from the tree to view or edit its content
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Diff Modal */}
      {showDiff && (
        <FixtureDiffViewer
          diff={showDiff}
          onClose={() => setShowDiff(null)}
          onApply={handleApplyDiff}
        />
      )}
    </div>
  );
}
