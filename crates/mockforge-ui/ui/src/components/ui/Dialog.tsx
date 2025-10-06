import { logger } from '@/utils/logger';
import React, { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../../utils/cn';
import { Button } from './button';
import { X } from 'lucide-react';

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
}

interface DialogContentProps {
  children: React.ReactNode;
  className?: string;
}

interface DialogHeaderProps {
  children: React.ReactNode;
  className?: string;
}

interface DialogTitleProps {
  children: React.ReactNode;
  className?: string;
  id?: string;
}

interface DialogDescriptionProps {
  children: React.ReactNode;
  className?: string;
  id?: string;
}

interface DialogFooterProps {
  children: React.ReactNode;
  className?: string;
}

export function Dialog({ open, onOpenChange, children }: DialogProps) {
  return (
    <DialogContext.Provider value={{ open, onOpenChange }}>
      {children}
    </DialogContext.Provider>
  );
}

const DialogContext = React.createContext<{ open: boolean; onOpenChange: (open: boolean) => void } | null>(null);

const useDialogContext = () => {
  const context = React.useContext(DialogContext);
  if (!context) {
    throw new Error('Dialog components must be used within a Dialog');
  }
  return context;
};

export function DialogContent({ children, className }: DialogContentProps) {
  const { open, onOpenChange } = useDialogContext();
  const dialogRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedElement = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    // Store the element that had focus before the dialog opened
    previouslyFocusedElement.current = document.activeElement as HTMLElement;

    // Focus the dialog
    if (dialogRef.current) {
      dialogRef.current.focus();
    }

    // Prevent body scroll
    document.body.style.overflow = 'hidden';

    // Cleanup
    return () => {
      document.body.style.overflow = '';
      // Restore focus to the previously focused element
      if (previouslyFocusedElement.current) {
        previouslyFocusedElement.current.focus();
      }
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onOpenChange(false);
      }

      // Focus trap
      if (e.key === 'Tab') {
        if (!dialogRef.current) return;

        const focusableElements = dialogRef.current.querySelectorAll(
          'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
        );

        const firstElement = focusableElements[0] as HTMLElement;
        const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

        if (e.shiftKey && document.activeElement === firstElement) {
          e.preventDefault();
          lastElement?.focus();
        } else if (!e.shiftKey && document.activeElement === lastElement) {
          e.preventDefault();
          firstElement?.focus();
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onOpenChange]);

  if (!open) return null;

  const dialogContent = (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/50 backdrop-blur-sm"
        onClick={() => onOpenChange(false)}
        aria-hidden="true"
      />

      {/* Dialog */}
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        className={cn(
          "relative bg-bg-primary border border-border rounded-lg shadow-xl max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto focus:outline-none",
          className
        )}
      >
        <div className="p-6">
          {children}
        </div>
      </div>
    </div>
  );

  return createPortal(dialogContent, document.body);
}

export function DialogHeader({ children, className }: DialogHeaderProps) {
  return (
    <div className={cn("flex items-center justify-between pb-4 border-b border-border", className)}>
      {children}
    </div>
  );
}

export function DialogTitle({ children, className, id = "dialog-title" }: DialogTitleProps) {
  return (
    <h2 id={id} className={cn("text-lg font-semibold text-gray-900 dark:text-gray-100", className)}>
      {children}
    </h2>
  );
}

export function DialogDescription({ children, className, id = "dialog-description" }: DialogDescriptionProps) {
  return (
    <p id={id} className={cn("text-sm text-gray-600 dark:text-gray-400 mt-1", className)}>
      {children}
    </p>
  );
}

export function DialogFooter({ children, className }: DialogFooterProps) {
  return (
    <div className={cn("flex items-center justify-end gap-3 pt-4 border-t border-border", className)}>
      {children}
    </div>
  );
}

export function DialogTrigger({ children, onClick, asChild }: { children: React.ReactNode; onClick?: () => void; asChild?: boolean }) {
  const { onOpenChange } = useDialogContext();

  const handleClick = () => {
    onOpenChange(true);
    onClick?.();
  };

  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children, {
      ...children.props,
      onClick: handleClick,
    } as any);
  }

  return (
    <div onClick={handleClick}>
      {children}
    </div>
  );
}

export function DialogClose({ onClick, className }: { onClick?: () => void; className?: string }) {
  const { onOpenChange } = useDialogContext();

  const handleClick = () => {
    onOpenChange(false);
    onClick?.();
  };

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn("h-8 w-8 p-0 hover:bg-bg-tertiary", className)}
      onClick={handleClick}
    >
      <X className="h-4 w-4" />
    </Button>
  );
}
