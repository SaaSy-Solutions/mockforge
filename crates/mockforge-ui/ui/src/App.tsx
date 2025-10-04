import React, { useState, useEffect } from 'react';
import { AppShell } from './components/layout/AppShell';
import { DashboardPage } from './pages/DashboardPage';
import { ServicesPage } from './pages/ServicesPage';
import { LogsPage } from './pages/LogsPage';
import { MetricsPage } from './pages/MetricsPage';
import { FixturesPage } from './pages/FixturesPage';
import { ConfigPage } from './pages/ConfigPage';
import { TestingPage } from './pages/TestingPage';
import { ImportPage } from './pages/ImportPage';
import WorkspacesPage from './pages/WorkspacesPage';
import { PluginsPage } from './pages/PluginsPage';
import { ChainsPage } from './pages/ChainsPage';
import { AuthGuard } from './components/auth/AuthGuard';
import { ErrorBoundary } from './components/error/ErrorBoundary';
import { ToastProvider } from './components/ui/ToastProvider';
import { useStartupPrefetch } from './hooks/usePrefetch';
import { useWorkspaceStore } from './stores/useWorkspaceStore';

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
              {renderPage()}
            </ErrorBoundary>
          </AppShell>
        </AuthGuard>
      </ToastProvider>
    </ErrorBoundary>
  );
}

export default App;
