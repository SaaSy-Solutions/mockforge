import React from 'react';
import { cn } from '../../../utils/cn';
import { X } from 'lucide-react';
import { ModernCard } from './Card';

interface ModalProps {
  open: boolean;
  onClose?: () => void;
  onOpenChange?: (open: boolean) => void;
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export function Modal({ open, onClose, onOpenChange, title, children, className }: ModalProps) {
  const handleClose = () => {
    onClose?.();
    onOpenChange?.(false);
  };
  if (!open) return null;

  return (
    <>
      <div
        className="fixed inset-0 bg-bg-overlay z-40"
        onClick={handleClose}
      />
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <ModernCard className={cn('max-w-md w-full max-h-[90vh] overflow-y-auto', className)}>
          {title && (
            <div className="flex items-center justify-between p-6 border-b border-border">
              <h3 className="text-lg font-semibold text-foreground">
                {title}
              </h3>
              <button
                onClick={handleClose}
                className="p-2 rounded-lg hover:bg-accent hover:text-accent-foreground text-muted-foreground"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
          )}
          <div className="p-6">
            {children}
          </div>
        </ModernCard>
      </div>
    </>
  );
}
