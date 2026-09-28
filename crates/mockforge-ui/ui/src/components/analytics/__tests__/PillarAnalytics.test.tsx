import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { PillarUsageMetrics } from '@/hooks/usePillarAnalytics';

vi.mock('react-chartjs-2', () => ({
  Doughnut: () => <div data-testid="doughnut" />,
}));

const dashboardProps = vi.fn();
vi.mock('@/components/analytics/PillarAnalyticsDashboard', () => ({
  PillarAnalyticsDashboard: (props: { workspaceId?: string; orgId?: string }) => {
    dashboardProps(props);
    return <div data-testid="dashboard">{`${props.workspaceId ?? '-'}|${props.orgId ?? '-'}`}</div>;
  },
}));

let cloudOrgId: string | null = null;
vi.mock('@/hooks/useCloudOrgId', () => ({ useCloudOrgId: () => cloudOrgId }));

import { PillarUsageChart } from '../PillarUsageChart';
import { PillarAnalyticsPage } from '@/pages/PillarAnalyticsPage';
import { useWorkspaceStore } from '@/stores/useWorkspaceStore';

const emptyMetrics: PillarUsageMetrics = {
  time_range: '7d',
  reality: null,
  contracts: null,
  devx: null,
  cloud: null,
  ai: null,
};

describe('PillarUsageChart', () => {
  it('shows loading only while loading', () => {
    render(<PillarUsageChart data={undefined} isLoading />);
    expect(screen.getByText('Loading chart data...')).toBeInTheDocument();
  });

  it('shows an empty state when there is no data and nothing is loading', () => {
    render(<PillarUsageChart data={undefined} isLoading={false} />);
    expect(screen.queryByText('Loading chart data...')).not.toBeInTheDocument();
    expect(screen.getByTestId('pillar-usage-chart-empty')).toBeInTheDocument();
  });

  it('shows an empty state when every pillar is null (analytics DB not configured)', () => {
    render(<PillarUsageChart data={emptyMetrics} />);
    expect(screen.getByTestId('pillar-usage-chart-empty')).toBeInTheDocument();
  });

  it('renders the chart when there is usage', () => {
    render(
      <PillarUsageChart
        data={{
          ...emptyMetrics,
          ai: {
            ai_generated_mocks: 3,
            ai_contract_diffs: 0,
            voice_commands: 0,
            llm_assisted_operations: 0,
          },
        }}
      />,
    );
    expect(screen.getByTestId('doughnut')).toBeInTheDocument();
  });
});

describe('PillarAnalyticsPage', () => {
  it('picks up a workspace that loads after mount', () => {
    cloudOrgId = null;
    useWorkspaceStore.setState({ activeWorkspace: null });
    render(<PillarAnalyticsPage />);
    expect(screen.getByText('Select Workspace')).toBeInTheDocument();

    act(() => {
      useWorkspaceStore.setState({
        activeWorkspace: { id: 'ws-1', name: 'Main' } as never,
      });
    });

    expect(screen.getByTestId('dashboard')).toHaveTextContent('ws-1|-');
  });

  it('falls back to org-wide metrics in cloud mode when no workspace is selected', () => {
    cloudOrgId = 'org-9';
    useWorkspaceStore.setState({ activeWorkspace: null });
    render(<PillarAnalyticsPage />);
    expect(screen.getByTestId('dashboard')).toHaveTextContent('-|org-9');
  });
});
