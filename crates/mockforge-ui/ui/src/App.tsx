import React, { useEffect, Suspense } from 'react';
import { Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { AppShell } from './components/layout/AppShell';
import { AuthGuard } from './components/auth/AuthGuard';
import { ErrorBoundary } from './components/error/ErrorBoundary';
import { ToastProvider } from './components/ui/ToastProvider';
import { useStartupPrefetch } from './hooks/usePrefetch';
import { useWorkspaceStore } from './stores/useWorkspaceStore';
import { useAuthStore } from './stores/useAuthStore';
import { usePreferencesStore } from './stores/usePreferencesStore';
import { useThemeSync } from './hooks/useThemeSync';
import { useI18n } from './i18n/I18nProvider';
import { routes } from './routes';

function NotFoundPage() {
  const { t } = useI18n();
  const navigate = useNavigate();

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-center py-12">
        <div className="text-center">
          <div className="text-6xl mb-4">🚧</div>
          <h2 className="text-2xl font-bold text-foreground mb-2">
            {t('app.pageNotFoundTitle')}
          </h2>
          <p className="text-muted-foreground mb-6">
            {t('app.pageNotFoundBody')}
          </p>
          <button
            onClick={() => navigate('/dashboard')}
            className="px-6 py-3 bg-primary text-primary-foreground hover:bg-primary/90 rounded-lg font-medium transition-colors"
          >
            {t('app.goToDashboard')}
          </button>
        </div>
      </div>
    </div>
  );
}

function App() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const loadWorkspaces = useWorkspaceStore(state => state.loadWorkspaces);
  const isAuthenticated = useAuthStore(state => state.isAuthenticated);
  const loadPreferences = usePreferencesStore(state => state.loadPreferences);
  const defaultPage = usePreferencesStore(state => state.preferences.ui.defaultPage);
  const rootRedirect = defaultPage && defaultPage !== '' ? `/${defaultPage}` : '/dashboard';

  // Apply theme preferences (accent color, font size, high contrast) to <html>
  useThemeSync();

  // Prefetch data on startup for better performance
  useStartupPrefetch();

  // Load workspaces once the user is authenticated. Firing this before auth
  // resolves causes a 401 on /api/v1/workspaces (and wasted network) in cloud
  // mode, since the persisted token may be expired or revoked.
  useEffect(() => {
    if (isAuthenticated) {
      loadWorkspaces();
      // Hydrate preferences from the server; merges over the localStorage cache.
      void loadPreferences();
    }
  }, [isAuthenticated, loadWorkspaces, loadPreferences]);

  // Handle deep-link navigation events (e.g., from RealityTracePanel)
  useEffect(() => {
    const handleNavigate = (event: CustomEvent<{ target: 'persona' | 'scenario' | 'chaos'; id: string }>) => {
      const { target } = event.detail;

      if (target === 'chaos') {
        navigate('/chaos');
      } else if (target === 'scenario') {
        navigate('/scenario-studio');
      } else if (target === 'persona') {
        navigate('/ai-studio');
      }
    };

    // Legacy: handle navigate-tab events from components not yet migrated
    const handleNavigateTab = (event: CustomEvent<{ tab: string }>) => {
      const { tab } = event.detail;
      if (tab) {
        navigate('/' + tab);
      }
    };

    window.addEventListener('navigate', handleNavigate as EventListener);
    window.addEventListener('navigate-tab', handleNavigateTab as EventListener);
    return () => {
      window.removeEventListener('navigate', handleNavigate as EventListener);
      window.removeEventListener('navigate-tab', handleNavigateTab as EventListener);
    };
  }, [navigate]);

  // Handle Tauri file open events (desktop app only)
  useEffect(() => {
    import('@/utils/tauri').then(({ isTauri, listenToTauriEvent }) => {
      if (isTauri) {
        // Listen for file-opened events
        const cleanup1 = listenToTauriEvent<string>('file-opened', (filePath) => {
          import('@/utils/tauri').then(({ handleFileOpen }) => {
            handleFileOpen(filePath).catch((err) => {
              console.error('Failed to handle file open:', err);
            });
          });
        });

        // Listen for file-dropped events
        const cleanup2 = listenToTauriEvent<string>('file-dropped', (filePath) => {
          import('@/utils/tauri').then(({ handleFileOpen }) => {
            handleFileOpen(filePath).catch((err) => {
              console.error('Failed to handle file drop:', err);
            });
          });
        });

        // Listen for config-file-opened events
        const cleanup3 = listenToTauriEvent<string>('config-file-opened', (_configContent) => {
          // Config file has been opened and loaded by backend
        });

        return () => {
          cleanup1();
          cleanup2();
          cleanup3();
        };
      }
    });
  }, []);

  const handleRefresh = () => {
    // Refresh data for the current page
  };

  return (
    <ErrorBoundary>
      <ToastProvider>
        <AuthGuard>
          <AppShell onRefresh={handleRefresh}>
            <ErrorBoundary>
              <Suspense fallback={
                <div className="flex items-center justify-center h-64">
                  <div className="text-center">
                    <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-info-600"></div>
                    <p className="mt-4 text-muted-foreground">{t('app.loading')}</p>
                  </div>
                </div>
              }>
                <Routes>
                  {/* Redirect root to the user's preferred default page (falls back to /dashboard). */}
                  <Route path="/" element={<Navigate to={rootRedirect} replace />} />

                  {/* All page routes */}
                  {routes.map((route) => (
                    <Route key={route.path} path={route.path} element={route.element} />
                  ))}

                  {/* Catch-all: show not found page */}
                  <Route path="*" element={<NotFoundPage />} />
                </Routes>
              </Suspense>
            </ErrorBoundary>
          </AppShell>
        </AuthGuard>
      </ToastProvider>
    </ErrorBoundary>
  );
}

export default App;
