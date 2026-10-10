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
});
