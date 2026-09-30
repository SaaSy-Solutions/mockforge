import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ManagementTokenField } from '../ManagementTokenField';
import { fetchJsonWithErrorBody } from '@/services/api/client';

vi.mock('@/services/api/client', () => ({ fetchJsonWithErrorBody: vi.fn() }));

describe('ManagementTokenField', () => {

  it('fetches the token only when revealed', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockResolvedValue({
      token: 'mfm_abc123',
      header: 'X-MockForge-Management-Token',
    });
    render(<ManagementTokenField deploymentId="dep-1" />);
    expect(fetchJsonWithErrorBody).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: /Reveal/ }));
    expect(await screen.findByText('mfm_abc123')).toBeInTheDocument();
    expect(fetchJsonWithErrorBody).toHaveBeenCalledWith(
      '/api/v1/hosted-mocks/dep-1/management-token',
    );
    expect(screen.getByText(/Send as X-MockForge-Management-Token/)).toBeInTheDocument();
  });

  it('shows why a reveal failed', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockImplementation(async () => {
      throw new Error('Access denied');
    });
    render(<ManagementTokenField deploymentId="dep-1" />);
    fireEvent.click(screen.getByRole('button', { name: /Reveal/ }));
    await waitFor(() => expect(screen.getByText('Access denied')).toBeInTheDocument());
  });
});
