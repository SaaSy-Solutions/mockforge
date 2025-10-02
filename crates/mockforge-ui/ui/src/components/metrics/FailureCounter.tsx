import React from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, BarChart, Bar, XAxis, YAxis, CartesianGrid } from 'recharts';
import type { FailureMetrics } from '../../types';

interface FailureCounterProps {
  metrics: FailureMetrics[];
  selectedService?: string;
  onServiceChange: (service: string) => void;
}

export function FailureCounter({ metrics, selectedService, onServiceChange }: FailureCounterProps) {
  const selectedMetric = selectedService ? metrics.find(m => m.service === selectedService) : metrics[0];

  const getSuccessFailureData = () => {
    if (!selectedMetric) return [];
    
    return [
      {
        name: 'Success',
        value: selectedMetric.success_count,
        color: '#10b981',
      },
      {
        name: 'Failure',
        value: selectedMetric.failure_count,
        color: '#ef4444',
      },
    ];
  };

  const getStatusCodeData = () => {
    if (!selectedMetric) return [];
    
    return Object.entries(selectedMetric.status_codes || {}).map(([code, count]) => ({
      status_code: code,
      count,
      color: getStatusCodeColor(parseInt(code)),
    }));
  };

  const getStatusCodeColor = (code: number) => {
    if (code >= 200 && code < 300) return '#10b981'; // green
    if (code >= 300 && code < 400) return '#3b82f6'; // blue
    if (code >= 400 && code < 500) return '#f59e0b'; // yellow
    if (code >= 500) return '#ef4444'; // red
    return '#6b7280'; // gray
  };

  const formatErrorRate = (rate: number) => {
    return `${(rate * 100).toFixed(2)}%`;
  };

  const successFailureData = getSuccessFailureData();
  const statusCodeData = getStatusCodeData();

  const renderCustomTooltip = (data: { active?: boolean; payload?: Array<{ payload: { name: string; value: number } }> }) => {
    if (data.active && data.payload && data.payload[0]) {
      const payload = data.payload[0].payload;
      return (
        <div className="bg-white p-3 border border-gray-200 rounded shadow">
          <p className="font-medium">{payload.name}</p>
          <p className="text-sm text-gray-600">
            {payload.value} requests ({((payload.value / (selectedMetric?.total_requests || 1)) * 100).toFixed(1)}%)
          </p>
        </div>
      );
    }
    return null;
  };

  return (
    <div className="space-y-6">
      {/* Service Selector and Overview */}
      <div className="rounded-lg border bg-card p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold">Failure Analysis</h3>
          <select
            value={selectedService || ''}
            onChange={(e) => onServiceChange(e.target.value)}
            className="px-3 py-1 border border-input rounded text-sm bg-background"
          >
            <option value="">All Services</option>
            {metrics.map(metric => (
              <option key={metric.service} value={metric.service}>
                {metric.service}
              </option>
            ))}
          </select>
        </div>

        {selectedMetric && (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
            <div className="space-y-1">
              <div className="text-2xl font-bold">{(selectedMetric.total_requests || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Total Requests</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-green-600">{(selectedMetric.success_count || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Successful</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-red-600">{(selectedMetric.failure_count || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Failed</div>
            </div>
            <div className="space-y-1">
              <div className={`text-2xl font-bold ${(selectedMetric.error_rate || 0) < 0.05 ? 'text-green-600' : (selectedMetric.error_rate || 0) < 0.1 ? 'text-yellow-600' : 'text-red-600'}`}>
                {formatErrorRate(selectedMetric.error_rate || 0)}
              </div>
              <div className="text-xs text-muted-foreground">Error Rate</div>
            </div>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Success/Failure Pie Chart */}
        <div className="rounded-lg border bg-card p-6">
          <h4 className="font-semibold mb-4">Success vs Failure</h4>
          
          {successFailureData.length > 0 && selectedMetric ? (
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={successFailureData}
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                  >
                    {successFailureData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip content={renderCustomTooltip} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="flex items-center justify-center h-64 text-center">
              <div className="space-y-2">
                <div className="text-4xl">📈</div>
                <div className="text-muted-foreground">No failure data available</div>
              </div>
            </div>
          )}

          {selectedMetric && (
            <div className="flex justify-center space-x-6 mt-4">
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-green-500 rounded"></div>
                <span className="text-sm">Success ({(((selectedMetric.success_count || 0) / (selectedMetric.total_requests || 1)) * 100).toFixed(1)}%)</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-red-500 rounded"></div>
                <span className="text-sm">Failure ({(((selectedMetric.failure_count || 0) / (selectedMetric.total_requests || 1)) * 100).toFixed(1)}%)</span>
              </div>
            </div>
          )}
        </div>

        {/* Status Code Distribution */}
        <div className="rounded-lg border bg-card p-6">
          <h4 className="font-semibold mb-4">Status Code Distribution</h4>
          
          {statusCodeData.length > 0 ? (
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={statusCodeData} margin={{ top: 20, right: 30, left: 20, bottom: 5 }}>
                  <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
                  <XAxis dataKey="status_code" fontSize={12} />
                  <YAxis fontSize={12} />
                  <Tooltip 
                    formatter={(value: number) => [`${value} requests`, 'Count']}
                    labelStyle={{ color: '#000' }}
                    contentStyle={{
                      backgroundColor: 'white',
                      border: '1px solid #e2e8f0',
                      borderRadius: '6px',
                    }}
                  />
                  <Bar dataKey="count" name="count">
                    {statusCodeData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="flex items-center justify-center h-64 text-center">
              <div className="space-y-2">
                <div className="text-4xl">📊</div>
                <div className="text-muted-foreground">No status code data available</div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Error Rate Trend (placeholder for future implementation) */}
      <div className="rounded-lg border bg-card p-6">
        <h4 className="font-semibold mb-4">Error Rate Trend</h4>
        <div className="flex items-center justify-center h-32 text-center">
          <div className="space-y-2">
            <div className="text-4xl">📈</div>
            <div className="text-muted-foreground">
              Error rate trends will be available with time-series data
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}