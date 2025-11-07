import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { cn } from '../../utils/cn';
import { Button } from '../ui/button';
import { SimpleThemeToggle } from '../ui/ThemeToggle';
import { UserProfile } from '../auth/UserProfile';
import { Logo } from '../ui/Logo';
import { Input } from '../ui/input';
import { useLogStore } from '../../stores/useLogStore';
import { useServiceStore } from '../../stores/useServiceStore';
import { useAppShortcuts } from '../../hooks/useKeyboardNavigation';
import { useSkipLinks } from '../../hooks/useFocusManagement';
import {
  BarChart3,
  Server,
  Database,
  FileText,
  Activity,
  TestTube,
  Settings,
  Menu,
  RefreshCw,
  X,
  Puzzle,
  FolderOpen,
  Import,
  Link2,
  GitBranch,
  Radio,
  Zap,
  Shield,
  Eye,
  Code2,
  PlayCircle,
  Network,
  Layers,
  Store,
  Package,
  GitBranch as GraphIcon,
  CheckCircle2,
} from 'lucide-react';

interface AppShellProps {
  children: React.ReactNode;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onRefresh: () => void;
}

const navItems = [
  // Core
  { id: 'dashboard', label: 'Dashboard', icon: BarChart3 },
  { id: 'workspaces', label: 'Workspaces', icon: FolderOpen },

  // Services & Data
  { id: 'services', label: 'Services', icon: Server },
  { id: 'fixtures', label: 'Fixtures', icon: Database },

  // Orchestration
  { id: 'chains', label: 'Chains', icon: Link2 },
  { id: 'graph', label: 'Graph', icon: GraphIcon },
  { id: 'state-machine-editor', label: 'State Machines', icon: GitBranch },
  { id: 'orchestration-builder', label: 'Orchestration Builder', icon: GitBranch },
  { id: 'orchestration-execution', label: 'Orchestration Execution', icon: PlayCircle },

  // Observability & Monitoring
  { id: 'observability', label: 'Observability', icon: Eye },
  { id: 'logs', label: 'Logs', icon: FileText },
  { id: 'traces', label: 'Traces', icon: Network },
  { id: 'metrics', label: 'Metrics', icon: Activity },
  { id: 'analytics', label: 'Analytics', icon: BarChart3 },
  { id: 'verification', label: 'Verification', icon: CheckCircle2 },

  // Testing
  { id: 'testing', label: 'Testing', icon: TestTube },
  { id: 'test-generator', label: 'Test Generator', icon: Code2 },
  { id: 'test-execution', label: 'Test Execution', icon: PlayCircle },
  { id: 'integration-test-builder', label: 'Integration Tests', icon: Layers },

  // Chaos & Resilience
  { id: 'chaos', label: 'Chaos Engineering', icon: Zap },
  { id: 'resilience', label: 'Resilience', icon: Shield },
  { id: 'recorder', label: 'Recorder', icon: Radio },

  // Import & Templates
  { id: 'import', label: 'Import', icon: Import },
  { id: 'template-marketplace', label: 'Template Marketplace', icon: Store },

  // Plugins
  { id: 'plugins', label: 'Plugins', icon: Puzzle },
  { id: 'plugin-registry', label: 'Plugin Registry', icon: Package },

  // Configuration
  { id: 'config', label: 'Config', icon: Settings },
];

export function AppShell({ children, activeTab, onTabChange, onRefresh }: AppShellProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const { setFilter: setLogFilter } = useLogStore();
  const { setGlobalSearch } = useServiceStore();
  const [globalQuery, setGlobalQuery] = useState('');
  const [isMac, setIsMac] = useState(false);

  // Setup keyboard shortcuts
  useAppShortcuts({
    onSearch: () => {
      const searchInput = document.getElementById('global-search-input') as HTMLInputElement;
      if (searchInput) {
        searchInput.focus();
        searchInput.select();
      }
    },
  });

  // Skip links functionality
  const { createSkipLink } = useSkipLinks();

  React.useEffect(() => {
    setIsMac(navigator.userAgent.toUpperCase().indexOf('MAC') >= 0);
  }, []);

  return (
    <div className="min-h-screen bg-bg-secondary">
      {/* Skip Links */}
      <nav className="sr-only focus-within:not-sr-only">
        <a {...createSkipLink('main-navigation', 'Skip to navigation')} />
        <a {...createSkipLink('main-content', 'Skip to main content')} />
        <a {...createSkipLink('global-search-input', 'Skip to search')} />
      </nav>

      {sidebarOpen && (
        <div className="fixed inset-0 z-50 md:hidden">
          <div
            className="fixed inset-0 bg-black/50 backdrop-blur-sm animate-fade-in"
            onClick={() => setSidebarOpen(false)}
          />
          <aside className="fixed left-0 top-0 h-full w-80 max-w-[90vw] bg-background border-r border-gray-200 dark:border-gray-800 shadow-2xl animate-slide-in-left">
            <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800 bg-card">
              <div className="flex items-center gap-3">
                <Logo variant="icon" size="md" />
                <span className="text-xl font-bold text-gray-900 dark:text-gray-100">MockForge</span>
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setSidebarOpen(false)}
                className="h-10 w-10 p-0 rounded-full spring-hover"
              >
                <X className="h-5 w-5" />
              </Button>
            </div>
            <nav className="p-6 space-y-2 overflow-y-auto">
              {navItems.map((item, index) => {
                const Icon = item.icon;
                return (
                  <Button
                    key={item.id}
                    variant={activeTab === item.id ? 'default' : 'ghost'}
                    className={cn(
                      'w-full justify-start gap-4 h-12 text-lg nav-item-hover focus-ring spring-hover',
                      'animate-slide-in-up',
                      activeTab === item.id
                        ? 'bg-brand text-white shadow-md'
                        : 'text-foreground/80 dark:text-gray-400 hover:text-foreground dark:hover:text-gray-100 hover:bg-muted/50'
                    )}
                    style={{ animationDelay: `${index * 50}ms` }}
                    onClick={() => {
                      onTabChange(item.id);
                      setSidebarOpen(false);
                    }}
                  >
                    <Icon className="h-5 w-5" />
                    {item.label}
                  </Button>
                );
              })}
            </nav>
          </aside>
        </div>
      )}

      <div className="flex">
        {/* Desktop Sidebar - Always visible on md and larger screens */}
        <aside className="hidden md:flex md:w-64 md:flex-col md:fixed md:inset-y-0 md:z-50">
          <div className="flex flex-col flex-grow bg-bg-primary border-r border-border">
            <div className="flex items-center gap-3 px-6 py-4 border-b border-border">
              <Logo variant="icon" size="md" />
              <span className="font-semibold text-gray-900 dark:text-gray-100">MockForge</span>
            </div>
            <nav id="main-navigation" className="flex-1 px-4 py-4 space-y-2" role="navigation" aria-label="Main navigation">
              {navItems.map((item) => {
                const Icon = item.icon;
                return (
                  <Button
                    key={item.id}
                    variant={activeTab === item.id ? 'default' : 'ghost'}
                    className={cn(
                      'w-full justify-start gap-3 h-11 transition-all duration-200 nav-item-hover focus-ring spring-hover',
                      activeTab === item.id
                        ? 'bg-brand text-white hover:bg-brand-600 shadow-lg'
                        : 'text-foreground/80 dark:text-gray-400 hover:text-foreground dark:hover:text-gray-100 hover:bg-muted/50'
                    )}
                    onClick={() => onTabChange(item.id)}
                  >
                    <Icon className="h-4 w-4" />
                    {item.label}
                  </Button>
                );
              })}
            </nav>
          </div>
        </aside>

        <div className="md:pl-64 flex flex-col flex-1 min-h-screen">
          <header className="sticky top-0 z-40 flex h-16 shrink-0 items-center border-b border-border bg-bg-primary shadow-sm">
            <div className="w-full max-w-[1400px] mx-auto flex items-center gap-x-4 px-4 sm:gap-x-6 sm:px-6 lg:px-8">
              <Button variant="ghost" size="sm" className="md:hidden" onClick={() => setSidebarOpen(true)}>
                <Menu className="h-5 w-5" />
              </Button>
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-sm text-gray-600 dark:text-gray-400">Home</span>
                <span className="text-gray-600 dark:text-gray-400">/</span>
                <span className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate capitalize">{navItems.find(n=>n.id===activeTab)?.label ?? activeTab}</span>
              </div>
              <div className="flex flex-1" />
              <div className="hidden sm:flex w-72 relative items-center">
                <Input
                  placeholder="Global search…"
                  id="global-search-input"
                  value={globalQuery}
                  onChange={(e)=>{
                    const q = e.target.value;
                    setGlobalQuery(q);
                    // lightweight sync across stores
                    setLogFilter({ path_pattern: q || undefined });
                    setGlobalSearch(q || undefined);
                  }}
                  onKeyDown={(e)=>{
                    if (e.key === 'Escape') {
                      setGlobalQuery('');
                      setLogFilter({ path_pattern: undefined });
                      setGlobalSearch(undefined);
                      (document.getElementById('global-search-input') as HTMLInputElement | null)?.blur();
                    }
                  }}
                />
                <span className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-[10px] text-gray-600 dark:text-gray-400 border border-border rounded px-1 py-0.5 bg-bg-primary">
                  {isMac ? '⌘K' : 'Ctrl K'}
                </span>
              </div>
              <div className="flex items-center gap-x-4 lg:gap-x-6">
                <SimpleThemeToggle />
                <Button variant="outline" size="sm" onClick={onRefresh} className="flex items-center gap-2">
                  <RefreshCw className="h-4 w-4" />
                  <span className="hidden sm:inline">Refresh</span>
                </Button>
                <UserProfile />
              </div>
            </div>
          </header>

          <main id="main-content" className="flex-1" role="main" aria-label="Main content">
            <div className="w-full max-w-[1400px] mx-auto px-6 py-6">{children}</div>
          </main>
        </div>
      </div>
    </div>
  );
}
