import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vitest';
import { useFixtures, useDeleteFixture } from '../useFixturesApi';
import { fixturesApi } from '../../../services/api';
vi.mock('../../../services/api', () => ({ fixturesApi: { getFixtures: vi.fn(), deleteFixture: vi.fn() }, apiService: {} }));
describe('fixture list cache', () => {
  it('refreshes the visible list immediately after deletion', async () => {
    let rows = [{ id: 'fixture', name: 'QA fixture' }];
    vi.mocked(fixturesApi.getFixtures).mockImplementation(async () => rows as never);
    vi.mocked(fixturesApi.deleteFixture).mockImplementation(async () => { rows = []; });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => ({ list: useFixtures(), remove: useDeleteFixture() }), {
      wrapper: ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>,
    });
    await waitFor(() => expect(result.current.list.data).toHaveLength(1));
    await act(async () => { await result.current.remove.mutateAsync('fixture'); });
    await waitFor(() => expect(result.current.list.data).toHaveLength(0));
  });
});
