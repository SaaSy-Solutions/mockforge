import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { TestTube, Play, CheckCircle, XCircle, Clock, AlertTriangle, RefreshCw } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Section
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { dashboardApi, smokeTestsApi } from '../services/api';
import type { SmokeTestResult } from '../types';

interface TestResult {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'passed' | 'failed';
  duration: number;
  error?: string;
  timestamp: string;
}

interface TestSuite {
  id: string;
  name: string;
  description: string;
  tests: TestResult[];
  status: 'idle' | 'running' | 'completed' | 'failed';
  totalTests: number;
  passedTests: number;
  failedTests: number;
  duration: number;
}

export function TestingPage() {
  const [selectedSuite] = useState<string | null>(null);
  const [isRunningTests, setIsRunningTests] = useState(false);
  const [testResults, setTestResults] = useState<TestSuite[]>([
    {
      id: 'smoke-tests',
      name: 'Smoke Tests',
      description: 'Basic functionality and endpoint availability tests',
      status: 'idle',
      totalTests: 0,
      passedTests: 0,
      failedTests: 0,
      duration: 0,
      tests: []
    },
    {
      id: 'health-check',
      name: 'Health Check',
      description: 'System health and service availability check',
      status: 'idle',
      totalTests: 1,
      passedTests: 0,
      failedTests: 0,
      duration: 0,
      tests: [
        { id: 'health', name: 'Health Endpoint', status: 'pending', duration: 0, timestamp: new Date().toISOString() }
      ]
    },
    {
      id: 'integration-tests',
      name: 'Integration Tests',
      description: 'Custom integration tests for API endpoints',
      status: 'idle',
      totalTests: 0,
      passedTests: 0,
      failedTests: 0,
      duration: 0,
      tests: []
    }
  ]);

  // Convert SmokeTestResult to TestResult
  const convertSmokeTestResult = (result: SmokeTestResult, index: number): TestResult => ({
    id: `smoke-${index}`,
    name: result.test_name,
    status: result.passed ? 'passed' : 'failed',
    duration: result.response_time_ms || 0,
    error: result.error_message,
    timestamp: new Date().toISOString()
  });

  const runSmokeTests = async () => {
    setIsRunningTests(true);

    // Update suite status to running
    setTestResults(prev => prev.map(s =>
      s.id === 'smoke-tests'
        ? { ...s, status: 'running' as const }
        : s
    ));

    try {
      const startTime = Date.now();
      const context = await smokeTestsApi.runSmokeTests();
      const endTime = Date.now();

      // Get the actual test results
      const smokeResults = await smokeTestsApi.getSmokeTests();
      const tests = smokeResults.map((result, index) => convertSmokeTestResult(result, index));

      setTestResults(prev => prev.map(s =>
        s.id === 'smoke-tests'
          ? {
              ...s,
              status: context.failed_tests > 0 ? 'failed' : 'completed',
              totalTests: context.total_tests,
              passedTests: context.passed_tests,
              failedTests: context.failed_tests,
              duration: endTime - startTime,
              tests
            }
          : s
      ));
    } catch (error) {
      setTestResults(prev => prev.map(s =>
        s.id === 'smoke-tests'
          ? {
              ...s,
              status: 'failed',
              failedTests: 1,
              tests: [{
                id: 'smoke-error',
                name: 'Smoke Tests Execution',
                status: 'failed',
                duration: 0,
                error: error instanceof Error ? error.message : 'Unknown error',
                timestamp: new Date().toISOString()
              }]
            }
          : s
      ));
    } finally {
      setIsRunningTests(false);
    }
  };

  const runHealthCheck = async () => {
    setIsRunningTests(true);

    // Update suite status to running
    setTestResults(prev => prev.map(s =>
      s.id === 'health-check'
        ? { ...s, status: 'running' as const, tests: s.tests.map(t => ({ ...t, status: 'running' as const })) }
        : s
    ));

    try {
      const startTime = Date.now();
      const health = await dashboardApi.getHealth();
      const endTime = Date.now();

      const passed = health.status === 'healthy' || health.status === 'ok';
      const issues = health.issues || [];

      setTestResults(prev => prev.map(s =>
        s.id === 'health-check'
          ? {
              ...s,
              status: passed ? 'completed' : 'failed',
              passedTests: passed ? 1 : 0,
              failedTests: passed ? 0 : 1,
              duration: endTime - startTime,
              tests: [{
                id: 'health',
                name: 'Health Endpoint',
                status: passed ? 'passed' : 'failed',
                duration: endTime - startTime,
                error: issues.length > 0 ? issues.join(', ') : undefined,
                timestamp: new Date().toISOString()
              }]
            }
          : s
      ));
    } catch (error) {
      setTestResults(prev => prev.map(s =>
        s.id === 'health-check'
          ? {
              ...s,
              status: 'failed',
              failedTests: 1,
              passedTests: 0,
              tests: [{
                id: 'health',
                name: 'Health Endpoint',
                status: 'failed',
                duration: 0,
                error: error instanceof Error ? error.message : 'Failed to fetch health status',
                timestamp: new Date().toISOString()
              }]
            }
          : s
      ));
    } finally {
      setIsRunningTests(false);
    }
  };

  const runTestSuite = async (suiteId: string) => {
    switch (suiteId) {
      case 'smoke-tests':
        await runSmokeTests();
        break;
      case 'health-check':
        await runHealthCheck();
        break;
      case 'integration-tests':
        // Placeholder for custom integration tests
        setIsRunningTests(true);
        setTestResults(prev => prev.map(s =>
          s.id === suiteId
            ? {
                ...s,
                status: 'completed',
                tests: [{
                  id: 'integration-placeholder',
                  name: 'Custom integration tests not configured',
                  status: 'pending',
                  duration: 0,
                  timestamp: new Date().toISOString()
                }]
              }
            : s
        ));
        setIsRunningTests(false);
        break;
    }
  };

  const runAllTests = async () => {
    await runHealthCheck();
    await runSmokeTests();
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'passed': return <CheckCircle className="h-4 w-4 text-green-600" />;
      case 'failed': return <XCircle className="h-4 w-4 text-red-600" />;
      case 'running': return <Clock className="h-4 w-4 text-blue-600 animate-spin" />;
      case 'pending': return <Clock className="h-4 w-4 text-gray-400" />;
      default: return <AlertTriangle className="h-4 w-4 text-yellow-600" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'passed': return 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400';
      case 'failed': return 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400';
      case 'running': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400';
      case 'pending': return 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400';
      default: return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400';
    }
  };

  const totalTests = testResults.reduce((acc, suite) => acc + suite.totalTests, 0);
  const totalPassed = testResults.reduce((acc, suite) => acc + suite.passedTests, 0);
  const totalFailed = testResults.reduce((acc, suite) => acc + suite.failedTests, 0);
  const totalDuration = testResults.reduce((acc, suite) => acc + suite.duration, 0);

  return (
    <div className="space-y-8">
      <PageHeader
        title="Testing Suite"
        subtitle="Run automated tests and validate MockForge functionality"
        action={
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setTestResults(prev => prev.map(suite => ({
                ...suite,
                status: 'idle',
                passedTests: 0,
                failedTests: 0,
                duration: 0,
                tests: suite.tests.map(test => ({
                  ...test,
                  status: 'pending' as const,
                  duration: 0,
                  error: undefined
                }))
              })))}
              disabled={isRunningTests}
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Reset
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={runAllTests}
              disabled={isRunningTests}
              className="flex items-center gap-2"
            >
              <Play className="h-4 w-4" />
              Run All Tests
            </Button>
          </div>
        }
      />

      {/* Test Statistics */}
      <Section title="Test Overview" subtitle="Summary of test execution results">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          <ModernCard>
            <div className="flex items-center gap-3">
              <div className="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
                <TestTube className="h-6 w-6" />
              </div>
              <div>
                <div className="text-2xl font-bold text-gray-900 dark:text-gray-100">{totalTests}</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">Total Tests</div>
              </div>
            </div>
          </ModernCard>

          <ModernCard>
            <div className="flex items-center gap-3">
              <div className="p-3 rounded-lg bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400">
                <CheckCircle className="h-6 w-6" />
              </div>
              <div>
                <div className="text-2xl font-bold text-green-600 dark:text-green-400">{totalPassed}</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">Passed</div>
              </div>
            </div>
          </ModernCard>

          <ModernCard>
            <div className="flex items-center gap-3">
              <div className="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400">
                <XCircle className="h-6 w-6" />
              </div>
              <div>
                <div className="text-2xl font-bold text-red-600 dark:text-red-400">{totalFailed}</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">Failed</div>
              </div>
            </div>
          </ModernCard>

          <ModernCard>
            <div className="flex items-center gap-3">
              <div className="p-3 rounded-lg bg-yellow-50 dark:bg-yellow-900/20 text-yellow-600 dark:text-yellow-400">
                <Clock className="h-6 w-6" />
              </div>
              <div>
                <div className="text-2xl font-bold text-gray-900 dark:text-gray-100">{(totalDuration / 1000).toFixed(1)}s</div>
                <div className="text-sm text-gray-600 dark:text-gray-400">Total Time</div>
              </div>
            </div>
          </ModernCard>
        </div>
      </Section>

      {/* Test Suites */}
      <Section title="Test Suites" subtitle="Organized collections of automated tests">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {testResults.map((suite) => (
            <ModernCard key={suite.id}>
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 flex items-center gap-2">
                    <TestTube className="h-5 w-5" />
                    {suite.name}
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                    {suite.description}
                  </p>
                </div>
                <ModernBadge
                  variant={
                    suite.status === 'completed' ? 'success' :
                    suite.status === 'failed' ? 'error' :
                    suite.status === 'running' ? 'info' : 'outline'
                  }
                >
                  {suite.status}
                </ModernBadge>
              </div>

              <div className="grid grid-cols-3 gap-4 mb-4">
                <div className="text-center">
                  <div className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {suite.totalTests}
                  </div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">Total</div>
                </div>
                <div className="text-center">
                  <div className="text-lg font-semibold text-green-600 dark:text-green-400">
                    {suite.passedTests}
                  </div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">Passed</div>
                </div>
                <div className="text-center">
                  <div className="text-lg font-semibold text-red-600 dark:text-red-400">
                    {suite.failedTests}
                  </div>
                  <div className="text-xs text-gray-600 dark:text-gray-400">Failed</div>
                </div>
              </div>

              <div className="space-y-2 mb-4">
                {suite.tests.slice(0, 5).map((test) => (
                  <div key={test.id} className="flex items-center justify-between py-2 px-3 rounded-lg bg-gray-50 dark:bg-gray-800/50">
                    <div className="flex items-center gap-2">
                      {getStatusIcon(test.status)}
                      <span className="text-sm text-gray-900 dark:text-gray-100">
                        {test.name}
                      </span>
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">
                      {test.duration > 0 ? `${test.duration.toFixed(0)}ms` : ''}
                    </div>
                  </div>
                ))}
                {suite.tests.length > 5 && (
                  <div className="text-center text-sm text-gray-500 dark:text-gray-400">
                    +{suite.tests.length - 5} more tests
                  </div>
                )}
              </div>

              <Button
                onClick={() => runTestSuite(suite.id)}
                disabled={isRunningTests || suite.status === 'running'}
                className="w-full flex items-center gap-2"
                variant={suite.status === 'running' ? 'outline' : 'default'}
              >
                {suite.status === 'running' ? (
                  <>
                    <Clock className="h-4 w-4 animate-spin" />
                    Running Tests...
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4" />
                    Run {suite.name}
                  </>
                )}
              </Button>
            </ModernCard>
          ))}
        </div>
      </Section>

      {/* Test Results Details */}
      {selectedSuite && (
        <Section title={`Test Results: ${testResults.find(s => s.id === selectedSuite)?.name}`} subtitle="Detailed test execution results">
          <ModernCard>
            <div className="space-y-4">
              {testResults.find(s => s.id === selectedSuite)?.tests.map((test) => (
                <div key={test.id} className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      {getStatusIcon(test.status)}
                      <h4 className="font-medium text-gray-900 dark:text-gray-100">
                        {test.name}
                      </h4>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(test.status)}`}>
                        {test.status}
                      </span>
                      {test.duration > 0 && (
                        <span className="text-xs text-gray-600 dark:text-gray-400">
                          {test.duration.toFixed(0)}ms
                        </span>
                      )}
                    </div>
                  </div>

                  {test.error && (
                    <div className="mt-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                      <div className="flex items-start gap-2">
                        <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400 mt-0.5 flex-shrink-0" />
                        <div className="text-sm text-red-800 dark:text-red-200">
                          {test.error}
                        </div>
                      </div>
                    </div>
                  )}

                  <div className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                    Executed at {new Date(test.timestamp).toLocaleString()}
                  </div>
                </div>
              ))}
            </div>
          </ModernCard>
        </Section>
      )}

      {/* Test Configuration */}
      <Section title="Test Configuration" subtitle="Configure test execution settings">
        <ModernCard>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Test Timeout (seconds)
              </label>
              <Input type="number" defaultValue="30" min="1" max="300" />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Parallel Execution
              </label>
              <select className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100">
                <option value="sequential">Sequential</option>
                <option value="parallel">Parallel</option>
                <option value="limited">Limited Parallel (4)</option>
              </select>
            </div>

            <div className="md:col-span-2">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Test Environment
              </label>
              <div className="flex gap-4">
                <label className="flex items-center">
                  <input type="radio" name="environment" value="development" defaultChecked className="mr-2" />
                  Development
                </label>
                <label className="flex items-center">
                  <input type="radio" name="environment" value="staging" className="mr-2" />
                  Staging
                </label>
                <label className="flex items-center">
                  <input type="radio" name="environment" value="production" className="mr-2" />
                  Production
                </label>
              </div>
            </div>
          </div>

          <div className="flex justify-end pt-6 border-t border-gray-200 dark:border-gray-700">
            <Button>Save Configuration</Button>
          </div>
        </ModernCard>
      </Section>
    </div>
  );
}
