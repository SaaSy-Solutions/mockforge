import React from 'react';
import { AlertCircle, Save, CheckCircle2 } from 'lucide-react';
import { cn } from '@/utils/cn';

export interface UnsavedChangesIndicatorProps {
  /** Whether there are unsaved changes */
  hasUnsavedChanges: boolean;
  /** Whether save is in progress */
  isSaving?: boolean;
  /** Last saved timestamp */
  lastSaved?: Date | null;
  /** Custom message */
  message?: string;
  /** Show timestamp */
  showTimestamp?: boolean;
  /** Additional className */
  className?: string;
}

/**
 * Visual indicator for unsaved changes in forms
 * 
 * @example
 * ```tsx
 * <UnsavedChangesIndicator
 *   hasUnsavedChanges={hasUnsavedChanges}
 *   isSaving={isSaving}
 *   lastSaved={lastSaved}
 * />
 * ```
 */
export function UnsavedChangesIndicator({
  hasUnsavedChanges,
  isSaving = false,
  lastSaved,
  message,
  showTimestamp = true,
  className,
}: UnsavedChangesIndicatorProps) {
  if (!hasUnsavedChanges && !isSaving && !lastSaved) {
    return null;
  }

  const formatTimestamp = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (seconds < 60) {
      return 'just now';
    } else if (minutes < 60) {
      return `${minutes}m ago`;
    } else if (hours < 24) {
      return `${hours}h ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  return (
    <div
      className={cn(
        'flex items-center gap-2 text-sm',
        hasUnsavedChanges && 'text-amber-600 dark:text-amber-400',
        isSaving && 'text-blue-600 dark:text-blue-400',
        !hasUnsavedChanges && !isSaving && 'text-green-600 dark:text-green-400',
        className
      )}
    >
      {isSaving ? (
        <>
          <Save className="h-4 w-4 animate-pulse" />
          <span>Saving...</span>
        </>
      ) : hasUnsavedChanges ? (
        <>
          <AlertCircle className="h-4 w-4" />
          <span>{message || 'Unsaved changes'}</span>
          {showTimestamp && lastSaved && (
            <span className="text-muted-foreground text-xs">
              (last saved {formatTimestamp(lastSaved)})
            </span>
          )}
        </>
      ) : lastSaved ? (
        <>
          <CheckCircle2 className="h-4 w-4" />
          <span>Saved</span>
          {showTimestamp && (
            <span className="text-muted-foreground text-xs">
              {formatTimestamp(lastSaved)}
            </span>
          )}
        </>
      ) : null}
    </div>
  );
}
