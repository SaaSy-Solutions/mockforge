import React, { lazy } from 'react';
import { Navigate } from 'react-router-dom';

// Eagerly loaded pages (small, frequently used, or needed for initial render)
import { VirtualBackendsPage } from '../pages/VirtualBackendsPage';
import { TunnelsPage } from '../pages/TunnelsPage';
import { FederationPage } from '../pages/FederationPage';
import { ConfigPage } from '../pages/ConfigPage';
import { BYOKConfigPage } from '../pages/BYOKConfigPage';
import { StatusPage } from '../pages/StatusPage';
import { OrganizationPage } from '../pages/OrganizationPage';
import { BillingPage } from '../pages/BillingPage';
import { ApiTokensPage } from '../pages/ApiTokensPage';
import PublisherKeysPage from '../pages/PublisherKeysPage';
import { UsageDashboardPage } from '../pages/UsageDashboardPage';
import { TimeTravelPage } from '../pages/TimeTravelPage';
import { ProxyInspectorPage } from '../pages/ProxyInspectorPage';

// Lazy-loaded pages
const DashboardPage = lazy(() => import('../pages/DashboardPage').then(m => ({ default: m.DashboardPage })));
const ServicesPage = lazy(() => import('../pages/ServicesPage').then(m => ({ default: m.ServicesPage })));
const LogsPage = lazy(() => import('../pages/LogsPage').then(m => ({ default: m.LogsPage })));
const MetricsPage = lazy(() => import('../pages/MetricsPage').then(m => ({ default: m.MetricsPage })));
const VerificationPage = lazy(() => import('../pages/VerificationPage').then(m => ({ default: m.VerificationPage })));
const ContractDiffPage = lazy(() => import('../pages/ContractDiffPage').then(m => ({ default: m.ContractDiffPage })));
const IncidentDashboardPage = lazy(() => import('../pages/IncidentDashboardPage').then(m => ({ default: m.IncidentDashboardPage })));
const FitnessFunctionsPage = lazy(() => import('../pages/FitnessFunctionsPage').then(m => ({ default: m.FitnessFunctionsPage })));
const FixturesPage = lazy(() => import('../pages/FixturesPage').then(m => ({ default: m.FixturesPage })));
const TestingPage = lazy(() => import('../pages/TestingPage').then(m => ({ default: m.TestingPage })));
const ImportPage = lazy(() => import('../pages/ImportPage').then(m => ({ default: m.ImportPage })));
const WorkspacesPage = lazy(() => import('../pages/WorkspacesPage'));
const PlaygroundPage = lazy(() => import('../pages/PlaygroundPage').then(m => ({ default: m.PlaygroundPage })));
const PluginsPage = lazy(() => import('../pages/PluginsPage').then(m => ({ default: m.PluginsPage })));
const ChainsPage = lazy(() => import('../pages/ChainsPage').then(m => ({ default: m.ChainsPage })));
const GraphPage = lazy(() => import('../pages/GraphPage').then(m => ({ default: m.GraphPage })));
const WorldStatePage = lazy(() => import('../pages/WorldStatePage').then(m => ({ default: m.WorldStatePage })));
const PerformancePage = lazy(() => import('../pages/PerformancePage').then(m => ({ default: m.default })));
const ScenarioStateMachineEditor = lazy(() => import('../pages/ScenarioStateMachineEditor').then(m => ({ default: m.ScenarioStateMachineEditor })));
const ScenarioStudioPage = lazy(() => import('../pages/ScenarioStudioPage').then(m => ({ default: m.ScenarioStudioPage })));
const AnalyticsPage = lazy(() => import('../pages/AnalyticsPage'));
const PillarAnalyticsPage = lazy(() => import('../pages/PillarAnalyticsPage').then(m => ({ default: m.PillarAnalyticsPage })));
const HostedMocksPage = lazy(() => import('../pages/HostedMocksPage').then(m => ({ default: m.HostedMocksPage })));
const ApiExplorerPage = lazy(() => import('../pages/ApiExplorerPage').then(m => ({ default: m.ApiExplorerPage })));

// Observability & Monitoring
const ObservabilityPage = lazy(() => import('../pages/ObservabilityPage').then(m => ({ default: m.ObservabilityPage })));
const TracesPage = lazy(() => import('../pages/TracesPage').then(m => ({ default: m.TracesPage })));

// Testing
const TestGeneratorPage = lazy(() => import('../pages/TestGeneratorPage'));
const TestExecutionDashboard = lazy(() => import('../pages/TestExecutionDashboard'));
const IntegrationTestBuilder = lazy(() => import('../pages/IntegrationTestBuilder'));
const ConformancePage = lazy(() => import('../pages/ConformancePage').then(m => ({ default: m.ConformancePage })));

// Chaos & Resilience
const ChaosPage = lazy(() => import('../pages/ChaosPage').then(m => ({ default: m.ChaosPage })));
const ResiliencePage = lazy(() => import('../pages/ResiliencePage').then(m => ({ default: m.ResiliencePage })));
const RecorderPage = lazy(() => import('../pages/RecorderPage').then(m => ({ default: m.RecorderPage })));
const BehavioralCloningPage = lazy(() => import('../pages/BehavioralCloningPage').then(m => ({ default: m.BehavioralCloningPage })));

// Orchestration
const OrchestrationBuilder = lazy(() => import('../pages/OrchestrationBuilder'));
const OrchestrationExecutionView = lazy(() => import('../pages/OrchestrationExecutionView'));

// Plugins & Templates
const PluginRegistryPage = lazy(() => import('../pages/PluginRegistryPage'));
const TemplateMarketplacePage = lazy(() => import('../pages/TemplateMarketplacePage'));
const ScenarioMarketplacePage = lazy(() => import('../pages/ScenarioMarketplacePage'));

// Community Portal
const ShowcasePage = lazy(() => import('../pages/ShowcasePage').then(m => ({ default: m.ShowcasePage })));
const LearningHubPage = lazy(() => import('../pages/LearningHubPage').then(m => ({ default: m.LearningHubPage })));

// User Management
const UserManagementPage = lazy(() => import('../pages/UserManagementPage').then(m => ({ default: m.UserManagementPage })));

// Registry admin (cloud: Postgres via /api/v1/*, self-hosted: SQLite via /api/admin/registry/*)
const RegistryLoginPage = lazy(() => import('../pages/RegistryLoginPage').then(m => ({ default: m.RegistryLoginPage })));
const RegistryAdminPage = lazy(() => import('../pages/RegistryAdminPage').then(m => ({ default: m.RegistryAdminPage })));
const RegistryInvitePage = lazy(() => import('../pages/RegistryInvitePage').then(m => ({ default: m.RegistryInvitePage })));

// MockAI
const MockAIPage = lazy(() => import('../pages/MockAIPage').then(m => ({ default: m.MockAIPage })));
const MockAIOpenApiGeneratorPage = lazy(() => import('../pages/MockAIOpenApiGeneratorPage').then(m => ({ default: m.MockAIOpenApiGeneratorPage })));
const MockAIRulesPage = lazy(() => import('../pages/MockAIRulesPage').then(m => ({ default: m.MockAIRulesPage })));

// Voice + LLM Interface
const VoicePage = lazy(() => import('../pages/VoicePage').then(m => ({ default: m.VoicePage })));

// AI Studio - Unified AI Copilot
const AIStudioPage = lazy(() => import('../pages/AIStudioPage').then(m => ({ default: m.AIStudioPage })));

/**
 * ApiExplorerWrapper handles the special logic that was previously in the
 * switch statement: it checks for window.__mockforge_explorer_deployment
 * and redirects to /hosted-mocks if no deployment context is set.
 */
function ApiExplorerWrapper() {
  const dep = window.__mockforge_explorer_deployment;
  if (!dep) {
    return <Navigate to="/hosted-mocks" replace />;
  }
  return (
    <ApiExplorerPage
      deployment={dep}
      onBack={() => {
        window.history.back();
      }}
    />
  );
}

export interface RouteConfig {
  path: string;
  element: React.ReactNode;
}

export const routes: RouteConfig[] = [
  // Core
  { path: '/dashboard', element: <DashboardPage /> },
  { path: '/workspaces', element: <WorkspacesPage /> },
  { path: '/playground', element: <PlaygroundPage /> },
  { path: '/federation', element: <FederationPage /> },

  // Services & Data
  { path: '/services', element: <ServicesPage /> },
  { path: '/virtual-backends', element: <VirtualBackendsPage /> },
  { path: '/fixtures', element: <FixturesPage /> },
  { path: '/hosted-mocks', element: <HostedMocksPage /> },
  { path: '/api-explorer', element: <ApiExplorerWrapper /> },
  { path: '/tunnels', element: <TunnelsPage /> },

  // Orchestration
  { path: '/chains', element: <ChainsPage /> },
  { path: '/graph', element: <GraphPage /> },
  { path: '/world-state', element: <WorldStatePage /> },
  { path: '/performance', element: <PerformancePage /> },
  { path: '/state-machine-editor', element: <ScenarioStateMachineEditor /> },
  { path: '/scenario-studio', element: <ScenarioStudioPage /> },
  { path: '/orchestration-builder', element: <OrchestrationBuilder /> },
  { path: '/orchestration-execution', element: <OrchestrationExecutionView orchestrationId="default" /> },

  // Observability & Monitoring
  { path: '/observability', element: <ObservabilityPage /> },
  { path: '/status', element: <StatusPage /> },
  { path: '/logs', element: <LogsPage /> },
  { path: '/traces', element: <TracesPage /> },
  { path: '/metrics', element: <MetricsPage /> },
  { path: '/analytics', element: <AnalyticsPage /> },
  { path: '/pillar-analytics', element: <PillarAnalyticsPage /> },
  { path: '/verification', element: <VerificationPage /> },
  { path: '/contract-diff', element: <ContractDiffPage /> },
  { path: '/incidents', element: <IncidentDashboardPage /> },
  { path: '/incident-dashboard', element: <IncidentDashboardPage /> },
  { path: '/fitness-functions', element: <FitnessFunctionsPage /> },

  // Testing
  { path: '/testing', element: <TestingPage /> },
  { path: '/test-generator', element: <TestGeneratorPage /> },
  { path: '/test-execution', element: <TestExecutionDashboard /> },
  { path: '/integration-test-builder', element: <IntegrationTestBuilder /> },
  { path: '/conformance', element: <ConformancePage /> },

  // Chaos & Resilience
  { path: '/chaos', element: <ChaosPage /> },
  { path: '/resilience', element: <ResiliencePage /> },
  { path: '/recorder', element: <RecorderPage /> },
  { path: '/behavioral-cloning', element: <BehavioralCloningPage /> },

  // Import & Templates
  { path: '/import', element: <ImportPage /> },
  { path: '/template-marketplace', element: <TemplateMarketplacePage /> },
  { path: '/scenario-marketplace', element: <ScenarioMarketplacePage /> },

  // Community Portal
  { path: '/showcase', element: <ShowcasePage /> },
  { path: '/learning-hub', element: <LearningHubPage /> },

  // Plugins
  { path: '/plugins', element: <PluginsPage /> },
  { path: '/plugin-registry', element: <PluginRegistryPage /> },

  // User Management
  { path: '/user-management', element: <UserManagementPage /> },

  // Registry admin (cloud: Postgres, self-hosted: SQLite)
  { path: '/registry-login', element: <RegistryLoginPage /> },
  { path: '/registry-admin', element: <RegistryAdminPage /> },
  { path: '/registry-admin/invite/:token', element: <RegistryInvitePage /> },

  // MockAI
  { path: '/mockai', element: <MockAIPage /> },
  { path: '/mockai-openapi-generator', element: <MockAIOpenApiGeneratorPage /> },
  { path: '/mockai-rules', element: <MockAIRulesPage /> },

  // Voice + LLM Interface
  { path: '/voice', element: <VoicePage /> },

  // AI Studio - Unified AI Copilot
  { path: '/ai-studio', element: <AIStudioPage /> },

  // Configuration
  { path: '/config', element: <ConfigPage /> },
  { path: '/organization', element: <OrganizationPage /> },
  { path: '/billing', element: <BillingPage /> },
  { path: '/api-tokens', element: <ApiTokensPage /> },
  { path: '/publisher-keys', element: <PublisherKeysPage /> },
  { path: '/byok', element: <BYOKConfigPage /> },
  { path: '/usage', element: <UsageDashboardPage /> },

  // Time Travel
  { path: '/time-travel', element: <TimeTravelPage /> },

  // Proxy Inspector
  { path: '/proxy-inspector', element: <ProxyInspectorPage /> },
];
