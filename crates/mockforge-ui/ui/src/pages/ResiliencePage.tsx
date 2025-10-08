import React, { useEffect, useState } from 'react';
import { useWebSocket } from '../hooks/useWebSocket';

interface CircuitBreakerState {
  endpoint: string;
  state: string;
  stats: {
    total_requests: number;
    successful_requests: number;
    failed_requests: number;
    rejected_requests: number;
    consecutive_failures: number;
    consecutive_successes: number;
    success_rate: number;
    failure_rate: number;
  };
}

interface BulkheadState {
  service: string;
  stats: {
    active_requests: number;
    queued_requests: number;
    total_requests: number;
    rejected_requests: number;
    timeout_requests: number;
    utilization_percent: number;
  };
}

interface DashboardSummary {
  circuit_breakers: {
    total: number;
    open: number;
    half_open: number;
    closed: number;
  };
  bulkheads: {
    total: number;
    active_requests: number;
    queued_requests: number;
  };
}

export const ResiliencePage: React.FC = () => {
  const [circuitBreakers, setCircuitBreakers] = useState<CircuitBreakerState[]>([]);
  const [bulkheads, setBulkheads] = useState<BulkheadState[]>([]);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [selectedTab, setSelectedTab] = useState<'circuit-breakers' | 'bulkheads'>('circuit-breakers');
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchCircuitBreakers = async () => {
    try {
      const response = await fetch('/api/resilience/circuit-breakers');
      const data = await response.json();
      setCircuitBreakers(data);
    } catch (error) {
      console.error('Failed to fetch circuit breakers:', error);
    }
  };

  const fetchBulkheads = async () => {
    try {
      const response = await fetch('/api/resilience/bulkheads');
      const data = await response.json();
      setBulkheads(data);
    } catch (error) {
      console.error('Failed to fetch bulkheads:', error);
    }
  };

  const fetchSummary = async () => {
    try {
      const response = await fetch('/api/resilience/dashboard/summary');
      const data = await response.json();
      setSummary(data);
    } catch (error) {
      console.error('Failed to fetch summary:', error);
    }
  };

  const resetCircuitBreaker = async (endpoint: string) => {
    try {
      await fetch(`/api/resilience/circuit-breakers/${encodeURIComponent(endpoint)}/reset`, {
        method: 'POST',
      });
      fetchCircuitBreakers();
    } catch (error) {
      console.error('Failed to reset circuit breaker:', error);
    }
  };

  const resetBulkhead = async (service: string) => {
    try {
      await fetch(`/api/resilience/bulkheads/${encodeURIComponent(service)}/reset`, {
        method: 'POST',
      });
      fetchBulkheads();
    } catch (error) {
      console.error('Failed to reset bulkhead:', error);
    }
  };

  useEffect(() => {
    fetchCircuitBreakers();
    fetchBulkheads();
    fetchSummary();

    const interval = setInterval(() => {
      if (autoRefresh) {
        fetchCircuitBreakers();
        fetchBulkheads();
        fetchSummary();
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [autoRefresh]);

  const getStateColor = (state: string): string => {
    switch (state) {
      case 'Closed':
        return 'bg-green-500';
      case 'Open':
        return 'bg-red-500';
      case 'HalfOpen':
        return 'bg-yellow-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getUtilizationColor = (percent: number): string => {
    if (percent < 50) return 'bg-green-500';
    if (percent < 80) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold text-gray-900">Resilience Dashboard</h1>
        <div className="flex items-center space-x-4">
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="form-checkbox h-5 w-5 text-blue-600"
            />
            <span className="text-sm text-gray-700">Auto-refresh (3s)</span>
          </label>
          <button
            onClick={() => {
              fetchCircuitBreakers();
              fetchBulkheads();
              fetchSummary();
            }}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Refresh Now
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      {summary && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Circuit Breakers</h2>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-600">Total:</span>
                <span className="font-semibold">{summary.circuit_breakers.total}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Closed:</span>
                <span className="font-semibold text-green-600">{summary.circuit_breakers.closed}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Half-Open:</span>
                <span className="font-semibold text-yellow-600">{summary.circuit_breakers.half_open}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Open:</span>
                <span className="font-semibold text-red-600">{summary.circuit_breakers.open}</span>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Bulkheads</h2>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-600">Total Services:</span>
                <span className="font-semibold">{summary.bulkheads.total}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Active Requests:</span>
                <span className="font-semibold text-blue-600">{summary.bulkheads.active_requests}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Queued Requests:</span>
                <span className="font-semibold text-yellow-600">{summary.bulkheads.queued_requests}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          <button
            onClick={() => setSelectedTab('circuit-breakers')}
            className={`${
              selectedTab === 'circuit-breakers'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm`}
          >
            Circuit Breakers
          </button>
          <button
            onClick={() => setSelectedTab('bulkheads')}
            className={`${
              selectedTab === 'bulkheads'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm`}
          >
            Bulkheads
          </button>
        </nav>
      </div>

      {/* Circuit Breakers Tab */}
      {selectedTab === 'circuit-breakers' && (
        <div className="space-y-4">
          {circuitBreakers.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              No circuit breakers configured
            </div>
          ) : (
            circuitBreakers.map((cb) => (
              <div key={cb.endpoint} className="bg-white rounded-lg shadow p-6">
                <div className="flex justify-between items-start mb-4">
                  <div className="flex items-center space-x-3">
                    <div className={`w-3 h-3 rounded-full ${getStateColor(cb.state)}`} />
                    <h3 className="text-lg font-semibold text-gray-900">{cb.endpoint}</h3>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      cb.state === 'Closed' ? 'bg-green-100 text-green-800' :
                      cb.state === 'Open' ? 'bg-red-100 text-red-800' :
                      'bg-yellow-100 text-yellow-800'
                    }`}>
                      {cb.state}
                    </span>
                  </div>
                  <button
                    onClick={() => resetCircuitBreaker(cb.endpoint)}
                    className="px-3 py-1 bg-gray-200 text-gray-700 rounded hover:bg-gray-300 text-sm"
                  >
                    Reset
                  </button>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <div className="text-sm text-gray-600">Total Requests</div>
                    <div className="text-2xl font-semibold">{cb.stats.total_requests}</div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Success Rate</div>
                    <div className="text-2xl font-semibold text-green-600">
                      {cb.stats.success_rate.toFixed(1)}%
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Failure Rate</div>
                    <div className="text-2xl font-semibold text-red-600">
                      {cb.stats.failure_rate.toFixed(1)}%
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Rejected</div>
                    <div className="text-2xl font-semibold text-orange-600">
                      {cb.stats.rejected_requests}
                    </div>
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-600">Successful:</span>
                    <span className="font-medium">{cb.stats.successful_requests}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Failed:</span>
                    <span className="font-medium">{cb.stats.failed_requests}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Consecutive Failures:</span>
                    <span className="font-medium">{cb.stats.consecutive_failures}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Consecutive Successes:</span>
                    <span className="font-medium">{cb.stats.consecutive_successes}</span>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {/* Bulkheads Tab */}
      {selectedTab === 'bulkheads' && (
        <div className="space-y-4">
          {bulkheads.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              No bulkheads configured
            </div>
          ) : (
            bulkheads.map((bh) => (
              <div key={bh.service} className="bg-white rounded-lg shadow p-6">
                <div className="flex justify-between items-start mb-4">
                  <h3 className="text-lg font-semibold text-gray-900">{bh.service}</h3>
                  <button
                    onClick={() => resetBulkhead(bh.service)}
                    className="px-3 py-1 bg-gray-200 text-gray-700 rounded hover:bg-gray-300 text-sm"
                  >
                    Reset Stats
                  </button>
                </div>

                {/* Utilization Bar */}
                <div className="mb-4">
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-gray-600">Utilization</span>
                    <span className="font-medium">{bh.stats.utilization_percent.toFixed(1)}%</span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${getUtilizationColor(bh.stats.utilization_percent)}`}
                      style={{ width: `${Math.min(bh.stats.utilization_percent, 100)}%` }}
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                  <div>
                    <div className="text-sm text-gray-600">Active</div>
                    <div className="text-2xl font-semibold text-blue-600">
                      {bh.stats.active_requests}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Queued</div>
                    <div className="text-2xl font-semibold text-yellow-600">
                      {bh.stats.queued_requests}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Total</div>
                    <div className="text-2xl font-semibold">{bh.stats.total_requests}</div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Rejected</div>
                    <div className="text-2xl font-semibold text-red-600">
                      {bh.stats.rejected_requests}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-600">Timeouts</div>
                    <div className="text-2xl font-semibold text-orange-600">
                      {bh.stats.timeout_requests}
                    </div>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
