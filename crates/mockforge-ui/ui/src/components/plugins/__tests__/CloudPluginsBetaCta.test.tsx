/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const cloudModeMock = vi.hoisted(() => ({ isCloudMode: vi.fn(() => true) }));
vi.mock('../../../utils/cloudMode', () => cloudModeMock);

const apiMock = vi.hoisted(() => ({
  getMyBetaInterest: vi.fn(),
  submitBetaInterest: vi.fn(),
}));
vi.mock('../../../services/api/cloudPlugins', () => ({
  cloudPluginsApi: apiMock,
}));

import { CloudPluginsBetaCta } from '../CloudPluginsBetaCta';

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
};

describe('CloudPluginsBetaCta', () => {
  beforeEach(() => {
    // Reset (not just clear) wipes the mockResolvedValue queue so each
    // test starts from a known-empty state.
    apiMock.getMyBetaInterest.mockReset();
    apiMock.submitBetaInterest.mockReset();
    cloudModeMock.isCloudMode.mockReset();
    cloudModeMock.isCloudMode.mockReturnValue(true);
  });

  it('renders nothing in self-hosted (local) mode', () => {
    cloudModeMock.isCloudMode.mockReturnValue(false);
    apiMock.getMyBetaInterest.mockResolvedValue({ signed_up: false });

    const { container } = renderWithProviders(<CloudPluginsBetaCta />);

    expect(container.firstChild).toBeNull();
    expect(apiMock.getMyBetaInterest).not.toHaveBeenCalled();
  });

  it('shows the request-access banner when the user has not signed up', async () => {
    apiMock.getMyBetaInterest.mockResolvedValue({ signed_up: false });

    renderWithProviders(<CloudPluginsBetaCta />);

    await waitFor(() => {
      expect(screen.getByTestId('cloud-plugins-beta-cta')).toBeInTheDocument();
    });
    // Use role-based query so "Request beta access" matches only the button,
    // not the subtitle text that also contains the phrase.
    expect(
      screen.getByRole('button', { name: /Request beta access/i }),
    ).toBeInTheDocument();
  });

  it('shows the signed-up banner when the user already registered interest', async () => {
    apiMock.getMyBetaInterest.mockResolvedValue({
      signed_up: true,
      created_at: '2026-05-05T12:00:00Z',
      use_case: 'request transformer for compliance redaction',
    });

    renderWithProviders(<CloudPluginsBetaCta />);

    await waitFor(() => {
      expect(
        screen.getByTestId('cloud-plugins-beta-signed-up'),
      ).toBeInTheDocument();
    });
    expect(screen.queryByTestId('cloud-plugins-beta-cta')).not.toBeInTheDocument();
  });

  it('submits the use case and switches to the signed-up state', async () => {
    // First call (initial load) → not signed up. After submit, the
    // mutation invalidates the query and we expect the next call to
    // return signed_up = true.
    apiMock.getMyBetaInterest
      .mockResolvedValueOnce({ signed_up: false })
      .mockResolvedValue({
        signed_up: true,
        created_at: '2026-05-05T12:00:00Z',
        use_case: 'webhook signing plugin',
      });
    apiMock.submitBetaInterest.mockResolvedValue({
      id: '00000000-0000-0000-0000-000000000001',
      created_at: '2026-05-05T12:00:00Z',
      updated_at: '2026-05-05T12:00:00Z',
    });

    renderWithProviders(<CloudPluginsBetaCta />);

    fireEvent.click(
      await screen.findByRole('button', { name: /Request beta access/i }),
    );

    const textarea = await screen.findByTestId('beta-use-case-input');
    fireEvent.change(textarea, { target: { value: 'webhook signing plugin' } });

    fireEvent.click(screen.getByTestId('beta-submit-button'));

    await waitFor(() => {
      expect(apiMock.submitBetaInterest).toHaveBeenCalledWith({
        use_case: 'webhook signing plugin',
      });
    });

    await waitFor(() => {
      expect(
        screen.getByTestId('cloud-plugins-beta-signed-up'),
      ).toBeInTheDocument();
    });
  });

  it('omits use_case from the payload when the textarea is empty', async () => {
    apiMock.getMyBetaInterest
      .mockResolvedValueOnce({ signed_up: false })
      .mockResolvedValue({ signed_up: true });
    apiMock.submitBetaInterest.mockResolvedValue({
      id: '00000000-0000-0000-0000-000000000001',
      created_at: '2026-05-05T12:00:00Z',
      updated_at: '2026-05-05T12:00:00Z',
    });

    renderWithProviders(<CloudPluginsBetaCta />);

    fireEvent.click(
      await screen.findByRole('button', { name: /Request beta access/i }),
    );
    fireEvent.click(await screen.findByTestId('beta-submit-button'));

    await waitFor(() => {
      expect(apiMock.submitBetaInterest).toHaveBeenCalledWith({
        use_case: undefined,
      });
    });
  });

  it('renders nothing on API error to avoid blocking the page', async () => {
    apiMock.getMyBetaInterest.mockRejectedValue(new Error('boom'));

    const { container } = renderWithProviders(<CloudPluginsBetaCta />);

    await waitFor(() => {
      expect(
        container.querySelector('[data-testid^="cloud-plugins-beta"]'),
      ).toBeNull();
    });
  });
});
