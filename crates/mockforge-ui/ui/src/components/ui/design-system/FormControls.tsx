import React from 'react';
import { cn } from '../../../utils/cn';

const inputBase =
  'w-full rounded-xl border-2 border-input bg-background text-foreground placeholder:text-muted-foreground ' +
  'focus:border-ring focus:outline-none focus:ring-4 focus:ring-ring/20 ' +
  'hover:border-ring/50 transition-all duration-200 ease-out shadow-sm hover:shadow-md focus:shadow-lg';

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
    <input className={cn(inputBase, sizes[size], className)} {...props} />
  );
}

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
      className={cn(inputBase, 'resize-none', sizes[size], className)}
      {...props}
    />
  );
}

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
      className={cn(inputBase, 'cursor-pointer', sizes[size], className)}
      {...props}
    >
      {children}
    </select>
  );
}

interface LabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  required?: boolean;
}

export function Label({ className, required, children, ...props }: LabelProps) {
  return (
    <label
      className={cn(
        'block text-lg font-medium text-foreground mb-2',
        className
      )}
      {...props}
    >
      {children}
      {required && <span className="text-destructive ml-1">*</span>}
    </label>
  );
}

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
        'h-4 w-4 rounded border-input text-primary focus:ring-ring',
        disabled && 'opacity-50 cursor-not-allowed',
        className
      )}
    />
  );
}

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
        className="text-primary focus:ring-ring"
      />
      <span className="text-sm text-foreground">{children}</span>
    </label>
  );
}

interface SwitchProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  className?: string;
}

export function Switch({ checked, onCheckedChange, className }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={cn(
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
        checked ? 'bg-primary' : 'bg-muted',
        className
      )}
      onClick={() => onCheckedChange(!checked)}
    >
      <span
        className={cn(
          'inline-block h-4 w-4 transform rounded-full bg-background transition-transform',
          checked ? 'translate-x-6' : 'translate-x-1'
        )}
      />
    </button>
  );
}
