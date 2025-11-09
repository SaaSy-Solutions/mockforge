import { logger } from '@/utils/logger';
import React from 'react';
import { Play } from 'lucide-react';
import { cn } from '../../utils/cn';
import { useAuthStore } from '../../stores/useAuthStore';

interface NavItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  requiredRoles?: ('admin' | 'user' | 'viewer')[];
}

interface NavigationProps {
  activeTab: string;
  onTabChange: (tabId: string) => void;
}

const navItems: NavItem[] = [
  { id: 'dashboard', label: 'Dashboard', requiredRoles: ['admin', 'viewer'] },
  { id: 'services', label: 'Services', requiredRoles: ['admin'] },
  { id: 'fixtures', label: 'Fixtures', requiredRoles: ['admin'] },
  { id: 'workspaces', label: 'Workspaces', requiredRoles: ['admin'] },
  { id: 'playground', label: 'Playground', icon: <Play className="h-4 w-4" />, requiredRoles: ['admin', 'viewer'] },
  { id: 'import', label: 'Import', requiredRoles: ['admin'] },
  { id: 'logs', label: 'Live Logs', requiredRoles: ['admin', 'viewer'] },
  { id: 'metrics', label: 'Metrics', requiredRoles: ['admin', 'viewer'] },
  { id: 'testing', label: 'Testing', requiredRoles: ['admin'] },
  { id: 'time-travel', label: 'Time Travel', requiredRoles: ['admin'] },
  { id: 'config', label: 'Configuration', requiredRoles: ['admin'] },
];

export function Navigation({ activeTab, onTabChange }: NavigationProps) {
  const { user } = useAuthStore();

  const visibleItems = navItems.filter(item => {
    if (!item.requiredRoles || !user) return true;
    return item.requiredRoles.includes(user.role);
  });

  return (
    <nav className="border-b bg-background">
      <div className="flex space-x-8 px-6">
        {visibleItems.map((item) => (
          <button
            key={item.id}
            onClick={() => onTabChange(item.id)}
            className={cn(
              "border-b-2 px-1 py-4 text-sm font-medium flex items-center gap-2",
              activeTab === item.id
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:border-border hover:text-foreground"
            )}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </div>
    </nav>
  );
}
