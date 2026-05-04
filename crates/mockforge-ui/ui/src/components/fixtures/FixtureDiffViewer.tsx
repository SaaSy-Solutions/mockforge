import { logger } from '@/utils/logger';
import React from 'react';
import { Button } from '../ui/button';
import type { FixtureDiff, DiffChange } from '../../types';

interface FixtureDiffViewerProps {
  diff: FixtureDiff;
  onClose: () => void;
  onApply?: (diff: FixtureDiff) => void;
}

export function FixtureDiffViewer({ diff, onClose, onApply }: FixtureDiffViewerProps) {
  const renderDiffLine = (change: DiffChange, index: number) => {
    const getLineStyle = (type: DiffChange['type']) => {
      switch (type) {
        case 'add':
          return 'bg-success-50 border-l-4 border-success text-success-700';
        case 'remove':
          return 'bg-danger-50 border-l-4 border-destructive text-danger-700';
        case 'modify':
          return 'bg-warning-50 border-l-4 border-warning text-warning-700';
        default:
          return '';
      }
    };

    const getTypeSymbol = (type: DiffChange['type']) => {
      switch (type) {
        case 'add':
          return '+';
        case 'remove':
          return '-';
        case 'modify':
          return '~';
        default:
          return ' ';
      }
    };

    return (
      <div key={index} className={`p-2 font-mono text-sm ${getLineStyle(change.type)}`}>
        <span className="inline-block w-8 text-center font-bold">
          {getTypeSymbol(change.type)}
        </span>
        <span className="inline-block w-12 text-right pr-2 text-muted-foreground">
          {change.line_number}
        </span>
        <span>{change.content}</span>
        {change.type === 'modify' && change.old_content && (
          <div className="mt-1 pl-20 text-danger-600">
            <span className="inline-block w-8 text-center">-</span>
            <span>{change.old_content}</span>
          </div>
        )}
      </div>
    );
  };

  const addedLines = diff.changes.filter(c => c.type === 'add').length;
  const removedLines = diff.changes.filter(c => c.type === 'remove').length;
  const modifiedLines = diff.changes.filter(c => c.type === 'modify').length;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg border shadow-lg w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="p-4 border-b flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Fixture Diff: {diff.name}</h2>
            <div className="flex items-center space-x-4 text-sm text-muted-foreground mt-1">
              <span className="text-success-600">+{addedLines} added</span>
              <span className="text-danger-600">-{removedLines} removed</span>
              <span className="text-warning-600">~{modifiedLines} modified</span>
              <span>• {new Date(diff.timestamp).toLocaleString()}</span>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            {onApply && (
              <Button onClick={() => onApply(diff)} size="sm">
                Apply Changes
              </Button>
            )}
            <Button variant="outline" onClick={onClose} size="sm">
              Close
            </Button>
          </div>
        </div>

        {/* Diff Content */}
        <div className="flex-1 overflow-auto">
          {diff.changes.length === 0 ? (
            <div className="p-8 text-center text-muted-foreground">
              No changes to display
            </div>
          ) : (
            <div className="divide-y">
              {diff.changes.map((change, index) => renderDiffLine(change, index))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t bg-muted/50">
          <div className="text-xs text-muted-foreground">
            <div className="flex items-center space-x-4">
              <span>Legend:</span>
              <span className="text-success-600">+ Added lines</span>
              <span className="text-danger-600">- Removed lines</span>
              <span className="text-warning-600">~ Modified lines</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
