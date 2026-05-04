import React from 'react';
import { cn } from '../../../utils/cn';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'outline' | 'ghost' | 'destructive';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
}

export function Button({
  children,
  variant = 'primary',
  size = 'md',
  className,
  loading,
  disabled,
  ...props
}: ButtonProps) {
  const variants = {
    primary:
      'bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm',
    secondary:
      'bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-sm hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm',
    danger:
      'bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm',
    success:
      'bg-success text-text-inverse hover:bg-success-600 shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm',
    outline:
      'border border-input bg-background hover:bg-accent hover:text-accent-foreground text-foreground',
    ghost:
      'bg-transparent hover:bg-accent hover:text-accent-foreground text-foreground',
    destructive:
      'bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm',
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  };

  return (
    <button
      className={cn(
        'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed',
        variants[variant],
        sizes[size],
        className
      )}
      disabled={disabled || loading}
      {...props}
    >
      {loading && (
        <svg className="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      )}
      {children}
    </button>
  );
}
