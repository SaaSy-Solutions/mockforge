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
const FixturesPage = lazy(() => import('./pages/FixturesPage').then(m => ({ default: m.FixturesPage })));
const ConfigPage = lazy(() => import('./pages/ConfigPage').then(m => ({ default: m.ConfigPage })));
const TestingPage = lazy(() => import('./pages/TestingPage').then(m => ({ default: m.TestingPage })));
const ImportPage = lazy(() => import('./pages/ImportPage').then(m => ({ default: m.ImportPage })));
const WorkspacesPage = lazy(() => import('./pages/WorkspacesPage'));
const PluginsPage = lazy(() => import('./pages/PluginsPage').then(m => ({ default: m.PluginsPage })));
const ChainsPage = lazy(() => import('./pages/ChainsPage').then(m => ({ default: m.ChainsPage })));

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
      case 'dashboard':
        return <DashboardPage />;
      case 'services':
        return <ServicesPage />;
      case 'chains':
        return <ChainsPage />;
      case 'logs':
        return <LogsPage />;
      case 'metrics':
        return <MetricsPage />;
      case 'fixtures':
        return <FixturesPage />;
      case 'import':
        return <ImportPage />;
      case 'workspaces':
        return <WorkspacesPage />;
      case 'testing':
        return <TestingPage />;
      case 'config':
        return <ConfigPage />;
      case 'plugins':
        return <PluginsPage />;
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
