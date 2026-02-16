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
import { useI18n } from '../../i18n/I18nProvider';
import type { Locale } from '../../i18n/translations';
import {
  BarChart3,
  Server,
  Database,
  FileJson,
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
  Brain,
  GitCompare,
  Mic,
  History,
  AlertTriangle,
  Search,
  Film,
  Copy,
  Users,
  BookOpen,
  Star,
  Layout,
  HeartPulse,
  Cloud,
  Globe,
  Key,
  CreditCard,
  LineChart,
  Wifi,
  Share2,
  Lock as LockIcon,
} from 'lucide-react';
import { GlobalConnectionStatus } from './ConnectionStatus';

interface AppShellProps {
  children: React.ReactNode;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onRefresh: () => void;
}

const navSections = [
  {
    titleKey: 'nav.core',
    items: [
      { id: 'dashboard', labelKey: 'tab.dashboard', icon: BarChart3 },
      { id: 'workspaces', labelKey: 'tab.workspaces', icon: FolderOpen },
      { id: 'federation', labelKey: 'tab.federation', icon: Share2 },
    ]
  },
  {
    titleKey: 'nav.servicesData',
    items: [
      { id: 'services', labelKey: 'tab.services', icon: Server },
      { id: 'virtual-backends', labelKey: 'tab.virtualBackends', icon: Database },
      { id: 'fixtures', labelKey: 'tab.fixtures', icon: FileJson },
      { id: 'hosted-mocks', labelKey: 'tab.hostedMocks', icon: Cloud },
      { id: 'tunnels', labelKey: 'tab.tunnels', icon: Wifi },
      { id: 'proxy-inspector', labelKey: 'tab.proxyInspector', icon: Search },
    ]
  },
  {
    titleKey: 'nav.orchestration',
    items: [
      { id: 'chains', labelKey: 'tab.chains', icon: Link2 },
      { id: 'graph', labelKey: 'tab.graph', icon: GraphIcon },
      { id: 'state-machine-editor', labelKey: 'tab.stateMachines', icon: GitBranch },
      { id: 'scenario-studio', labelKey: 'tab.scenarioStudio', icon: Film },
      { id: 'orchestration-builder', labelKey: 'tab.orchestrationBuilder', icon: GitBranch },
      { id: 'orchestration-execution', labelKey: 'tab.orchestrationExecution', icon: PlayCircle },
    ]
  },
  {
    titleKey: 'nav.observability',
    items: [
      { id: 'observability', labelKey: 'tab.observability', icon: Eye },
      { id: 'world-state', labelKey: 'tab.worldState', icon: Layers },
      { id: 'performance', labelKey: 'tab.performance', icon: Activity },
      { id: 'status', labelKey: 'tab.systemStatus', icon: Globe },
      { id: 'incidents', labelKey: 'tab.incidents', icon: AlertTriangle },
      { id: 'logs', labelKey: 'tab.logs', icon: FileText },
      { id: 'traces', labelKey: 'tab.traces', icon: Network },
      { id: 'metrics', labelKey: 'tab.metrics', icon: Activity },
      { id: 'analytics', labelKey: 'tab.analytics', icon: BarChart3 },
      { id: 'pillar-analytics', labelKey: 'tab.pillarAnalytics', icon: Layout },
      { id: 'fitness-functions', labelKey: 'tab.fitnessFunctions', icon: HeartPulse },
      { id: 'verification', labelKey: 'tab.verification', icon: CheckCircle2 },
      { id: 'contract-diff', labelKey: 'tab.contractDiff', icon: GitCompare },
    ]
  },
  {
    titleKey: 'nav.testing',
    items: [
      { id: 'testing', labelKey: 'tab.testing', icon: TestTube },
      { id: 'test-generator', labelKey: 'tab.testGenerator', icon: Code2 },
      { id: 'test-execution', labelKey: 'tab.testExecution', icon: PlayCircle },
      { id: 'integration-test-builder', labelKey: 'tab.integrationTests', icon: Layers },
      { id: 'time-travel', labelKey: 'tab.timeTravel', icon: History },
    ]
  },
  {
    titleKey: 'nav.chaosResilience',
    items: [
      { id: 'chaos', labelKey: 'tab.chaosEngineering', icon: Zap },
      { id: 'resilience', labelKey: 'tab.resilience', icon: Shield },
      { id: 'recorder', labelKey: 'tab.recorder', icon: Radio },
      { id: 'behavioral-cloning', labelKey: 'tab.behavioralCloning', icon: Copy },
    ]
  },
  {
    titleKey: 'nav.importTemplates',
    items: [
      { id: 'import', labelKey: 'tab.import', icon: Import },
      { id: 'template-marketplace', labelKey: 'tab.templateMarketplace', icon: Store },
    ]
  },
  {
    titleKey: 'nav.aiIntelligence',
    items: [
      { id: 'ai-studio', labelKey: 'tab.aiStudio', icon: Brain },
      { id: 'mockai', labelKey: 'tab.mockai', icon: Brain },
      { id: 'mockai-openapi-generator', labelKey: 'tab.mockaiOpenApiGenerator', icon: Code2 },
      { id: 'mockai-rules', labelKey: 'tab.mockaiRules', icon: BarChart3 },
      { id: 'voice', labelKey: 'tab.voiceLlm', icon: Mic },
    ]
  },
  {
    titleKey: 'nav.community',
    items: [
      { id: 'showcase', labelKey: 'tab.showcase', icon: Star },
      { id: 'learning-hub', labelKey: 'tab.learningHub', icon: BookOpen },
    ]
  },
  {
    titleKey: 'nav.plugins',
    items: [
      { id: 'plugins', labelKey: 'tab.plugins', icon: Puzzle },
      { id: 'plugin-registry', labelKey: 'tab.pluginRegistry', icon: Package },
    ]
  },
  {
    titleKey: 'nav.configuration',
    items: [
      { id: 'config', labelKey: 'tab.config', icon: Settings },
      { id: 'organization', labelKey: 'tab.organization', icon: Users },
      { id: 'billing', labelKey: 'tab.billing', icon: CreditCard },
      { id: 'api-tokens', labelKey: 'tab.apiTokens', icon: Key },
      { id: 'byok', labelKey: 'tab.byok', icon: LockIcon },
      { id: 'usage', labelKey: 'tab.usage', icon: LineChart },
      { id: 'user-management', labelKey: 'tab.userManagement', icon: Users },
    ]
  }
];

// Flattened items for title lookup
const allNavItems = navSections.flatMap(section => section.items);

export function AppShell({ children, activeTab, onTabChange, onRefresh }: AppShellProps) {
  const { t, locale, supportedLocales, setLocale } = useI18n();
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
        <a {...createSkipLink('main-navigation', t('a11y.skipNavigation'))} />
        <a {...createSkipLink('main-content', t('a11y.skipMain'))} />
        <a {...createSkipLink('global-search-input', t('a11y.skipSearch'))} />
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
                <span className="text-xl font-bold text-gray-900 dark:text-gray-100">{t('app.brand')}</span>
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
            <nav className="p-6 space-y-6 overflow-y-auto h-[calc(100%-88px)]">
              {navSections.map((section, sectionIndex) => (
                <div key={section.titleKey} className="space-y-2">
                  <h3 className="px-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {t(section.titleKey)}
                  </h3>
                  <div className="space-y-1">
                    {section.items.map((item, itemIndex) => {
                      const Icon = item.icon;
                      return (
                        <Button
                          key={item.id}
                          variant={activeTab === item.id ? 'default' : 'ghost'}
                          className={cn(
                            'w-full justify-start gap-4 h-10 text-sm nav-item-hover focus-ring spring-hover',
                            'animate-slide-in-up',
                            activeTab === item.id
                              ? 'bg-brand-500 text-white shadow-md hover:bg-brand-600'
                              : 'text-foreground/80 dark:text-gray-400 hover:text-foreground dark:hover:text-gray-100 hover:bg-muted/50'
                          )}
                          style={{ animationDelay: `${(sectionIndex * 5 + itemIndex) * 20}ms` }}
                          onClick={() => {
                            onTabChange(item.id);
                            setSidebarOpen(false);
                          }}
                        >
                          <Icon className="h-4 w-4" />
                          {t(item.labelKey)}
                        </Button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </nav>
          </aside>
        </div>
      )}

      <div className="flex">
        {/* Desktop Sidebar - Always visible on md and larger screens */}
        <aside className="hidden md:flex md:w-64 md:flex-col md:fixed md:inset-y-0 md:z-50">
          <div className="flex flex-col flex-grow bg-bg-primary border-r border-border">
            <div className="flex items-center gap-3 px-6 py-4 border-b border-border flex-shrink-0">
              <Logo variant="icon" size="md" />
              <span className="font-semibold text-gray-900 dark:text-gray-100">{t('app.brand')}</span>
            </div>
            <nav id="main-navigation" className="flex-1 px-4 py-6 space-y-6 overflow-y-auto" role="navigation" aria-label={t('a11y.mainNavigation')}>
              {navSections.map((section) => (
                <div key={section.titleKey} className="space-y-2">
                  <h3 className="px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {t(section.titleKey)}
                  </h3>
                  <div className="space-y-1">
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      return (
                        <Button
                          key={item.id}
                          variant={activeTab === item.id ? 'default' : 'ghost'}
                          className={cn(
                            'w-full justify-start gap-3 h-9 transition-all duration-200 nav-item-hover focus-ring spring-hover',
                            activeTab === item.id
                              ? 'bg-brand-600 text-white shadow-lg ring-1 ring-brand-200/60 dark:ring-brand-600/70 hover:bg-brand-700'
                              : 'text-foreground/80 dark:text-gray-200 hover:text-foreground dark:hover:text-white hover:bg-muted/50 dark:hover:bg-white/5'
                          )}
                          onClick={() => onTabChange(item.id)}
                        >
                          <Icon className="h-4 w-4" />
                          {t(item.labelKey)}
                        </Button>
                      );
                    })}
                  </div>
                </div>
              ))}
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
                <span className="text-sm text-gray-600 dark:text-gray-400">{t('app.home')}</span>
                <span className="text-gray-600 dark:text-gray-400">/</span>
                <span className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate capitalize">
                  {t(allNavItems.find(n => n.id === activeTab)?.labelKey ?? '', activeTab)}
                </span>
              </div>
              <div className="flex flex-1" />
              <div className="hidden sm:flex w-72 relative items-center">
                <Input
                  placeholder={t('app.searchPlaceholder')}
                  id="global-search-input"
                  value={globalQuery}
                  onChange={(e) => {
                    const q = e.target.value;
                    setGlobalQuery(q);
                    // lightweight sync across stores
                    setLogFilter({ path_pattern: q || undefined });
                    setGlobalSearch(q || undefined);
                  }}
                  onKeyDown={(e) => {
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
                <GlobalConnectionStatus className="hidden sm:flex" />
                {supportedLocales.length > 1 && (
                  <select
                    value={locale}
                    onChange={(e) => setLocale(e.target.value as Locale)}
                    className="hidden sm:block h-9 rounded-md border border-border bg-bg-primary px-2 text-xs"
                    aria-label="Language"
                  >
                    {supportedLocales.map((supportedLocale) => (
                      <option key={supportedLocale} value={supportedLocale}>
                        {supportedLocale.toUpperCase()}
                      </option>
                    ))}
                  </select>
                )}
                <SimpleThemeToggle />
                <Button variant="outline" size="sm" onClick={onRefresh} className="flex items-center gap-2">
                  <RefreshCw className="h-4 w-4" />
                  <span className="hidden sm:inline">{t('app.refresh')}</span>
                </Button>
                <UserProfile />
              </div>
            </div>
          </header>

          <main id="main-content" className="flex-1" role="main" aria-label={t('a11y.mainContent')}>
            <div className="w-full max-w-[1400px] mx-auto px-6 py-6">{children}</div>
          </main>
        </div>
      </div>
    </div>
  );
}
