import React from 'react';

interface ContextMenuItemData {
  label: string;
  onClick: () => void;
  icon?: React.ReactNode;
}

interface ContextMenuProps {
  children: React.ReactNode;
  items: ContextMenuItemData[];
  position: { x: number; y: number };
  onClose: () => void;
}

export function ContextMenu({ children, items, position, onClose }: ContextMenuProps) {
  return (
    <>
      {children}
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
      />
      <div
        className="absolute z-50 min-w-48 rounded-md shadow-lg bg-white dark:bg-gray-800 ring-1 ring-black ring-opacity-5 py-1"
        style={{ left: position.x, top: position.y }}
      >
        {items.map((item, index) => (
          <button
            key={index}
            className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 hover:text-gray-900 dark:hover:text-gray-100 transition-colors duration-150"
            onClick={() => {
              item.onClick();
              onClose();
            }}
          >
            {item.icon && <span className="mr-2">{item.icon}</span>}
            {item.label}
          </button>
        ))}
      </div>
    </>
  );
}

export function ContextMenuContent({ children }: { children: React.ReactNode }) {
  return <div>{children}</div>;
}

export function ContextMenuItem({ children, onClick, className }: { children: React.ReactNode; onClick?: () => void; className?: string }) {
  return <div onClick={onClick} className={className}>{children}</div>;
}

export function ContextMenuTrigger({ children }: { children: React.ReactNode }) {
  return <div>{children}</div>;
}
