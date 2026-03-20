import React from 'react';
import { cn } from '../../../utils/cn';

// Input Component
interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  size?: 'sm' | 'md' | 'lg';
}

export function Input({ className, size = 'md', ...props }: InputProps) {
  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2.5 text-lg',
    lg: 'px-6 py-3.5 text-lg',
  };

  return (
    <input
      className={cn(
        'w-full rounded-xl border-2 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900',
        'text-primary placeholder-tertiary',
        'focus:border-brand focus:outline-none focus:ring-4 focus:ring-brand/20',
        'hover:border-gray-300 dark:hover:border-gray-600',
        'transition-all duration-200 ease-out',
        'shadow-sm hover:shadow-md focus:shadow-lg',
        sizes[size],
        className
      )}
      {...props}
    />
  );
}

// Textarea Component
interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  size?: 'sm' | 'md' | 'lg';
}

export function Textarea({ className, size = 'md', ...props }: TextareaProps) {
  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2.5 text-lg',
    lg: 'px-6 py-3.5 text-lg',
  };

  return (
    <textarea
      className={cn(
        'w-full rounded-xl border-2 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900',
        'text-primary placeholder-tertiary',
        'focus:border-brand focus:outline-none focus:ring-4 focus:ring-brand/20',
        'hover:border-gray-300 dark:hover:border-gray-600',
        'transition-all duration-200 ease-out',
        'shadow-sm hover:shadow-md focus:shadow-lg',
        'resize-none',
        sizes[size],
        className
      )}
      {...props}
    />
  );
}

// Select Component
interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  size?: 'sm' | 'md' | 'lg';
  placeholder?: string;
  children: React.ReactNode;
  className?: string;
}

export function Select({ className, size = 'md', children, ...props }: SelectProps) {
  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2.5 text-lg',
    lg: 'px-6 py-3.5 text-lg',
  };

  return (
    <select
      className={cn(
        'w-full rounded-xl border-2 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900',
        'text-primary',
        'focus:border-brand focus:outline-none focus:ring-4 focus:ring-brand/20',
        'hover:border-gray-300 dark:hover:border-gray-600',
        'transition-all duration-200 ease-out',
        'shadow-sm hover:shadow-md focus:shadow-lg',
        'cursor-pointer',
        sizes[size],
        className
      )}
      {...props}
    >
      {children}
    </select>
  );
}

// Label Component
interface LabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  required?: boolean;
}

export function Label({ className, required, children, ...props }: LabelProps) {
  return (
    <label
      className={cn(
        'block text-lg font-medium text-gray-900 dark:text-gray-100 mb-2',
        className
      )}
      {...props}
    >
      {children}
      {required && <span className="text-red-700 dark:text-red-500 ml-1">*</span>}
    </label>
  );
}

// Checkbox Component
interface CheckboxProps {
  id?: string;
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
}

export function Checkbox({ id, checked, onCheckedChange, disabled, className }: CheckboxProps) {
  return (
    <input
      id={id}
      type="checkbox"
      checked={checked}
      onChange={(e) => onCheckedChange?.(e.target.checked)}
      disabled={disabled}
      className={cn(
        'h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500',
        disabled && 'opacity-50 cursor-not-allowed',
        className
      )}
    />
  );
}

// RadioGroup Components
interface RadioGroupProps {
  value: string;
  onValueChange: (value: string) => void;
  children: React.ReactNode;
  className?: string;
}

export function RadioGroup({ value, onValueChange, children, className }: RadioGroupProps) {
  return (
    <div className={cn('space-y-2', className)} data-value={value}>
      {React.Children.map(children, (child) =>
        React.isValidElement(child)
          ? React.cloneElement(child, { onValueChange } as Partial<unknown>)
          : child
      )}
    </div>
  );
}

interface RadioGroupItemProps {
  value: string;
  children: React.ReactNode;
  onValueChange?: (value: string) => void;
  className?: string;
}

export function RadioGroupItem({ value, children, onValueChange, className }: RadioGroupItemProps) {
  return (
    <label className={cn('flex items-center space-x-2 cursor-pointer', className)}>
      <input
        type="radio"
        value={value}
        onChange={(e) => onValueChange?.(e.target.value)}
        className="text-blue-600 focus:ring-blue-500"
      />
      <span className="text-sm text-gray-700 dark:text-gray-300">{children}</span>
    </label>
  );
}

// Switch Component
interface SwitchProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  className?: string;
}

export function Switch({ checked, onCheckedChange, className }: SwitchProps) {
  return (
    <button
      type="button"
      className={cn(
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2',
        checked ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700',
        className
      )}
      onClick={() => onCheckedChange(!checked)}
    >
      <span
        className={cn(
          'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
          checked ? 'translate-x-6' : 'translate-x-1'
        )}
      />
    </button>
  );
}
