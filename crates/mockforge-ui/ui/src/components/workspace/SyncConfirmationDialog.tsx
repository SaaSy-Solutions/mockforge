import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '../ui/Dialog';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import { AlertTriangle, FileText, Plus, Edit, Trash2 } from 'lucide-react';

interface SyncChange {
  change_type: string;
  path: string;
  description: string;
  requires_confirmation: boolean;
}

interface SyncConfirmationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaceId: string;
  changes: SyncChange[];
  onConfirm: (applyAll: boolean) => Promise<void>;
  loading?: boolean;
}

export function SyncConfirmationDialog({
  open,
  onOpenChange,
  workspaceId,
  changes,
  onConfirm,
  loading = false
}: SyncConfirmationDialogProps) {
  const [applyAll, setApplyAll] = useState(false);

  const getChangeIcon = (changeType: string) => {
    switch (changeType.toLowerCase()) {
      case 'created':
        return <Plus className="h-4 w-4 text-green-500" />;
      case 'modified':
        return <Edit className="h-4 w-4 text-blue-500" />;
      case 'deleted':
        return <Trash2 className="h-4 w-4 text-red-500" />;
      default:
        return <FileText className="h-4 w-4 text-gray-500" />;
    }
  };

  const getChangeBadgeVariant = (changeType: string) => {
    switch (changeType.toLowerCase()) {
      case 'created':
        return 'success';
      case 'modified':
        return 'info';
      case 'deleted':
        return 'danger';
      default:
        return 'secondary';
    }
  };

  const handleConfirm = async () => {
    try {
      await onConfirm(applyAll);
      onOpenChange(false);
    } catch (error) {
      logger.error('Failed to confirm sync changes',error);
    }
  };

  const requiringConfirmation = changes.filter(c => c.requires_confirmation);
  const autoApplied = changes.filter(c => !c.requires_confirmation);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-yellow-500" />
            Confirm Directory Sync Changes
          </DialogTitle>
          <DialogDescription>
            The following changes were detected in the sync directory for workspace "{workspaceId}".
            Review and confirm which changes to apply.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 max-h-96 overflow-y-auto">
          {requiringConfirmation.length > 0 && (
            <div>
              <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-2">Changes requiring confirmation:</h4>
              <div className="space-y-2">
                {requiringConfirmation.map((change, index) => (
                  <div key={index} className="flex items-start gap-3 p-3 bg-bg-secondary rounded-lg">
                    {getChangeIcon(change.change_type)}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant={getChangeBadgeVariant(change.change_type)}>
                          {change.change_type}
                        </Badge>
                        <code className="text-sm text-gray-600 dark:text-gray-400 truncate">
                          {change.path}
                        </code>
                      </div>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {change.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {autoApplied.length > 0 && (
            <div>
              <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-2">Changes that will be applied automatically:</h4>
              <div className="space-y-2">
                {autoApplied.map((change, index) => (
                  <div key={index} className="flex items-start gap-3 p-3 bg-bg-tertiary rounded-lg opacity-75">
                    {getChangeIcon(change.change_type)}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant={getChangeBadgeVariant(change.change_type)}>
                          {change.change_type}
                        </Badge>
                        <code className="text-sm text-gray-600 dark:text-gray-400 truncate">
                          {change.path}
                        </code>
                      </div>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {change.description}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {changes.length === 0 && (
            <div className="text-center py-8 text-gray-600 dark:text-gray-400">
              No changes detected.
            </div>
          )}
        </div>

        {requiringConfirmation.length > 0 && (
          <div className="flex items-center gap-2 p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg">
            <input
              type="checkbox"
              id="apply-all"
              checked={applyAll}
              onChange={(e) => setApplyAll(e.target.checked)}
              className="rounded border-border"
            />
            <label htmlFor="apply-all" className="text-sm text-gray-900 dark:text-gray-100">
              Apply all changes including those requiring confirmation
            </label>
          </div>
        )}

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={loading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={loading || changes.length === 0}
          >
            {loading ? 'Applying...' : 'Confirm Changes'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}