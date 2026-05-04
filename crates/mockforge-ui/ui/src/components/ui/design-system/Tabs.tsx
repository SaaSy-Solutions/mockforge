import React from 'react';
import { cn } from '../../../utils/cn';

interface TabsProps {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  children: React.ReactNode;
  className?: string;
}

export function Tabs({ value, defaultValue, onValueChange, children, className }: TabsProps) {
  const [internalValue, setInternalValue] = React.useState(defaultValue || value || '');
  const currentValue = value || internalValue;

  const handleValueChange = (newValue: string) => {
    setInternalValue(newValue);
    onValueChange?.(newValue);
  };

  return (
    <div className={cn('', className)} data-value={currentValue}>
      {React.Children.map(children, (child) => {
        if (React.isValidElement(child)) {
          return React.cloneElement(child as React.ReactElement<{ value?: string; onValueChange?: (v: string) => void }>, {
            value: currentValue,
            onValueChange: handleValueChange,
          });
        }
        return child;
      })}
    </div>
  );
}

interface TabsContentProps {
  value: string;
  children: React.ReactNode;
  className?: string;
}

export function TabsContent({ value, children, className }: TabsContentProps) {
  return (
    <div data-value={value} className={className}>
      {children}
    </div>
  );
}

export function TabsList({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn('flex space-x-1 border-b border-border', className)}>
      {children}
    </div>
  );
}

interface TabsTriggerProps {
  value: string;
  children: React.ReactNode;
  className?: string;
}

export function TabsTrigger({ value, children, className }: TabsTriggerProps) {
  return (
    <button
      className={cn(
        'px-4 py-2 text-sm font-medium rounded-t-lg border-b-2 border-transparent text-muted-foreground hover:text-foreground hover:border-border transition-colors duration-200 focus:outline-none focus:border-ring',
        className
      )}
      data-value={value}
    >
      {children}
    </button>
  );
}
