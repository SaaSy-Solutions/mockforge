import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CloudAIQuotaBanner } from '../CloudAIQuotaBanner';

const query = vi.hoisted(() => ({ data: {} as Record<string, unknown> }));
vi.mock('../../../utils/cloudMode', () => ({ isCloudMode: () => true }));
vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: query.data, isLoading: false, isError: false }),
}));

describe('Cloud AI admission feedback', () => {
  beforeEach(() => {
    query.data = { provider: 'platform', call_allowed: false,
      tokens_used_this_period: 0, tokens_reserved_this_period: 0, tokens_limit: 5000 };
  });

  it('shows verification guidance without calling unused credits exhausted', () => {
    query.data.deny_reason = 'Verify the organization owner\'s email before using Free platform-funded resources';
    render(<CloudAIQuotaBanner />);
    expect(screen.getByText(/Verify the organization owner's email/)).toBeVisible();
    expect(screen.queryByText(/Monthly AI quota exhausted/)).not.toBeInTheDocument();
  });

  it('keeps outstanding-call guidance when an older server omits the reason', () => {
    query.data.tokens_reserved_this_period = 100;
    render(<CloudAIQuotaBanner />);
    expect(screen.getByText(/AI credits are reserved by outstanding calls/)).toBeVisible();
    expect(screen.getByText(/100 reserved/)).toBeVisible();
  });
});
