import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { Button } from '../ui/button';
import type { FixtureInfo } from '../../types';

interface FixtureEditorProps {
  fixture: FixtureInfo;
  onSave: (fixtureId: string, content: string) => void;
  onClose: () => void;
  readOnly?: boolean;
}

export function FixtureEditor({ fixture, onSave, onClose, readOnly = false }: FixtureEditorProps) {
  const [content, setContent] = useState(typeof fixture.content === 'string' ? fixture.content : JSON.stringify(fixture.content, null, 2));
  const [hasChanges, setHasChanges] = useState(false);

  const handleContentChange = (newContent: string) => {
    setContent(newContent);
    setHasChanges(newContent !== fixture.content);
  };

  const handleSave = () => {
    if (hasChanges) {
      onSave(fixture.id, content || '');
      setHasChanges(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === 's') {
      e.preventDefault();
      handleSave();
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Simple syntax highlighting for JSON
  const renderContent = () => {
    if (readOnly) {
      return (
        <div className="h-full p-4 font-mono text-sm bg-muted/30 rounded border overflow-auto">
          <pre>{content as string}</pre>
        </div>
      );
    }

    return (
      <textarea
        value={content as string}
        onChange={(e) => handleContentChange(e.target.value)}
        onKeyDown={handleKeyDown}
        className="w-full h-full p-4 font-mono text-sm bg-background border rounded resize-none focus:outline-none focus:ring-2 focus:ring-ring"
        placeholder="Enter fixture content..."
        spellCheck={false}
      />
    );
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center space-x-4">
          <div>
            <h3 className="font-semibold">{fixture.name}</h3>
            <div className="flex items-center space-x-4 text-xs text-muted-foreground">
              <span>{fixture.path}</span>
              <span>{formatFileSize(fixture.size_bytes || 0)}</span>
              <span>Modified: {new Date(fixture.last_modified || fixture.updatedAt).toLocaleString()}</span>
              {fixture.route_path && (
                <>
                  <span>•</span>
                  <span className="font-mono">
                    {fixture.method} {fixture.route_path}
                  </span>
                </>
              )}
            </div>
          </div>
          {hasChanges && (
            <span className="text-xs bg-yellow-100 text-yellow-800 px-2 py-1 rounded">
              Unsaved changes
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2">
          {!readOnly && (
            <Button
              onClick={handleSave}
              disabled={!hasChanges}
              size="sm"
            >
              Save (Ctrl+S)
            </Button>
          )}
          <Button variant="outline" onClick={onClose} size="sm">
            Close
          </Button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-4">
        {renderContent()}
      </div>

      {/* Footer */}
      <div className="p-4 border-t bg-muted/30">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center space-x-4">
            <span>Lines: {(content as string).split('\n').length}</span>
            <span>Characters: {(content as string).length}</span>
            <span>Version: {fixture.version as string}</span>
          </div>
          {!readOnly && (
            <div className="flex items-center space-x-2">
              <span>Press Ctrl+S to save</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
