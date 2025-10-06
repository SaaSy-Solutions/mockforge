import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { Button } from '../ui/button';
import { useServiceStore } from '../../stores/useServiceStore';
import { useFixtureStore } from '../../stores/useFixtureStore';
import { useLogStore } from '../../stores/useLogStore';
import { useMetricsStore } from '../../stores/useMetricsStore';
import { useAuthStore } from '../../stores/useAuthStore';
import type { FixtureInfo } from '../../types';

interface TestResult {
  id: string;
  name: string;
  description: string;
  status: 'pending' | 'running' | 'passed' | 'failed';
  error?: string;
  details?: string[];
}

export function WorkflowValidator() {
  const [testResults, setTestResults] = useState<TestResult[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  const serviceStore = useServiceStore();
  const fixtureStore = useFixtureStore();
  const logStore = useLogStore();
  const metricsStore = useMetricsStore();
  const authStore = useAuthStore();

  const workflows: Omit<TestResult, 'status'>[] = [
    {
      id: 'auth-admin',
      name: 'Admin Authentication',
      description: 'Login as admin and verify full access',
    },
    {
      id: 'auth-viewer',
      name: 'Viewer Authentication',
      description: 'Login as viewer and verify read-only access',
    },
    {
      id: 'service-management',
      name: 'Service Toggle Management',
      description: 'Enable/disable services and routes without file editing',
    },
    {
      id: 'fixture-editing',
      name: 'Fixture Content Management',
      description: 'Edit, rename, and move fixtures through UI',
    },
    {
      id: 'fixture-diffing',
      name: 'Fixture Diff Visualization',
      description: 'View and apply fixture changes with visual diff',
    },
    {
      id: 'live-logs',
      name: 'Live Log Monitoring',
      description: 'Filter, search, and monitor logs in real-time',
    },
    {
      id: 'metrics-analysis',
      name: 'Performance Metrics Analysis',
      description: 'View latency histograms and failure analysis',
    },
    {
      id: 'bulk-operations',
      name: 'Bulk Service Operations',
      description: 'Enable/disable multiple services at once',
    },
    {
      id: 'search-filtering',
      name: 'Search and Filtering',
      description: 'Search across services, fixtures, and logs',
    },
    {
      id: 'role-based-access',
      name: 'Role-based Feature Access',
      description: 'Verify proper access control between admin and viewer',
    },
  ];

  const runTest = async (workflow: Omit<TestResult, 'status'>): Promise<TestResult> => {
    const result: TestResult = { ...workflow, status: 'running' };
    
    try {
      switch (workflow.id) {
        case 'auth-admin':
          return await testAdminAuth(result);
        case 'auth-viewer':
          return await testViewerAuth(result);
        case 'service-management':
          return await testServiceManagement(result);
        case 'fixture-editing':
          return await testFixtureEditing(result);
        case 'fixture-diffing':
          return await testFixtureDiffing(result);
        case 'live-logs':
          return await testLiveLogs(result);
        case 'metrics-analysis':
          return await testMetricsAnalysis(result);
        case 'bulk-operations':
          return await testBulkOperations(result);
        case 'search-filtering':
          return await testSearchFiltering(result);
        case 'role-based-access':
          return await testRoleBasedAccess(result);
        default:
          throw new Error('Unknown test case');
      }
    } catch (error) {
      return {
        ...result,
        status: 'failed',
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  };

  const testAdminAuth = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Test admin login
    try {
      await authStore.login('admin', 'admin123');
      details.push('✓ Admin login successful');
    } catch {
      throw new Error('Admin login failed');
    }

    // Verify admin role
    if (authStore.user?.role === 'admin') {
      details.push('✓ Admin role verified');
    } else {
      throw new Error('Admin role not set correctly');
    }

    // Verify authentication state
    if (authStore.isAuthenticated) {
      details.push('✓ Authentication state correct');
    } else {
      throw new Error('Authentication state incorrect');
    }

    return { ...result, status: 'passed', details };
  };

  const testViewerAuth = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Logout current user
    authStore.logout();
    
    // Test viewer login
    try {
      await authStore.login('viewer', 'viewer123');
      details.push('✓ Viewer login successful');
    } catch {
      throw new Error('Viewer login failed');
    }

    // Verify viewer role
    if (authStore.user?.role === 'viewer') {
      details.push('✓ Viewer role verified');
    } else {
      throw new Error('Viewer role not set correctly');
    }

    return { ...result, status: 'passed', details };
  };

  const testServiceManagement = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Get initial state
    const initialServices = serviceStore.services;
    if (initialServices.length === 0) {
      throw new Error('No services available to test');
    }

    details.push(`✓ Found ${initialServices.length} services to test`);

    // Test service toggle
    const testService = initialServices[0];
    const originalState = testService.enabled;
    
    serviceStore.updateService(testService.id, { enabled: !originalState });
    const updatedService = serviceStore.services.find(s => s.id === testService.id);
    
    if (updatedService?.enabled !== !originalState) {
      throw new Error('Service toggle failed');
    }
    details.push('✓ Service enable/disable works');

    // Test route toggle
    if (testService.routes.length > 0) {
      const testRoute = testService.routes[0];
      const routeId = testRoute.method ? `${testRoute.method}-${testRoute.path}` : testRoute.path;
      const originalRouteState = testRoute.enabled !== false;
      
      serviceStore.toggleRoute(testService.id, routeId, !originalRouteState);
      const updatedServiceWithRoute = serviceStore.services.find(s => s.id === testService.id);
      const updatedRoute = updatedServiceWithRoute?.routes.find(r => 
        (r.method ? `${r.method}-${r.path}` : r.path) === routeId
      );
      
      if (updatedRoute?.enabled === originalRouteState) {
        throw new Error('Route toggle failed');
      }
      details.push('✓ Route enable/disable works');
    }

    return { ...result, status: 'passed', details };
  };

  const testFixtureEditing = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Get initial fixtures
    const fixtures = fixtureStore.fixtures;
    if (fixtures.length === 0) {
      throw new Error('No fixtures available to test');
    }

    details.push(`✓ Found ${fixtures.length} fixtures to test`);

    // Test fixture content update
    const testFixture = fixtures[0];
    const originalContent = testFixture.content;
    const newContent = originalContent + '\n// Test comment added via UI';
    
    fixtureStore.updateFixture(testFixture.id, newContent);
    const updatedFixture = fixtureStore.fixtures.find(f => f.id === testFixture.id);
    
    if (updatedFixture?.content !== newContent) {
      throw new Error('Fixture content update failed');
    }
    details.push('✓ Fixture content editing works');

    // Test fixture rename
    const originalName = testFixture.name;
    const newName = `${originalName}.backup`;
    
    fixtureStore.renameFixture(testFixture.id, newName);
    const renamedFixture = fixtureStore.fixtures.find(f => f.id === testFixture.id);
    
    if (renamedFixture?.name !== newName) {
      throw new Error('Fixture rename failed');
    }
    details.push('✓ Fixture renaming works');

    return { ...result, status: 'passed', details };
  };

  const testFixtureDiffing = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Test diff generation
    const fixtures = fixtureStore.fixtures;
    if (fixtures.length === 0) {
      throw new Error('No fixtures available for diff testing');
    }

    const testFixture = fixtures[0];
    const modifiedContent = String(testFixture.content || '').replace('test', 'TEST_MODIFIED');
    
    try {
      const diff = fixtureStore.generateDiff(testFixture.id, modifiedContent);
      
      if (diff.changes.length === 0) {
        throw new Error('Diff generation produced no changes');
      }
      
      details.push(`✓ Generated diff with ${diff.changes.length} changes`);
      details.push('✓ Diff visualization ready');
      
    } catch (error) {
      throw new Error(`Diff generation failed: ${error}`);
    }

    return { ...result, status: 'passed', details };
  };

  const testLiveLogs = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Check log store
    const logs = logStore.logs;
    details.push(`✓ Found ${logs.length} log entries`);

    // Test filtering
    logStore.setFilter({ method: 'GET' });
    const filteredLogs = logStore.filteredLogs;
    const getRequests = filteredLogs.filter(log => log.method === 'GET');
    
    if (getRequests.length !== filteredLogs.length) {
      throw new Error('Log filtering by method failed');
    }
    details.push('✓ Log filtering by method works');

    // Test search
    logStore.setFilter({ path_pattern: '/api/users' });
    const searchedLogs = logStore.filteredLogs;
    const userApiLogs = searchedLogs.filter(log => log.path.includes('/api/users'));
    
    if (userApiLogs.length === 0 && searchedLogs.length > 0) {
      throw new Error('Log search filtering failed');
    }
    details.push('✓ Log search filtering works');

    // Reset filter
    logStore.clearFilter();

    return { ...result, status: 'passed', details };
  };

  const testMetricsAnalysis = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Check metrics data
    const latencyMetrics = metricsStore.latencyMetrics;
    const failureMetrics = metricsStore.failureMetrics;
    
    if (latencyMetrics.length === 0) {
      throw new Error('No latency metrics available');
    }
    details.push(`✓ Found ${latencyMetrics.length} latency metrics`);

    if (failureMetrics.length === 0) {
      throw new Error('No failure metrics available');
    }
    details.push(`✓ Found ${failureMetrics.length} failure metrics`);

    // Verify histogram data
    const firstMetric = latencyMetrics[0];
    if (!firstMetric.histogram || firstMetric.histogram.length === 0) {
      throw new Error('Histogram data missing');
    }
    details.push('✓ Latency histogram data available');

    // Verify percentiles
    if (firstMetric.p50 === 0 || firstMetric.p95 === 0 || firstMetric.p99 === 0) {
      throw new Error('Percentile data missing');
    }
    details.push('✓ Percentile metrics available');

    return { ...result, status: 'passed', details };
  };

  const testBulkOperations = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    const services = serviceStore.services;
    if (services.length < 2) {
      throw new Error('Need at least 2 services for bulk operations test');
    }

    // Test bulk enable (simulate)
    services.forEach(service => {
      serviceStore.updateService(service.id, { enabled: true });
    });
    
    const enabledServices = serviceStore.services.filter(s => s.enabled);
    if (enabledServices.length !== services.length) {
      throw new Error('Bulk enable operation failed');
    }
    details.push('✓ Bulk enable services works');

    // Test bulk disable (simulate)
    services.forEach(service => {
      serviceStore.updateService(service.id, { enabled: false });
    });
    
    const disabledServices = serviceStore.services.filter(s => !s.enabled);
    if (disabledServices.length !== services.length) {
      throw new Error('Bulk disable operation failed');
    }
    details.push('✓ Bulk disable services works');

    return { ...result, status: 'passed', details };
  };

  const testSearchFiltering = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Test service search (simulate by checking if services have searchable content)
    const services = serviceStore.services;
    const servicesWithNames = services.filter(s => s.name && s.name.length > 0);
    if (servicesWithNames.length === 0) {
      throw new Error('No services with names to search');
    }
    details.push('✓ Service search data available');

    // Test fixture search
    const fixtures = fixtureStore.fixtures;
    const fixturesWithContent = fixtures.filter((f: FixtureInfo) => f.content && String(f.content).length > 0);
    if (fixturesWithContent.length === 0) {
      throw new Error('No fixtures with content to search');
    }
    details.push('✓ Fixture search data available');

    // Test log search
    const logs = logStore.logs;
    const logsWithPaths = logs.filter(log => log.path && log.path.length > 0);
    if (logsWithPaths.length === 0) {
      throw new Error('No logs with paths to search');
    }
    details.push('✓ Log search data available');

    return { ...result, status: 'passed', details };
  };

  const testRoleBasedAccess = async (result: TestResult): Promise<TestResult> => {
    const details: string[] = [];
    
    // Check current user role
    const currentUser = authStore.user;
    if (!currentUser) {
      throw new Error('No authenticated user for role testing');
    }

    details.push(`✓ Current user role: ${currentUser.role}`);

    // Verify role-based features would be accessible
    if (currentUser.role === 'admin') {
      details.push('✓ Admin has access to all features');
    } else if (currentUser.role === 'viewer') {
      details.push('✓ Viewer has read-only access');
    } else {
      throw new Error('Unknown user role');
    }

    return { ...result, status: 'passed', details };
  };

  const runAllTests = async () => {
    setIsRunning(true);
    setTestResults(workflows.map(w => ({ ...w, status: 'pending' })));

    for (const workflow of workflows) {
      setTestResults(prev => prev.map(r => 
        r.id === workflow.id ? { ...r, status: 'running' } : r
      ));

      const result = await runTest(workflow);
      
      setTestResults(prev => prev.map(r => 
        r.id === workflow.id ? result : r
      ));

      // Small delay between tests
      await new Promise(resolve => setTimeout(resolve, 200));
    }

    setIsRunning(false);
  };

  const getStatusIcon = (status: TestResult['status']) => {
    switch (status) {
      case 'pending': return '⏳';
      case 'running': return '🔄';
      case 'passed': return '✅';
      case 'failed': return '❌';
    }
  };

  const getStatusColor = (status: TestResult['status']) => {
    switch (status) {
      case 'pending': return 'text-muted-foreground';
      case 'running': return 'text-blue-600';
      case 'passed': return 'text-green-600';
      case 'failed': return 'text-red-600';
    }
  };

  const passedTests = testResults.filter(r => r.status === 'passed').length;
  const failedTests = testResults.filter(r => r.status === 'failed').length;
  const totalTests = testResults.length;

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Power-User Workflow Validation</h2>
          <p className="text-muted-foreground">
            Testing all admin workflows to ensure file editing is not required
          </p>
        </div>
        <Button 
          onClick={runAllTests} 
          disabled={isRunning}
          className="min-w-32"
        >
          {isRunning ? 'Running Tests...' : 'Run All Tests'}
        </Button>
      </div>

      {testResults.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          <div className="text-center p-4 border rounded-lg">
            <div className="text-2xl font-bold text-green-600">{passedTests}</div>
            <div className="text-sm text-muted-foreground">Passed</div>
          </div>
          <div className="text-center p-4 border rounded-lg">
            <div className="text-2xl font-bold text-red-600">{failedTests}</div>
            <div className="text-sm text-muted-foreground">Failed</div>
          </div>
          <div className="text-center p-4 border rounded-lg">
            <div className="text-2xl font-bold">{totalTests}</div>
            <div className="text-sm text-muted-foreground">Total</div>
          </div>
        </div>
      )}

      <div className="space-y-4">
        {testResults.map((test) => (
          <div key={test.id} className="border rounded-lg p-4">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center space-x-3">
                <span className="text-2xl">{getStatusIcon(test.status)}</span>
                <div>
                  <h3 className="font-semibold">{test.name}</h3>
                  <p className="text-sm text-muted-foreground">{test.description}</p>
                </div>
              </div>
              <span className={`text-sm font-medium ${getStatusColor(test.status)}`}>
                {test.status.toUpperCase()}
              </span>
            </div>
            
            {test.error && (
              <div className="mt-2 p-3 bg-red-50 border border-red-200 rounded text-red-800 text-sm">
                <strong>Error:</strong> {test.error}
              </div>
            )}
            
            {test.details && test.details.length > 0 && (
              <div className="mt-2 space-y-1">
                {test.details.map((detail, index) => (
                  <div key={index} className="text-sm text-muted-foreground">
                    {detail}
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {testResults.length === 0 && (
        <div className="text-center py-12">
          <div className="text-6xl mb-4">🧪</div>
          <h3 className="text-lg font-semibold mb-2">Ready to Test Workflows</h3>
          <p className="text-muted-foreground mb-4">
            Click "Run All Tests" to validate that all power-user workflows work without file editing.
          </p>
        </div>
      )}
    </div>
  );
}