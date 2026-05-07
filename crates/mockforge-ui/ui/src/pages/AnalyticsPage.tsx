/**
 * Analytics Page - Uses the V2 Analytics Dashboard
 * This page provides comprehensive analytics visualization with real-time updates.
 *
 * In cloud mode, the underlying /api/v2/analytics/* surface is local-only and
 * stubbed empty (see LOCAL_ONLY_API_PREFIXES in apiClient.ts), so we render a
 * banner pointing cloud users to PillarAnalyticsPage and CloudTracesPage —
 * the closest equivalents at workspace / org scope. See issue #394.
 */

import React from 'react';
import { useNavigate } from 'react-router-dom';
import { AnalyticsDashboardV2 } from '@/components/analytics/AnalyticsDashboardV2';
import { isCloudMode } from '../utils/cloudMode';

export const AnalyticsPage: React.FC = () => {
  if (isCloudMode()) {
    return <CloudModeNotice />;
  }
  return <AnalyticsDashboardV2 />;
};

const CloudModeNotice: React.FC = () => {
  const navigate = useNavigate();
  return (
    <div className="p-6 max-w-3xl mx-auto">
      <div className="rounded-lg border border-blue-200 bg-blue-50 p-5 text-sm text-blue-800 dark:border-blue-900 dark:bg-blue-900/20 dark:text-blue-300">
        <h2 className="text-base font-semibold mb-2">
          Request-traffic analytics is a self-hosted feature
        </h2>
        <p className="mb-3">
          The per-server request-volume, p95/p99 latency, error-rate, and endpoint
          heatmap charts are part of the local MockForge runtime. They aren&apos;t
          aggregated at workspace scope in cloud mode.
        </p>
        <p className="mb-2">For analytics on hosted mocks, use:</p>
        <ul className="list-disc ml-5 space-y-1">
          <li>
            <button
              type="button"
              className="font-medium underline"
              onClick={() => navigate('/pillar-analytics')}
            >
              Analytics
            </button>{' '}
            &mdash; pillar-level usage stats (Reality / Chaos / Contracts / etc.)
            scoped to your org.
          </li>
          <li>
            <button
              type="button"
              className="font-medium underline"
              onClick={() => navigate('/cloud-traces')}
            >
              Cloud Traces
            </button>{' '}
            &mdash; cross-deployment OTLP trace search for individual requests.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default AnalyticsPage;
