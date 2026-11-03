import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { fetchJsonWithErrorBody } from '@/services/api/client';
import { ApiExplorerPage } from '../ApiExplorerPage';

vi.mock('@/services/api/client', () => ({ fetchJsonWithErrorBody: vi.fn() }));
vi.mock('@scalar/api-reference-react', () => ({
  ApiReferenceReact: ({ configuration }: { configuration: { content?: unknown; url?: string } }) => (
    <div data-testid="scalar" data-url={configuration.url ?? ''}>
      {JSON.stringify(configuration.content)}
    </div>
  ),
}));
vi.mock('@scalar/api-reference-react/style.css', () => ({}));

const deployment = {
  id: 'dep-1',
  name: 'Orders',
  deployment_url: 'https://orders.example.dev',
  status: 'active',
};

function headerOf(init: RequestInit | undefined, name: string): string | null {
  return new Headers(init?.headers).get(name);
}

describe('ApiExplorerPage', () => {
  beforeEach(() => {
    vi.mocked(fetchJsonWithErrorBody).mockReset();
    vi.stubGlobal('fetch', vi.fn());
  });

  it('sends the management token on the spec fetch and hands Scalar the spec', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockResolvedValue({
      token: 'mfm_tok',
      header: 'X-MockForge-Management-Token',
    });
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify({ openapi: '3.0.0', info: { title: 'Orders' } }), {
        status: 200,
      }),
    );
    render(<ApiExplorerPage deployment={deployment} onBack={() => {}} />);

    const scalar = await screen.findByTestId('scalar');
    expect(scalar.getAttribute('data-url')).toBe('');
    expect(scalar.textContent).toContain('"openapi":"3.0.0"');
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe('https://orders.example.dev/__mockforge/api/spec');
    expect(headerOf(init, 'X-MockForge-Management-Token')).toBe('mfm_tok');

    const link = screen.getByRole('link', { name: /Open in new tab/ });
    expect(link.getAttribute('href')).toBe(
      'https://orders.example.dev/__mockforge/docs#management_token=mfm_tok',
    );
  });

  it('sends the token on the routes fallback', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockResolvedValue({
      token: 'mfm_tok',
      header: 'X-MockForge-Management-Token',
    });
    vi.mocked(fetch)
      .mockResolvedValueOnce(new Response('not found', { status: 404 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ routes: [{ method: 'GET', path: '/orders' }] }), {
          status: 200,
        }),
      );
    render(<ApiExplorerPage deployment={deployment} onBack={() => {}} />);

    expect(await screen.findByText('/orders')).toBeInTheDocument();
    const [url, init] = vi.mocked(fetch).mock.calls[1];
    expect(url).toBe('https://orders.example.dev/__mockforge/routes');
    expect(headerOf(init, 'X-MockForge-Management-Token')).toBe('mfm_tok');
  });

  it('degrades gracefully when the token is unavailable', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockRejectedValue(new Error('Access denied'));
    render(<ApiExplorerPage deployment={deployment} onBack={() => {}} />);

    expect(await screen.findByText(/requires its management token/)).toBeInTheDocument();
    await waitFor(() => expect(fetchJsonWithErrorBody).toHaveBeenCalledTimes(1));
    expect(fetch).not.toHaveBeenCalled();
    expect(screen.queryByTestId('scalar')).not.toBeInTheDocument();
    const link = screen.getByRole('link', { name: /Open in new tab/ });
    expect(link.getAttribute('href')).toBe('https://orders.example.dev/__mockforge/docs');
  });
});
