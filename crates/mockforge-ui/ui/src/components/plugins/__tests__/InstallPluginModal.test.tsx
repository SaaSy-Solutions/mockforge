import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { InstallPluginModal } from '../InstallPluginModal';

const api = vi.hoisted(() => ({ authenticatedFetch: vi.fn() }));
vi.mock('@/utils/apiClient', () => api);

describe('plugin signature choice', () => {
  beforeEach(() => {
    api.authenticatedFetch.mockReset();
    api.authenticatedFetch.mockResolvedValue({
      json: async () => ({ success: false, error: 'Plugin signature verification failed' }),
    });
  });

  it('requires signatures by default and shows verification failures', async () => {
    render(<InstallPluginModal onClose={vi.fn()} />);
    expect(screen.getByRole('checkbox')).not.toBeChecked();
    fireEvent.change(screen.getByLabelText('Plugin Directory Path'), { target: { value: '/tmp/owned-plugin' } });
    fireEvent.click(screen.getByRole('button', { name: 'Install Plugin' }));
    await screen.findByText('Plugin signature verification failed');
    expect(JSON.parse(api.authenticatedFetch.mock.calls[0][1].body).allow_unsigned).toBe(false);
  });

  it('sends the development opt-in only after it is checked', async () => {
    render(<InstallPluginModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Plugin Directory Path'), { target: { value: '/tmp/owned-plugin' } });
    fireEvent.click(screen.getByRole('checkbox', { name: /Allow unsigned plugin/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Install Plugin' }));
    await waitFor(() => expect(api.authenticatedFetch).toHaveBeenCalledOnce());
    expect(JSON.parse(api.authenticatedFetch.mock.calls[0][1].body).allow_unsigned).toBe(true);
    await screen.findByText('Plugin signature verification failed');
  });

  it('keeps a visible error when the server supplies no error text', async () => {
    api.authenticatedFetch.mockResolvedValueOnce({ json: async () => ({ success: false }) });
    render(<InstallPluginModal onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Plugin Directory Path'), { target: { value: '/tmp/owned-plugin' } });
    fireEvent.click(screen.getByRole('button', { name: 'Install Plugin' }));
    await screen.findByText('Plugin installation failed');
  });
});
