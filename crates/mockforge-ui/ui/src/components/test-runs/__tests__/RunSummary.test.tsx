import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RunSummary } from '../RunSummary';

describe('Run summary presentation', () => {
  it('shows structured benchmark results for a matching executor phase', () => {
    render(<RunSummary run={{ kind: 'bench', summary: { executor_phase: 'real_bench', total_requests: 120 } }} />);
    expect(screen.getByText('Bench results')).toBeVisible();
    expect(screen.getByText(/120 total/)).toBeVisible();
  });
  it('keeps an unexpected executor phase in the generic summary', () => {
    render(<RunSummary run={{ kind: 'bench', summary: { executor_phase: 'future_executor' } }} />);
    expect(screen.getByText('Run summary')).toBeVisible();
    expect(screen.queryByText('Bench results')).not.toBeInTheDocument();
  });
  it.each(['constructor', 'future_kind'])('renders unsupported kind %s without interpreting it as a component', kind => {
    render(<RunSummary run={{ kind, summary: { executor_phase: 'real_bench' } }} />);
    expect(screen.getByText('Run summary')).toBeVisible();
  });
  it('shows a not-implemented run as such instead of a results widget', () => {
    render(
      <RunSummary
        run={{
          kind: 'chaos_campaign',
          summary: {
            executor_phase: 'unsupported',
            status_reason: 'not_implemented',
            error: "'snapshot_restore' runs are not implemented by the cloud runner",
            work_count: 0,
          },
        }}
      />,
    );
    expect(screen.getByText('Not implemented')).toBeVisible();
    expect(screen.getByText("'snapshot_restore' runs are not implemented by the cloud runner")).toBeVisible();
  });
  it('shows a zero-work run as no work executed', () => {
    render(
      <RunSummary
        run={{ kind: 'bench', summary: { executor_phase: 'real_bench', status_reason: 'no_work_executed' } }}
      />,
    );
    expect(screen.getByText('No work executed')).toBeVisible();
    expect(screen.queryByText('Bench results')).not.toBeInTheDocument();
  });
  it('names other status reasons', () => {
    render(<RunSummary run={{ kind: 'contract_diff', summary: { status_reason: 'spec_http_error' } }} />);
    expect(screen.getByText('Run did not execute (spec_http_error)')).toBeVisible();
  });
});
