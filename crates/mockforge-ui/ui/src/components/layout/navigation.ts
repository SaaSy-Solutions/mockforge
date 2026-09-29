/**
 * Sidebar / command-palette navigation model.
 *
 * Single source of truth for which pages exist in the shell, how they are
 * grouped, and how they behave in cloud mode. AppShell, the command palette
 * and the breadcrumb all read from here so the three can never disagree.
 */
import type { LucideIcon } from 'lucide-react';
import {
  Activity,
  AlertTriangle,
  BarChart3,
  Bell,
  BookOpen,
  Brain,
  Camera,
  CheckCircle2,
  Cloud,
  Code2,
  Copy,
  CreditCard,
  Database,
  Eye,
  FileJson,
  FileText,
  Film,
  FolderOpen,
  GitBranch,
  GitCompare,
  Globe,
  HeartPulse,
  History,
  Import,
  Key,
  Layers,
  Layout,
  LayoutDashboard,
  LifeBuoy,
  LineChart,
  Link2,
  Lock,
  Mail,
  MessageCircle,
  Mic,
  Network,
  Package,
  PlayCircle,
  Puzzle,
  Radio,
  Search,
  Server,
  Settings,
  Share2,
  Shield,
  Sparkles,
  Star,
  Store,
  TestTube,
  Users,
  Waypoints,
  Wifi,
  Workflow,
  Zap,
} from 'lucide-react';
import { isCloudMode as detectCloudMode } from '../../utils/cloudMode';

export interface NavItem {
  id: string;
  labelKey: string;
  icon: LucideIcon;
}

export interface NavSection {
  /** Stable id, also used to persist the collapsed state. */
  id: string;
  /** Omitted for the untitled top group (Dashboard / Workspaces). */
  titleKey?: string;
  items: NavItem[];
}

export interface ResolvedNavItem extends NavItem {
  /** Present in the sidebar but not functional in this deployment. */
  localOnly: boolean;
  sectionId: string;
}

export interface ResolvedNavSection extends Omit<NavSection, 'items'> {
  items: ResolvedNavItem[];
}

/**
 * Grouped by what the user is trying to do, not by implementation. Local and
 * cloud variants of a feature sit next to each other; in cloud mode the local
 * one is hidden (see `cloudHiddenNavItemIds`) so users only ever see one.
 */
export const navSections: NavSection[] = [
  {
    id: 'overview',
    items: [
      { id: 'dashboard', labelKey: 'tab.dashboard', icon: LayoutDashboard },
      { id: 'workspaces', labelKey: 'tab.workspaces', icon: FolderOpen },
    ],
  },
  {
    id: 'mocks',
    titleKey: 'nav.mocks',
    items: [
      { id: 'services', labelKey: 'tab.services', icon: Server },
      { id: 'hosted-mocks', labelKey: 'tab.hostedMocks', icon: Cloud },
      { id: 'fixtures', labelKey: 'tab.fixtures', icon: FileJson },
      { id: 'virtual-backends', labelKey: 'tab.virtualBackends', icon: Database },
      { id: 'cloud-snapshots', labelKey: 'tab.cloudSnapshots', icon: Camera },
      { id: 'import', labelKey: 'tab.import', icon: Import },
      { id: 'federation', labelKey: 'tab.federation', icon: Share2 },
      { id: 'tunnels', labelKey: 'tab.tunnels', icon: Wifi },
      { id: 'proxy-inspector', labelKey: 'tab.proxyInspector', icon: Search },
    ],
  },
  {
    id: 'protocols',
    titleKey: 'nav.protocols',
    items: [
      { id: 'smtp-mailbox', labelKey: 'tab.smtpMailbox', icon: Mail },
      { id: 'mqtt-broker', labelKey: 'tab.mqttBroker', icon: Radio },
      { id: 'kafka-broker', labelKey: 'tab.kafkaBroker', icon: Database },
      { id: 'amqp-broker', labelKey: 'tab.amqpBroker', icon: Network },
    ],
  },
  {
    id: 'flows',
    titleKey: 'nav.flows',
    items: [
      { id: 'scenario-studio', labelKey: 'tab.scenarioStudio', icon: Film },
      { id: 'chains', labelKey: 'tab.chains', icon: Link2 },
      { id: 'state-machine-editor', labelKey: 'tab.stateMachines', icon: GitBranch },
      { id: 'orchestration-builder', labelKey: 'tab.orchestrationBuilder', icon: Workflow },
      { id: 'orchestration-execution', labelKey: 'tab.orchestrationExecution', icon: PlayCircle },
      { id: 'cloud-flows', labelKey: 'tab.cloudFlows', icon: Layers },
      { id: 'graph', labelKey: 'tab.graph', icon: Waypoints },
    ],
  },
  {
    id: 'observe',
    titleKey: 'nav.observe',
    items: [
      { id: 'observability', labelKey: 'tab.observability', icon: Eye },
      { id: 'logs', labelKey: 'tab.logs', icon: FileText },
      { id: 'traces', labelKey: 'tab.traces', icon: Network },
      { id: 'cloud-traces', labelKey: 'tab.cloudTraces', icon: Network },
      { id: 'metrics', labelKey: 'tab.metrics', icon: Activity },
      { id: 'analytics', labelKey: 'tab.analytics', icon: BarChart3 },
      { id: 'pillar-analytics', labelKey: 'tab.pillarAnalytics', icon: Layout },
      { id: 'performance', labelKey: 'tab.performance', icon: Activity },
      { id: 'world-state', labelKey: 'tab.worldState', icon: Globe },
      { id: 'incidents', labelKey: 'tab.incidents', icon: AlertTriangle },
      { id: 'cloud-incidents', labelKey: 'tab.cloudIncidents', icon: AlertTriangle },
      { id: 'status', labelKey: 'tab.systemStatus', icon: HeartPulse },
    ],
  },
  {
    id: 'test',
    titleKey: 'nav.test',
    items: [
      { id: 'testing', labelKey: 'tab.testing', icon: TestTube },
      { id: 'test-generator', labelKey: 'tab.testGenerator', icon: Code2 },
      { id: 'test-execution', labelKey: 'tab.testExecution', icon: PlayCircle },
      { id: 'cloud-test-runs', labelKey: 'tab.cloudTestRuns', icon: PlayCircle },
      { id: 'integration-test-builder', labelKey: 'tab.integrationTests', icon: Layers },
      { id: 'conformance', labelKey: 'tab.conformance', icon: Shield },
      { id: 'verification', labelKey: 'tab.verification', icon: CheckCircle2 },
      { id: 'contract-diff', labelKey: 'tab.contractDiff', icon: GitCompare },
      { id: 'cloud-contract', labelKey: 'tab.cloudContract', icon: GitCompare },
      { id: 'fitness-functions', labelKey: 'tab.fitnessFunctions', icon: HeartPulse },
      { id: 'time-travel', labelKey: 'tab.timeTravel', icon: History },
    ],
  },
  {
    id: 'resilience',
    titleKey: 'nav.resilience',
    items: [
      { id: 'chaos', labelKey: 'tab.chaosEngineering', icon: Zap },
      { id: 'cloud-chaos', labelKey: 'tab.cloudChaos', icon: Zap },
      { id: 'resilience', labelKey: 'tab.resilience', icon: Shield },
      { id: 'recorder', labelKey: 'tab.recorder', icon: Radio },
      { id: 'cloud-recorder', labelKey: 'tab.cloudRecorder', icon: Radio },
      { id: 'behavioral-cloning', labelKey: 'tab.behavioralCloning', icon: Copy },
      { id: 'cloud-behavioral-cloning', labelKey: 'tab.cloudBehavioralCloning', icon: Copy },
    ],
  },
  {
    id: 'ai',
    titleKey: 'nav.ai',
    items: [
      { id: 'ai-studio', labelKey: 'tab.aiStudio', icon: Sparkles },
      { id: 'mockai', labelKey: 'tab.mockai', icon: Brain },
      { id: 'mockai-openapi-generator', labelKey: 'tab.mockaiOpenApiGenerator', icon: Code2 },
      { id: 'mockai-rules', labelKey: 'tab.mockaiRules', icon: BarChart3 },
      { id: 'voice', labelKey: 'tab.voiceLlm', icon: Mic },
    ],
  },
  {
    id: 'ecosystem',
    titleKey: 'nav.ecosystem',
    items: [
      { id: 'template-marketplace', labelKey: 'tab.templateMarketplace', icon: Store },
      { id: 'scenario-marketplace', labelKey: 'tab.scenarioMarketplace', icon: Store },
      { id: 'plugins', labelKey: 'tab.plugins', icon: Puzzle },
      { id: 'cloud-plugins', labelKey: 'tab.cloudPlugins', icon: Puzzle },
      { id: 'plugin-registry', labelKey: 'tab.pluginRegistry', icon: Package },
      { id: 'showcase', labelKey: 'tab.showcase', icon: Star },
      { id: 'cloud-showcase-admin', labelKey: 'tab.cloudShowcaseAdmin', icon: Star },
      { id: 'learning-hub', labelKey: 'tab.learningHub', icon: BookOpen },
    ],
  },
  {
    id: 'settings',
    titleKey: 'nav.settings',
    items: [
      { id: 'config', labelKey: 'tab.config', icon: Settings },
      { id: 'organization', labelKey: 'tab.organization', icon: Users },
      { id: 'billing', labelKey: 'tab.billing', icon: CreditCard },
      { id: 'usage', labelKey: 'tab.usage', icon: LineChart },
      { id: 'api-tokens', labelKey: 'tab.apiTokens', icon: Key },
      { id: 'publisher-keys', labelKey: 'tab.publisherKeys', icon: Key },
      { id: 'byok', labelKey: 'tab.byok', icon: Lock },
      { id: 'notification-channels', labelKey: 'tab.notificationChannels', icon: Bell },
      // user-management retired (#15) — surface lives inside the
      // Organization page's Members / Roles / Activity tabs now.
    ],
  },
];

/** Rendered pinned to the bottom of the sidebar rather than as a section. */
export const helpNavItems: NavItem[] = [
  { id: 'faq', labelKey: 'tab.faq', icon: MessageCircle },
  { id: 'support', labelKey: 'tab.support', icon: LifeBuoy },
];

/** Pages reachable by URL but not listed in the sidebar (breadcrumb only). */
const unlistedNavItems: NavItem[] = [
  { id: 'api-explorer', labelKey: 'tab.apiExplorer', icon: Code2 },
];

// Cloud mode: only these ids are functional on the cloud app. Everything
// else stays discoverable in a collapsed "Local only" group.
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
  // through aiStudioApi (chat / generate-openapi / explain-rule, rule
  // explanations, learn, generate-from-traffic, voice handlers — #353).
  'ai-studio',
  'mockai',
  'mockai-rules',
  'mockai-openapi-generator',
  'voice',
  // Import → /api/v1/import/preview + /api/v1/workspaces/{id}/import.
  'import',
  // Tunnels → cloudTunnelsApi (/api/v1/organizations/{org_id}/tunnels).
  'tunnels',
  // Snapshot capture / diff / restore via cloudSnapshotsApi.
  'cloud-snapshots',
  // Resilience dashboard (#468) via cloudResilienceApi; shows an explicit
  // pending banner while the hosted runtime middleware is unwired.
  'resilience',
  // Virtual Backends (#461) via cloudConsistencyApi.
  'virtual-backends',
  // Org-wide incident dashboard via cloudIncidentsApi.
  'cloud-incidents',
  // Org-wide test-run history with SSE tailing via cloudTestRunsApi.
  'cloud-test-runs',
  // Smoke runs via cloudSmokeApi (#392).
  'testing',
  // test_suite kind='integration' via IntegrationExecutor (#356).
  'integration-test-builder',
  // Cross-deployment OTLP search via cloudObservabilityApi.
  'cloud-traces',
  // Workspace-scoped chaos campaigns via cloudChaosApi.
  'cloud-chaos',
  // Versioned flow definitions via cloudFlowsApi (#9, #14).
  'cloud-flows',
  // kind='chain' flows executed by ChainExecutor (#354).
  'chains',
  // Workspace dependency graph (#460); cloud polls every 30s.
  'graph',
  // Saved-query tiles via cloudObservabilityApi (#465).
  'observability',
  // cloudFlowsApi kind='scenario' / 'state_machine' / 'orchestration'.
  'scenario-studio',
  'state-machine-editor',
  'orchestration-builder',
  // Streams test_run_events for cloudFlowsApi.triggerRun results.
  'orchestration-execution',
  // Read-only via cloudContractApi.listFitnessFunctions.
  'fitness-functions',
  // Contract diff + verification via cloudContractApi.
  'cloud-contract',
  // Request verification against runtime_captures (#390).
  'verification',
  // Ad-hoc OpenAPI conformance runs via cloudTestRunsApi (#391).
  'conformance',
  // Recorder + behavioral cloning via cloudRecorderApi (#393).
  'cloud-recorder',
  'cloud-behavioral-cloning',
  // Showcase authoring via cloudShowcaseApi.admin*.
  'cloud-showcase-admin',
  // Read-only plugin attachment listing (Phase 3).
  'cloud-plugins',
  // Public showcase + learning hub via cloudCommunityApi.
  'showcase',
  'learning-hub',
  // Workspace request logs from runtime_captures (#462).
  'logs',
  // Per-deployment world state via cloudWorldStateApi (#464); polls 5s.
  'world-state',
  // Async LLM test-generation jobs (#469); worker lands in Phase 3.
  'test-generator',
  // Per-deployment virtual clock via cloudTimeTravelApi (#466).
  'time-travel',
  // Incident dispatch destinations via cloudNotificationsApi.
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

// HIDDEN entirely in cloud mode because a cloud-* sibling supersedes them.
const cloudHiddenNavItemIds = new Set([
  'chaos', // → cloud-chaos
  'recorder', // → cloud-recorder
  'behavioral-cloning', // → cloud-behavioral-cloning
  'incidents', // → cloud-incidents
  'traces', // → cloud-traces
  'contract-diff', // → cloud-contract
  'plugins', // → plugin-registry
  'test-execution', // → cloud-test-runs (TestExecutionDashboard is mock-data only)
  'analytics', // → pillar-analytics (request-traffic analytics is local-only)
  'metrics', // → pillar-analytics (#463)
  'performance', // → cloud-test-runs (#467)
  'api-explorer', // → reached via HostedMocksPage "Open" action
]);

// With the local sibling hidden, the "Cloud" prefix on the cloud variant is
// just noise — every page in the cloud app is a cloud page. Show the plain
// feature name instead (#394 did the same for Analytics).
const cloudLabelOverrides: Record<string, string> = {
  'pillar-analytics': 'tab.analytics',
  'cloud-chaos': 'tab.chaosEngineering',
  'cloud-recorder': 'tab.recorder',
  'cloud-behavioral-cloning': 'tab.behavioralCloning',
  'cloud-incidents': 'tab.incidents',
  'cloud-traces': 'tab.traces',
  'cloud-contract': 'tab.contracts',
  'cloud-test-runs': 'tab.testRuns',
  'cloud-flows': 'tab.flowLibrary',
  'cloud-plugins': 'tab.plugins',
};

/** Extra search terms so users can find a page by concept, not exact label. */
const navSearchKeywords: Record<string, string[]> = {
  dashboard: ['home', 'overview'],
  'hosted-mocks': ['deploy', 'deployments', 'cloud mocks'],
  fixtures: ['overrides', 'responses', 'stubs', 'canned'],
  config: ['settings', 'overrides', 'latency', 'validation', 'reality', 'environment', 'env vars'],
  organization: ['team', 'teams', 'members', 'invite', 'roles', 'sso', 'audit log'],
  billing: ['plan', 'subscription', 'invoices', 'payment', 'upgrade'],
  'api-tokens': ['keys', 'personal access tokens', 'pat', 'credentials'],
  'publisher-keys': ['signing', 'signatures'],
  byok: ['llm', 'openai', 'anthropic', 'api key', 'bring your own key'],
  usage: ['quota', 'limits', 'consumption'],
  'notification-channels': ['alerts', 'slack', 'pagerduty', 'email', 'webhooks'],
  observability: ['monitoring', 'dashboards', 'tiles', 'metrics'],
  status: ['health', 'uptime', 'services'],
  'cloud-incidents': ['alerts', 'outages', 'cloud'],
  'cloud-traces': ['tracing', 'otel', 'opentelemetry', 'spans', 'cloud'],
  'pillar-analytics': ['analytics', 'metrics', 'traffic'],
  'plugin-registry': ['plugins', 'extensions', 'marketplace'],
  'cloud-chaos': ['fault injection', 'failures', 'latency', 'cloud'],
  'cloud-test-runs': ['runs', 'history', 'cloud'],
  'cloud-flows': ['flows', 'versions', 'cloud'],
  'cloud-contract': ['contract', 'diff', 'drift', 'cloud'],
  tunnels: ['expose', 'public url', 'ngrok'],
  support: ['help', 'contact'],
  faq: ['help', 'questions'],
};

export const isCloud = detectCloudMode();

function resolveItem(item: NavItem, sectionId: string, cloud: boolean): ResolvedNavItem {
  return {
    ...item,
    sectionId,
    labelKey: (cloud && cloudLabelOverrides[item.id]) || item.labelKey,
    localOnly: cloud && !cloudNavItemIds.has(item.id),
  };
}

/**
 * Sections as the sidebar renders them. In cloud mode, superseded items are
 * dropped and non-functional ones are moved into a trailing `local-only`
 * section instead of being sprinkled (disabled) through every group.
 */
export function getNavSections(cloud: boolean = isCloud): ResolvedNavSection[] {
  const resolved = navSections.map((section) => ({
    ...section,
    items: section.items
      .filter((item) => !(cloud && cloudHiddenNavItemIds.has(item.id)))
      .map((item) => resolveItem(item, section.id, cloud)),
  }));

  const localOnly = resolved.flatMap((section) => section.items.filter((i) => i.localOnly));
  const active = resolved
    .map((section) => ({ ...section, items: section.items.filter((i) => !i.localOnly) }))
    .filter((section) => section.items.length > 0);

  if (localOnly.length > 0) {
    active.push({ id: 'local-only', titleKey: 'nav.localOnly.section', items: localOnly });
  }
  return active;
}

export function getHelpNavItems(cloud: boolean = isCloud): ResolvedNavItem[] {
  return helpNavItems.map((item) => resolveItem(item, 'help', cloud));
}

/** Every page the shell knows about, for breadcrumb/title lookup. */
export function findNavItem(
  id: string,
  cloud: boolean = isCloud,
): { item: ResolvedNavItem; section?: NavSection } | undefined {
  for (const section of navSections) {
    const item = section.items.find((i) => i.id === id);
    if (item) return { item: resolveItem(item, section.id, cloud), section };
  }
  const extra = [...helpNavItems, ...unlistedNavItems].find((i) => i.id === id);
  return extra ? { item: resolveItem(extra, 'help', cloud) } : undefined;
}

export function matchesNavQuery(item: { id: string }, label: string, query: string): boolean {
  if (!query) return true;
  const haystack = [label, item.id.replace(/-/g, ' '), ...(navSearchKeywords[item.id] ?? [])]
    .join(' ')
    .toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((term) => haystack.includes(term));
}

/* Recently visited pages (feeds the empty command palette). */

const RECENT_KEY = 'mockforge-recent-pages';
const MAX_RECENT = 5;

export function readRecentPages(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === 'string') : [];
  } catch {
    return [];
  }
}

/** Remember the last few pages visited so an empty palette has something useful. */
export function recordRecentPage(id: string) {
  if (!findNavItem(id)) return;
  try {
    const next = [id, ...readRecentPages().filter((r) => r !== id)].slice(0, MAX_RECENT);
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    // Storage unavailable (private mode) — recents are a convenience only.
  }
}
