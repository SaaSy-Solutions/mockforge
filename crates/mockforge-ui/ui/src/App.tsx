import { logger } from '@/utils/logger';
import React, { useState, useEffect, Suspense, lazy } from 'react';
import { AppShell } from './components/layout/AppShell';
import { AuthGuard } from './components/auth/AuthGuard';
import { ErrorBoundary } from './components/error/ErrorBoundary';
import { ToastProvider } from './components/ui/ToastProvider';
import { useStartupPrefetch } from './hooks/usePrefetch';
import { useWorkspaceStore } from './stores/useWorkspaceStore';

// Lazy load all pages for better code splitting
const DashboardPage = lazy(() => import('./pages/DashboardPage').then(m => ({ default: m.DashboardPage })));
const ServicesPage = lazy(() => import('./pages/ServicesPage').then(m => ({ default: m.ServicesPage })));
const LogsPage = lazy(() => import('./pages/LogsPage').then(m => ({ default: m.LogsPage })));
const MetricsPage = lazy(() => import('./pages/MetricsPage').then(m => ({ default: m.MetricsPage })));
const VerificationPage = lazy(() => import('./pages/VerificationPage').then(m => ({ default: m.VerificationPage })));
const ContractDiffPage = lazy(() => import('./pages/ContractDiffPage').then(m => ({ default: m.ContractDiffPage })));
const FixturesPage = lazy(() => import('./pages/FixturesPage').then(m => ({ default: m.FixturesPage })));
const ConfigPage = lazy(() => import('./pages/ConfigPage').then(m => ({ default: m.ConfigPage })));
const TestingPage = lazy(() => import('./pages/TestingPage').then(m => ({ default: m.TestingPage })));
const ImportPage = lazy(() => import('./pages/ImportPage').then(m => ({ default: m.ImportPage })));
const WorkspacesPage = lazy(() => import('./pages/WorkspacesPage'));
const PluginsPage = lazy(() => import('./pages/PluginsPage').then(m => ({ default: m.PluginsPage })));
const ChainsPage = lazy(() => import('./pages/ChainsPage').then(m => ({ default: m.ChainsPage })));
const GraphPage = lazy(() => import('./pages/GraphPage').then(m => ({ default: m.GraphPage })));
const ScenarioStateMachineEditor = lazy(() => import('./pages/ScenarioStateMachineEditor').then(m => ({ default: m.ScenarioStateMachineEditor })));
const AnalyticsPage = lazy(() => import('./pages/AnalyticsPage'));

// Observability & Monitoring
const ObservabilityPage = lazy(() => import('./pages/ObservabilityPage').then(m => ({ default: m.ObservabilityPage })));
const TracesPage = lazy(() => import('./pages/TracesPage').then(m => ({ default: m.TracesPage })));

// Testing
const TestGeneratorPage = lazy(() => import('./pages/TestGeneratorPage'));
const TestExecutionDashboard = lazy(() => import('./pages/TestExecutionDashboard'));
const IntegrationTestBuilder = lazy(() => import('./pages/IntegrationTestBuilder'));

// Chaos & Resilience
const ChaosPage = lazy(() => import('./pages/ChaosPage').then(m => ({ default: m.ChaosPage })));
const ResiliencePage = lazy(() => import('./pages/ResiliencePage').then(m => ({ default: m.ResiliencePage })));
const RecorderPage = lazy(() => import('./pages/RecorderPage').then(m => ({ default: m.RecorderPage })));

// Time Travel
const TimeTravelPage = lazy(() => import('./pages/TimeTravelPage').then(m => ({ default: m.TimeTravelPage })));

// Proxy Inspector
const ProxyInspectorPage = lazy(() => import('./pages/ProxyInspectorPage').then(m => ({ default: m.ProxyInspectorPage })));

// Orchestration
const OrchestrationBuilder = lazy(() => import('./pages/OrchestrationBuilder'));
const OrchestrationExecutionView = lazy(() => import('./pages/OrchestrationExecutionView'));

// Plugins & Templates
const PluginRegistryPage = lazy(() => import('./pages/PluginRegistryPage'));
const TemplateMarketplacePage = lazy(() => import('./pages/TemplateMarketplacePage'));

// User Management
const UserManagementPage = lazy(() => import('./pages/UserManagementPage').then(m => ({ default: m.UserManagementPage })));

// MockAI
const MockAIPage = lazy(() => import('./pages/MockAIPage').then(m => ({ default: m.MockAIPage })));
const MockAIOpenApiGeneratorPage = lazy(() => import('./pages/MockAIOpenApiGeneratorPage').then(m => ({ default: m.MockAIOpenApiGeneratorPage })));
const MockAIRulesPage = lazy(() => import('./pages/MockAIRulesPage').then(m => ({ default: m.MockAIRulesPage })));

function App() {
  const [activeTab, setActiveTab] = useState('dashboard');
  const loadWorkspaces = useWorkspaceStore(state => state.loadWorkspaces);

  // Prefetch data on startup for better performance
  useStartupPrefetch();

  // Load workspaces on app startup
  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  const handleRefresh = () => {
    // Refresh data for the current page
  };

  const renderPage = () => {
    switch (activeTab) {
      // Core
      case 'dashboard':
        return <DashboardPage />;
      case 'workspaces':
        return <WorkspacesPage />;

      // Services & Data
      case 'services':
        return <ServicesPage />;
      case 'fixtures':
        return <FixturesPage />;

      // Orchestration
      case 'chains':
        return <ChainsPage />;
      case 'graph':
        return <GraphPage />;
      case 'state-machine-editor':
        return <ScenarioStateMachineEditor />;
      case 'orchestration-builder':
        return <OrchestrationBuilder />;
      case 'orchestration-execution':
        // OrchestrationExecutionView requires an orchestrationId prop
        // For navigation, we'll show it with an empty ID (it will handle loading state)
        // In a real app, this would typically be accessed via a link from Orchestration Builder
        return <OrchestrationExecutionView orchestrationId="default" />;

      // Observability & Monitoring
      case 'observability':
        return <ObservabilityPage />;
      case 'logs':
        return <LogsPage />;
      case 'traces':
        return <TracesPage />;
      case 'metrics':
        return <MetricsPage />;
      case 'analytics':
        return <AnalyticsPage />;
      case 'verification':
        return <VerificationPage />;
      case 'contract-diff':
        return <ContractDiffPage />;

      // Testing
      case 'testing':
        return <TestingPage />;
      case 'test-generator':
        return <TestGeneratorPage />;
      case 'test-execution':
        return <TestExecutionDashboard />;
      case 'integration-test-builder':
        return <IntegrationTestBuilder />;

      // Chaos & Resilience
      case 'chaos':
        return <ChaosPage />;
      case 'resilience':
        return <ResiliencePage />;
      case 'recorder':
        return <RecorderPage />;

      // Import & Templates
      case 'import':
        return <ImportPage />;
      case 'template-marketplace':
        return <TemplateMarketplacePage />;

      // Plugins
      case 'plugins':
        return <PluginsPage />;
      case 'plugin-registry':
        return <PluginRegistryPage />;

      // User Management
      case 'user-management':
        return <UserManagementPage />;

      // MockAI
      case 'mockai':
        return <MockAIPage />;
      case 'mockai-openapi-generator':
        return <MockAIOpenApiGeneratorPage />;
      case 'mockai-rules':
        return <MockAIRulesPage />;

      // Configuration
      case 'config':
        return <ConfigPage />;

      // Time Travel
      case 'time-travel':
        return <TimeTravelPage />;

      // Proxy Inspector
      case 'proxy-inspector':
        return <ProxyInspectorPage />;

      default:
        return (
          <div className="space-y-8">
            <div className="flex items-center justify-center py-12">
              <div className="text-center">
                <div className="text-6xl mb-4">🚧</div>
                <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                  Page Not Found
                </h2>
                <p className="text-gray-600 dark:text-gray-400 mb-6">
                  The page you're looking for doesn't exist yet.
                </p>
                <button
                  onClick={() => setActiveTab('dashboard')}
                  className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                  Go to Dashboard
                </button>
              </div>
            </div>
          </div>
        );
    }
  };

  return (
    <ErrorBoundary>
      <ToastProvider>
        <AuthGuard>
          <AppShell activeTab={activeTab} onTabChange={setActiveTab} onRefresh={handleRefresh}>
            <ErrorBoundary>
              <Suspense fallback={
                <div className="flex items-center justify-center h-64">
                  <div className="text-center">
                    <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    <p className="mt-4 text-gray-600 dark:text-gray-400">Loading...</p>
                  </div>
                </div>
              }>
                {renderPage()}
              </Suspense>
            </ErrorBoundary>
          </AppShell>
        </AuthGuard>
      </ToastProvider>
    </ErrorBoundary>
  );
}

export default App;
