import React from 'react';
import { Moon, Sun, Monitor } from 'lucide-react';
import { Button } from './button';
import { useThemePaletteStore } from '../../stores/useThemePaletteStore';
import { cn } from '../../utils/cn';

interface ThemeToggleProps {
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

export function ThemeToggle({ className, size = 'md' }: ThemeToggleProps) {
  const { theme, setTheme } = useThemeStore();

  const iconSizes = {
    sm: 'h-4 w-4',
    md: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  return (
    <div className={cn('flex items-center rounded-lg border border-border bg-bg-primary p-1', className)}>
      <Button
        variant={theme === 'light' ? 'default' : 'ghost'}
        size="sm"
        onClick={() => setTheme('light')}
        className={cn(
          'rounded-md transition-all duration-200',
          theme === 'light' && 'bg-brand-50 text-brand-700 shadow-sm hover:bg-brand-100 dark:bg-brand-900/20 dark:text-brand-300'
        )}
        aria-label="Switch to light mode"
      >
        <Sun className={iconSizes[size]} />
      </Button>

      <Button
        variant={theme === 'system' ? 'default' : 'ghost'}
        size="sm"
        onClick={() => setTheme('system')}
        className={cn(
          'rounded-md transition-all duration-200',
          theme === 'system' && 'bg-brand-50 text-brand-700 shadow-sm hover:bg-brand-100 dark:bg-brand-900/20 dark:text-brand-300'
        )}
        aria-label="Switch to system theme"
      >
        <Monitor className={iconSizes[size]} />
      </Button>

      <Button
        variant={theme === 'dark' ? 'default' : 'ghost'}
        size="sm"
        onClick={() => setTheme('dark')}
        className={cn(
          'rounded-md transition-all duration-200',
          theme === 'dark' && 'bg-brand-50 text-brand-700 shadow-sm hover:bg-brand-100 dark:bg-brand-900/20 dark:text-brand-300'
        )}
        aria-label="Switch to dark mode"
      >
        <Moon className={iconSizes[size]} />
      </Button>
    </div>
  );
}

// Simple toggle version for minimal UI
export function SimpleThemeToggle({ className, size = 'md' }: ThemeToggleProps) {
  const { toggleTheme, resolvedTheme } = useThemeStore();

  const sizeClasses = {
    sm: 'h-8 w-8',
    md: 'h-9 w-9',
    lg: 'h-10 w-10',
  };

  const iconSizes = {
    sm: 'h-4 w-4',
    md: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={toggleTheme}
      className={cn(
        'btn-hover transition-all duration-200',
        sizeClasses[size],
        className
      )}
      aria-label={`Switch to ${resolvedTheme === 'light' ? 'dark' : 'light'} mode`}
    >
      {resolvedTheme === 'light' ? (
        <Moon className={iconSizes[size]} />
      ) : (
        <Sun className={iconSizes[size]} />
      )}
    </Button>
  );
}
