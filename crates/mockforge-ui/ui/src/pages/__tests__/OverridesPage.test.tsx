import type { ReactNode } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OverridesPage } from '../OverridesPage';
import { overridesApi, type OverrideRule } from '../../services/api/overrides';

vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => false }));
vi.mock('../../services/api/overrides', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../services/api/overrides')>();
  return {
    ...actual,
    overridesApi: { listHostedMocks: vi.fn(), list: vi.fn(), save: vi.fn() },
  };
});
vi.mock('sonner', () => ({ toast: { success: vi.fn(), warning: vi.fn(), error: vi.fn() } }));

const vip: OverrideRule = {
  name: 'VIP tier',
  enabled: true,
  targets: ['operation:getUser'],
  patch: [{ op: 'replace', path: '/tier', value: 'gold' }],
  mode: 'replace',
  post_templating: false,
};

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <MemoryRouter>
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    </MemoryRouter>
  );
  return render(<OverridesPage />, { wrapper });
}

describe('OverridesPage', () => {
  beforeEach(() => {
    vi.mocked(overridesApi.list).mockResolvedValue([vip]);
    vi.mocked(overridesApi.save).mockImplementation(async (_id, rules) => ({ rules, runtime: 'applied' }));
  });

  it('lists the live rules and saves an edited set as a whole', async () => {
    renderPage();
    expect(await screen.findByText('VIP tier')).toBeInTheDocument();
    expect(screen.getByText('operation:getUser')).toBeInTheDocument();

    const save = screen.getByRole('button', { name: 'Save changes' });
    expect(save).toBeDisabled();

    fireEvent.click(screen.getByRole('switch', { name: 'Disable VIP tier' }));
    expect(save).toBeEnabled();
    fireEvent.click(save);

    await waitFor(() =>
      expect(overridesApi.save).toHaveBeenCalledWith(null, [{ ...vip, enabled: false }]),
    );
    await waitFor(() => expect(save).toBeDisabled());
  });

  it('adds a rule through the editor and blocks invalid input', async () => {
    vi.mocked(overridesApi.list).mockResolvedValue([]);
    renderPage();
    fireEvent.click(await screen.findByRole('button', { name: /Add rule/ }));

    fireEvent.change(screen.getByPlaceholderText(/operation:getUser/), { target: { value: 'users' } });
    fireEvent.click(screen.getByRole('button', { name: 'Done' }));
    expect(await screen.findByText(/must start with operation:/)).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText(/operation:getUser/), { target: { value: 'tag:Users' } });
    fireEvent.change(screen.getByLabelText('Patch 1 path'), { target: { value: '/tier' } });
    fireEvent.change(screen.getByLabelText('Patch 1 value'), { target: { value: '"gold"' } });
    fireEvent.click(screen.getByRole('button', { name: 'Done' }));

    expect(await screen.findByText('tag:Users')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(overridesApi.save).toHaveBeenCalledWith(null, [
        expect.objectContaining({
          targets: ['tag:Users'],
          patch: [{ op: 'replace', path: '/tier', value: 'gold' }],
        }),
      ]),
    );
  });
});
