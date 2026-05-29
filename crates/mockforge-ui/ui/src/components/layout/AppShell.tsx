import React, { useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { cn } from '../../utils/cn';
import { Button } from '../ui/button';
import { SimpleThemeToggle } from '../ui/ThemeToggle';
import { UserProfile } from '../auth/UserProfile';
import { HelpSupport } from '../auth/HelpSupport';
import { Logo } from '../ui/Logo';
import { Input } from '../ui/input';
import { useLogStore } from '../../stores/useLogStore';
import { useServiceStore } from '../../stores/useServiceStore';
import { useHelpStore } from '../../stores/useHelpStore';
import { usePreferencesStore } from '../../stores/usePreferencesStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
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
  Mail,
  Radio as RadioIcon,
  Network as NetworkIcon,
  MessageCircle as MessageCircleIcon,
  LifeBuoy,
  Bell,
  Camera,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { GlobalConnectionStatus } from './ConnectionStatus';
import { isCloudMode as detectCloudMode } from '../../utils/cloudMode';

interface AppShellProps {
  children: React.ReactNode;
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
      { id: 'cloud-snapshots', labelKey: 'tab.cloudSnapshots', icon: Camera },
      { id: 'tunnels', labelKey: 'tab.tunnels', icon: Wifi },
      { id: 'proxy-inspector', labelKey: 'tab.proxyInspector', icon: Search },
    ]
  },
  {
    titleKey: 'nav.protocolBrokers',
    items: [
      { id: 'smtp-mailbox', labelKey: 'tab.smtpMailbox', icon: Mail },
      { id: 'mqtt-broker', labelKey: 'tab.mqttBroker', icon: RadioIcon },
      { id: 'kafka-broker', labelKey: 'tab.kafkaBroker', icon: Database },
      { id: 'amqp-broker', labelKey: 'tab.amqpBroker', icon: NetworkIcon },
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
      { id: 'cloud-flows', labelKey: 'tab.cloudFlows', icon: GitBranch },
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
      { id: 'cloud-incidents', labelKey: 'tab.cloudIncidents', icon: Bell },
      { id: 'logs', labelKey: 'tab.logs', icon: FileText },
      { id: 'traces', labelKey: 'tab.traces', icon: Network },
      { id: 'cloud-traces', labelKey: 'tab.cloudTraces', icon: Network },
      { id: 'metrics', labelKey: 'tab.metrics', icon: Activity },
      { id: 'analytics', labelKey: 'tab.analytics', icon: BarChart3 },
      { id: 'pillar-analytics', labelKey: 'tab.pillarAnalytics', icon: Layout },
      { id: 'fitness-functions', labelKey: 'tab.fitnessFunctions', icon: HeartPulse },
      { id: 'verification', labelKey: 'tab.verification', icon: CheckCircle2 },
      { id: 'contract-diff', labelKey: 'tab.contractDiff', icon: GitCompare },
      { id: 'cloud-contract', labelKey: 'tab.cloudContract', icon: GitCompare },
    ]
  },
  {
    titleKey: 'nav.testing',
    items: [
      { id: 'testing', labelKey: 'tab.testing', icon: TestTube },
      { id: 'test-generator', labelKey: 'tab.testGenerator', icon: Code2 },
      { id: 'test-execution', labelKey: 'tab.testExecution', icon: PlayCircle },
      { id: 'cloud-test-runs', labelKey: 'tab.cloudTestRuns', icon: PlayCircle },
      { id: 'integration-test-builder', labelKey: 'tab.integrationTests', icon: Layers },
      { id: 'conformance', labelKey: 'tab.conformance', icon: Shield },
      { id: 'time-travel', labelKey: 'tab.timeTravel', icon: History },
    ]
  },
  {
    titleKey: 'nav.chaosResilience',
    items: [
      { id: 'chaos', labelKey: 'tab.chaosEngineering', icon: Zap },
      { id: 'cloud-chaos', labelKey: 'tab.cloudChaos', icon: Zap },
      { id: 'resilience', labelKey: 'tab.resilience', icon: Shield },
      { id: 'recorder', labelKey: 'tab.recorder', icon: Radio },
      { id: 'cloud-recorder', labelKey: 'tab.cloudRecorder', icon: Radio },
      { id: 'behavioral-cloning', labelKey: 'tab.behavioralCloning', icon: Copy },
      { id: 'cloud-behavioral-cloning', labelKey: 'tab.cloudBehavioralCloning', icon: Copy },
    ]
  },
  {
    titleKey: 'nav.importTemplates',
    items: [
      { id: 'import', labelKey: 'tab.import', icon: Import },
      { id: 'template-marketplace', labelKey: 'tab.templateMarketplace', icon: Store },
      { id: 'scenario-marketplace', labelKey: 'tab.scenarioMarketplace', icon: Store },
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
      { id: 'cloud-showcase-admin', labelKey: 'tab.cloudShowcaseAdmin', icon: Star },
      { id: 'learning-hub', labelKey: 'tab.learningHub', icon: BookOpen },
    ]
  },
  {
    titleKey: 'nav.plugins',
    items: [
      { id: 'plugins', labelKey: 'tab.plugins', icon: Puzzle },
      { id: 'cloud-plugins', labelKey: 'tab.cloudPlugins', icon: Puzzle },
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
      { id: 'publisher-keys', labelKey: 'tab.publisherKeys', icon: Key },
      { id: 'byok', labelKey: 'tab.byok', icon: LockIcon },
      { id: 'usage', labelKey: 'tab.usage', icon: LineChart },
      { id: 'notification-channels', labelKey: 'tab.notificationChannels', icon: Bell },
      // user-management retired (#15) — surface lives inside the
      // Organization page's Members / Roles / Activity tabs now.
    ]
  },
  {
    titleKey: 'nav.help',
    items: [
      { id: 'faq', labelKey: 'tab.faq', icon: MessageCircleIcon },
      { id: 'support', labelKey: 'tab.support', icon: LifeBuoy },
    ]
  }
];

// Cloud mode: only show functional nav items when running on the cloud app
const isCloudMode = detectCloudMode();

const cloudNavItemIds = new Set([
  'dashboard',
  'workspaces',
  'federation',
  'services',
  'fixtures',
  'hosted-mocks',
  'template-marketplace',
  'scenario-marketplace',
  'plugin-registry',
  'pillar-analytics',
  'status',
  // AI Studio chat + the rest of the MockAI suite are wired end-to-end
  // through aiStudioApi: chat / generate-openapi / explain-rule plus the
  // post-#353 cloud routes for rule explanations, learn, generate-from-
  // traffic, and the three voice handlers (process / transpile-hook /
  // create-workspace-scenario).
  'ai-studio',
  'mockai',
  'mockai-rules',
  'mockai-openapi-generator',
  'voice',
  // Import dispatches to /api/v1/import/preview + /api/v1/workspaces/{id}/import
  // when isCloudMode(); requires an active workspace selection.
  'import',
  // Tunnels page detects cloud mode and dispatches CRUD + DNS verify
  // through cloudTunnelsApi against /api/v1/organizations/{org_id}/tunnels.
  'tunnels',
  // Cloud snapshots — synchronous capture / diff / restore for the
  // active workspace via cloudSnapshotsApi.
  'cloud-snapshots',
  // Resilience dashboard (#468 cloud scaffold) — circuit-breaker /
  // bulkhead state via cloudResilienceApi. Phase 1 returns
  // `runtime_state: 'pending'` because the hosted-mock runtime hasn't
  // wired the middleware yet; the page shows an explicit pending
  // banner so users don't confuse "no breakers configured" with the
  // empty scaffold response.
  'resilience',
  // Virtual Backends (#461) — entities tab dispatches to
  // cloudConsistencyApi against /api/v1/workspaces/{id}/consistency/*.
  // Lifecycle preset library is shared with local mode (static array
  // server-side); cloud-mode snapshots tab links to the cloud-snapshots
  // page instead of trying to host two snapshot UIs.
  'virtual-backends',
  // Cloud incidents — org-wide dashboard wired through cloudIncidentsApi
  // (different feature from drift IncidentDashboard).
  'cloud-incidents',
  // Cloud test runs — org-wide history with SSE event tailing via
  // cloudTestRunsApi.streamRunEvents.
  'cloud-test-runs',
  // Testing page — cloud mode dispatches through cloudSmokeApi against
  // /api/v1/hosted-mocks/{id}/smoke-runs and tails route_pass/_fail/_skipped
  // events via cloudTestRunsApi.streamRunEvents (#392).
  'testing',
  // Integration Test Builder persists as test_suite kind='integration'
  // and runs through the IntegrationExecutor in mockforge-test-runner
  // (#356).
  'integration-test-builder',
  // Cloud traces — cross-deployment OTLP search via cloudObservabilityApi.
  'cloud-traces',
  // Cloud chaos campaigns — workspace-scoped via cloudChaosApi.
  'cloud-chaos',
  // Cloud flows — versioned scenario / state-machine / orchestration /
  // chain definitions via cloudFlowsApi (covers #9 + #14 collab).
  'cloud-flows',
  // Chains share the flows resource (kind='chain') and are executed by
  // the ChainExecutor in mockforge-test-runner (#354).
  'chains',
  // Workspace dependency graph (#460) — services + flows as nodes,
  // clustered by the active workspace. Phase 1 returns no edges; SSE
  // updates are local-only for now (cloud falls back to 30s polling).
  'graph',
  // Observability dashboard (#465) — Phase 1 lists the org's saved
  // queries and runs them on-demand via cloudObservabilityApi.execute
  // SavedQuery. Live dashboard tiles + event stream are a follow-up.
  'observability',
  // Scenario Studio uses cloudFlowsApi with kind='scenario'. Each flow
  // version stores the full {flow_type, steps, connections, tags}
  // payload as the FlowVersion config; runs queue through test_runs.
  'scenario-studio',
  // State Machine Editor uses cloudFlowsApi with kind='state_machine'.
  // The page exposes a workspace-scoped picker; the selected flow's
  // current FlowVersion.config carries {state_machine, visual_layout}.
  'state-machine-editor',
  // Orchestration Builder uses cloudFlowsApi with kind='orchestration'.
  // The page persists the full Orchestration object (name, description,
  // variables, hooks, steps, conditionalSteps, assertions,
  // enableReporting) as the FlowVersion config.
  'orchestration-builder',
  // Orchestration Execution viewer streams test_run_events via
  // cloudTestRunsApi.streamRunEvents for the cloudFlowsApi.triggerRun
  // result. step_start / step_pass / step_fail / step_skip / done get
  // mapped onto the existing ExecutionStep visualization.
  'orchestration-execution',
  // Fitness Functions: read-only via cloudContractApi.listFitnessFunctions.
  // Cloud rows have a generic {kind, config} blob — we adapt them into
  // the local typed shape and hide create/edit/delete (no write paths
  // on the registry yet).
  'fitness-functions',
  // Cloud contract diff + verification via cloudContractApi.
  'cloud-contract',
  // Cloud-mode request verification (#390): WireMock-style assertions
  // against the workspace's runtime_captures table, dispatched through
  // cloudVerificationApi against /api/v1/workspaces/{id}/request-log/*.
  'verification',
  // Cloud-mode conformance (#391): ad-hoc OpenAPI conformance runs
  // dispatched as transient kind='conformance' test_suites through
  // cloudTestRunsApi. The runner side uses NativeConformanceExecutor
  // and streams `started` / `check_completed` / `finished` events
  // through test_run_events.
  'conformance',
  // Cloud recorder + behavioral cloning via cloudRecorderApi.
  'cloud-recorder',
  // Cloud behavioral cloning (#393) — clone-model-centric view with live
  // SSE training/replay streams via cloudTestRunsApi.streamRunEvents.
  'cloud-behavioral-cloning',
  // Showcase admin authoring via cloudShowcaseApi.adminList / adminCreate /
  // adminUpdate / adminDelete.
  'cloud-showcase-admin',
  // Cloud plugins (Phase 3) — read-only attachment listing today;
  // attach/permission/detach controls land in subsequent sub-PRs once
  // PR #395's control-plane API merges. Listed here so the sidebar
  // shows it as active in cloud mode.
  'cloud-plugins',
  // Public showcase + learning hub adapt /api/v1/showcase/* and
  // /api/v1/learning/* into the legacy ShowcaseProject / LearningResource
  // shapes via cloudCommunityApi (services/api/cloudCommunity.ts).
  'showcase',
  'learning-hub',
  // Workspace request logs (#462) — read from `runtime_captures` rows
  // tagged with this workspace via cloudLogsApi. Hosted-mock captures
  // without workspace_id are invisible until the shipper backfill lands;
  // cloud-shipped captures (--cloud-ship) work today.
  'logs',
  // World State (#464 Phase 2) — per-deployment graph + snapshot + layers
  // + slice query via cloudWorldStateApi against /api/v1/hosted-mocks/
  // {deployment_id}/world-state/*. The local `/stream` WebSocket isn't
  // proxied yet (Phase 2 follow-up); cloud mode polls every 5s, which
  // matches the local TanStack Query refetchInterval.
  'world-state',
  // Test Generator (#469 Phase 2) — async LLM jobs over runtime_captures
  // via cloudTestGeneratorApi against /api/v1/workspaces/{id}/test-generation
  // /jobs. Phase 1 shipped the data plane (table + 4 CRUD endpoints + TS
  // client); Phase 2 ships the page branch. The background BYOK LLM
  // worker is Phase 3 — jobs created here sit in 'queued' until that
  // lands. The page surfaces this honestly via an inline banner.
  'test-generator',
  // Time Travel (#466 Phase 2) — per-deployment virtual-clock control via
  // cloudTimeTravelApi against /api/v1/hosted-mocks/{deployment_id}
  // /time-travel/* (registry proxies over Fly 6PN to port 3000). Only
  // the 7 clock-control endpoints are wired in cloud mode (status /
  // enable / disable / advance / set / scale / reset); cron jobs and
  // mutation rules stay local-only because they manage scenario state,
  // not a hosted mock's single-process clock.
  'time-travel',
  // Notification channels (cloud-only) — incident dispatch destinations
  // wired through cloudNotificationsApi.
  'notification-channels',
  'config',
  'organization',
  'billing',
  'api-tokens',
  'publisher-keys',
  'byok',
  'usage',
  'faq',
  'support',
]);

// Items in this set are HIDDEN entirely in cloud mode because a cloud-*
// sibling page already supersedes them. Showing both creates sidebar
// noise without adding value (e.g. ChaosPage vs CloudChaosPage).
const cloudHiddenNavItemIds = new Set([
  // Each entry has a cloud sibling that serves the same purpose:
  'chaos',                  // → cloud-chaos
  'recorder',               // → cloud-recorder
  'behavioral-cloning',     // → cloud-behavioral-cloning
  'incidents',              // → cloud-incidents
  'traces',                 // → cloud-traces
  'contract-diff',          // → cloud-contract
  'plugins',                // → plugin-registry (cloud-side plugin discovery)
  'test-execution',         // → cloud-test-runs (TestExecutionDashboard is mock-data only)
  'analytics',              // → pillar-analytics (request-traffic analytics is local-only)
  // The next two (#463, #467) are redundant with pages that are already
  // cloud-enabled — keeping both visible just adds sidebar noise.
  'metrics',                // → pillar-analytics (request rate / latency / errors live there)
  'performance',            // → cloud-test-runs (k6 / load runs already covered)
  // ApiExplorerPage is already cloud-aware (takes a `deployment` prop and
  // fetches the OpenAPI spec from the runtime). It is reached by clicking
  // "Open" on a deployment in HostedMocksPage, not standalone from the
  // sidebar — keeping it visible suggested a global explorer that does not
  // exist in cloud mode.
  'api-explorer',           // → reached via HostedMocksPage "Open" action
]);

// In cloud mode, items outside the allowlist are shown as disabled "Local only"
// entries so users can discover the full product surface and understand what
// requires a local MockForge instance. In self-hosted mode every item is active.
//
// Cloud-mode label overrides: when the local Analytics tab is hidden in cloud,
// pillar-analytics becomes the de-facto analytics destination, so it surfaces
// under the plain "Analytics" label instead of "Pillar Analytics" (#394).
const cloudLabelOverrides: Record<string, string> = {
  'pillar-analytics': 'tab.analytics',
};
const effectiveNavSections = navSections
  .map(section => ({
    ...section,
    items: section.items
      .filter(item => !(isCloudMode && cloudHiddenNavItemIds.has(item.id)))
      .map(item => ({
        ...item,
        labelKey: (isCloudMode && cloudLabelOverrides[item.id]) || item.labelKey,
        localOnly: isCloudMode && !cloudNavItemIds.has(item.id),
      })),
  }))
  .filter(section => section.items.length > 0);

// Flattened items for title lookup (includes non-sidebar pages for breadcrumb
// resolution). Apply the same cloud-mode label overrides so the breadcrumb
// matches the nav label users clicked on.
const allNavItems = [
  ...navSections.flatMap(section => section.items),
  { id: 'api-explorer', labelKey: 'tab.apiExplorer', icon: Code2 },
].map(item => ({
  ...item,
  labelKey: (isCloudMode && cloudLabelOverrides[item.id]) || item.labelKey,
}));

export function AppShell({ children, onRefresh }: AppShellProps) {
  const { t, locale, supportedLocales, setLocale } = useI18n();
  const location = useLocation();
  const navigate = useNavigate();
  const activeTab = location.pathname.replace(/^\//, '') || 'dashboard';
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const { setFilter: setLogFilter } = useLogStore();
  const { setGlobalSearch } = useServiceStore();
  const [globalQuery, setGlobalQuery] = useState('');
  const [isMac, setIsMac] = useState(false);

  const helpOpen = useHelpStore(state => state.isOpen);
  const openHelp = useHelpStore(state => state.open);
  const setHelpOpen = useHelpStore(state => state.setOpen);
  const workspaces = useWorkspaceStore(state => state.workspaces);
  const activeWorkspace = useWorkspaceStore(state => state.activeWorkspace);
  const setActiveWorkspaceById = useWorkspaceStore(state => state.setActiveWorkspaceById);
  const keyboardShortcutsEnabled = usePreferencesStore(
    state => state.preferences.ui.keyboardShortcuts,
  );
  const sidebarCollapsed = usePreferencesStore(state => state.preferences.ui.sidebarCollapsed);
  const updateUI = usePreferencesStore(state => state.updateUI);
  const defaultSearchScope = usePreferencesStore(
    state => state.preferences.search.defaultScope,
  );
  type SearchScope = 'all' | 'current' | 'logs' | 'services';
  const [searchScope, setSearchScope] = useState<SearchScope>(
    (defaultSearchScope as SearchScope) ?? 'all',
  );
  // Keep local scope state aligned with the default when the preference changes.
  React.useEffect(() => {
    setSearchScope((defaultSearchScope as SearchScope) ?? 'all');
  }, [defaultSearchScope]);

  const dispatchSearch = (q: string | undefined) => {
    const wantLogs = searchScope === 'all' || searchScope === 'logs' || searchScope === 'current';
    const wantServices =
      searchScope === 'all' || searchScope === 'services' || searchScope === 'current';
    setLogFilter({ path_pattern: wantLogs ? q : undefined });
    setGlobalSearch(wantServices ? q : undefined);
  };

  // Setup keyboard shortcuts (user-disablable via preferences.ui.keyboardShortcuts)
  useAppShortcuts({
    onSearch: () => {
      const searchInput = document.getElementById('global-search-input') as HTMLInputElement;
      if (searchInput) {
        searchInput.focus();
        searchInput.select();
      }
    },
    onHelp: () => openHelp(),
    enabled: keyboardShortcutsEnabled,
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
              {effectiveNavSections.map((section, sectionIndex) => (
                <div key={section.titleKey} className="space-y-2">
                  <h3 className="px-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {t(section.titleKey)}
                  </h3>
                  <div className="space-y-1">
                    {section.items.map((item, itemIndex) => {
                      const Icon = item.icon;
                      const isLocalOnly = item.localOnly;
                      return (
                        <Button
                          key={item.id}
                          variant={activeTab === item.id ? 'default' : 'ghost'}
                          disabled={isLocalOnly}
                          title={isLocalOnly ? t('nav.localOnly.tooltip') : undefined}
                          className={cn(
                            'w-full justify-start gap-4 h-10 text-sm nav-item-hover focus-ring spring-hover',
                            'animate-slide-in-up',
                            isLocalOnly
                              ? 'text-muted-foreground/60 cursor-not-allowed opacity-70'
                              : activeTab === item.id
                              ? 'bg-brand-500 text-white shadow-md hover:bg-brand-600'
                              : 'text-foreground/80 dark:text-gray-400 hover:text-foreground dark:hover:text-gray-100 hover:bg-muted/50'
                          )}
                          style={{ animationDelay: `${(sectionIndex * 5 + itemIndex) * 20}ms` }}
                          onClick={() => {
                            if (isLocalOnly) return;
                            navigate('/' + item.id);
                            setSidebarOpen(false);
                          }}
                        >
                          <Icon className="h-4 w-4" />
                          <span className="flex-1 text-left">{t(item.labelKey)}</span>
                          {isLocalOnly && (
                            <span className="flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                              <LockIcon className="h-3 w-3" />
                              {t('nav.localOnly.badge')}
                            </span>
                          )}
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
        {/* Desktop Sidebar - Always visible on md and larger screens; width
            driven by preferences.ui.sidebarCollapsed. */}
        <aside
          className={cn(
            'hidden md:flex md:flex-col md:fixed md:inset-y-0 md:z-50 overflow-hidden transition-[width] duration-200',
            sidebarCollapsed ? 'md:w-16' : 'md:w-64',
          )}
        >
          <div className="flex flex-col flex-grow overflow-hidden bg-bg-primary border-r border-border">
            <div className="flex items-center gap-3 px-4 py-4 border-b border-border flex-shrink-0">
              <Logo variant="icon" size="md" />
              {!sidebarCollapsed && (
                <span className="font-semibold text-gray-900 dark:text-gray-100">{t('app.brand')}</span>
              )}
              <Button
                variant="ghost"
                size="sm"
                className="ml-auto h-7 w-7 p-0"
                onClick={() => updateUI({ sidebarCollapsed: !sidebarCollapsed })}
                aria-label={sidebarCollapsed ? t('a11y.expandSidebar') : t('a11y.collapseSidebar')}
                title={sidebarCollapsed ? t('a11y.expandSidebar') : t('a11y.collapseSidebar')}
              >
                {sidebarCollapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
              </Button>
            </div>
            <nav id="main-navigation" className="flex-1 px-2 py-6 space-y-6 overflow-y-auto" role="navigation" aria-label={t('a11y.mainNavigation')}>
              {effectiveNavSections.map((section) => (
                <div key={section.titleKey} className="space-y-2">
                  {!sidebarCollapsed && (
                    <h3 className="px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                      {t(section.titleKey)}
                    </h3>
                  )}
                  <div className="space-y-1">
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const isLocalOnly = item.localOnly;
                      const label = t(item.labelKey);
                      return (
                        <Button
                          key={item.id}
                          variant={activeTab === item.id ? 'default' : 'ghost'}
                          disabled={isLocalOnly}
                          title={
                            isLocalOnly
                              ? t('nav.localOnly.tooltip')
                              : sidebarCollapsed
                              ? label
                              : undefined
                          }
                          aria-label={sidebarCollapsed ? label : undefined}
                          className={cn(
                            'w-full h-9 transition-all duration-200 nav-item-hover focus-ring spring-hover',
                            sidebarCollapsed ? 'justify-center px-0' : 'justify-start gap-3',
                            isLocalOnly
                              ? 'text-muted-foreground/60 cursor-not-allowed opacity-70'
                              : activeTab === item.id
                              ? 'bg-brand-600 text-white shadow-lg ring-1 ring-brand-200/60 dark:ring-brand-600/70 hover:bg-brand-700'
                              : 'text-foreground/80 dark:text-gray-200 hover:text-foreground dark:hover:text-white hover:bg-muted/50 dark:hover:bg-white/5'
                          )}
                          onClick={() => {
                            if (isLocalOnly) return;
                            navigate('/' + item.id);
                          }}
                        >
                          <Icon className="h-4 w-4" />
                          {!sidebarCollapsed && (
                            <>
                              <span className="flex-1 text-left">{label}</span>
                              {isLocalOnly && (
                                <span className="flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                                  <LockIcon className="h-3 w-3" />
                                  {t('nav.localOnly.badge')}
                                </span>
                              )}
                            </>
                          )}
                        </Button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </nav>
          </div>
        </aside>

        <div className={cn('flex flex-col flex-1 min-h-screen', sidebarCollapsed ? 'md:pl-16' : 'md:pl-64')}>
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
              <div className="hidden sm:flex w-80 relative items-center gap-1">
                <select
                  value={searchScope}
                  onChange={(e) => {
                    const next = e.target.value as SearchScope;
                    setSearchScope(next);
                    if (globalQuery) {
                      // Re-dispatch with the new scope so stale filters clear.
                      const wantLogs = next === 'all' || next === 'logs' || next === 'current';
                      const wantServices = next === 'all' || next === 'services' || next === 'current';
                      setLogFilter({ path_pattern: wantLogs ? globalQuery : undefined });
                      setGlobalSearch(wantServices ? globalQuery : undefined);
                    }
                  }}
                  aria-label={t('a11y.searchScope')}
                  className="h-9 rounded-md border border-border bg-bg-primary px-1.5 text-xs text-foreground"
                >
                  <option value="all">{t('search.scope.all')}</option>
                  <option value="current">{t('search.scope.current')}</option>
                  <option value="logs">{t('search.scope.logs')}</option>
                  <option value="services">{t('search.scope.services')}</option>
                </select>
                <div className="relative flex-1">
                  <Input
                    placeholder={t('app.searchPlaceholder')}
                    id="global-search-input"
                    value={globalQuery}
                    onChange={(e) => {
                      const q = e.target.value;
                      setGlobalQuery(q);
                      dispatchSearch(q || undefined);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Escape') {
                        setGlobalQuery('');
                        dispatchSearch(undefined);
                        (document.getElementById('global-search-input') as HTMLInputElement | null)?.blur();
                      }
                    }}
                  />
                  <span className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-[10px] text-gray-600 dark:text-gray-400 border border-border rounded px-1 py-0.5 bg-bg-primary">
                    {isMac ? '⌘K' : 'Ctrl K'}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-x-4 lg:gap-x-6">
                <GlobalConnectionStatus className="hidden sm:flex" />
                {workspaces.length > 0 && (
                  <select
                    value={activeWorkspace?.id ?? ''}
                    onChange={(e) => {
                      const id = e.target.value;
                      if (id) void setActiveWorkspaceById(id);
                    }}
                    className="hidden sm:block h-9 max-w-[200px] rounded-md border border-border bg-bg-primary px-2 text-xs text-foreground"
                    aria-label={t('workspace.selector.label')}
                    title={activeWorkspace?.name ?? t('workspace.selector.placeholder')}
                  >
                    {!activeWorkspace && (
                      <option value="" disabled>
                        {t('workspace.selector.placeholder')}
                      </option>
                    )}
                    {workspaces.map((w) => (
                      <option key={w.id} value={w.id}>
                        {w.name}
                      </option>
                    ))}
                  </select>
                )}
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

      {/* Shared Help & Support modal — opened by Shift+? or the avatar menu. */}
      <HelpSupport open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
