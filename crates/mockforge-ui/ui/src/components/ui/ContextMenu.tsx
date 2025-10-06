import { logger } from '@/utils/logger';
import React, { useEffect, useRef } from 'react';
import { cn } from '../../utils/cn';

export interface ContextMenuItem {
  label: string;
  onClick: () => void;
  icon?: React.ReactNode;
  disabled?: boolean;
  separator?: boolean;
}

interface ContextMenuWithItemsProps {
  items: ContextMenuItem[];
  position: { x: number; y: number };
  onClose: () => void;
  className?: string;
}

export function ContextMenuWithItems({ items, position, onClose, className }: ContextMenuWithItemsProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [onClose]);

  // Adjust position to keep menu within viewport
  const adjustedPosition = React.useMemo(() => {
    if (!menuRef.current) return position;

    const menuRect = menuRef.current.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    let x = position.x;
    let y = position.y;

    // Adjust horizontal position
    if (x + menuRect.width > viewportWidth) {
      x = viewportWidth - menuRect.width - 10;
    }

    // Adjust vertical position
    if (y + menuRect.height > viewportHeight) {
      y = viewportHeight - menuRect.height - 10;
    }

    return { x, y };
  }, [position]);

  return (
    <div
      ref={menuRef}
      className={cn(
        "fixed z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[200px]",
        className
      )}
      style={{
        left: adjustedPosition.x,
        top: adjustedPosition.y,
      }}
    >
      {items.map((item, index) => (
        <React.Fragment key={index}>
          {item.separator && <div className="border-t border-border my-1" />}
          <button
            onClick={() => {
              if (!item.disabled) {
                item.onClick();
                onClose();
              }
            }}
            disabled={item.disabled}
            className={cn(
              "w-full px-3 py-2 text-left text-sm hover:bg-accent hover:text-accent-foreground flex items-center gap-2",
              "focus:outline-none focus:bg-accent focus:text-accent-foreground",
              item.disabled && "opacity-50 cursor-not-allowed"
            )}
          >
            {item.icon && <span className="w-4 h-4 flex items-center justify-center">{item.icon}</span>}
            <span>{item.label}</span>
          </button>
        </React.Fragment>
      ))}
    </div>
  );
}

// Compound component pattern for ContextMenu
export function ContextMenu({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}

export function ContextMenuTrigger({ children, onContextMenu }: { children: React.ReactNode; onContextMenu?: (e: React.MouseEvent) => void }) {
  return (
    <div onContextMenu={onContextMenu}>
      {children}
    </div>
  );
}

export function ContextMenuContent({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn("bg-white border border-gray-200 rounded-md shadow-lg py-1 min-w-[200px]", className)}>
      {children}
    </div>
  );
}
