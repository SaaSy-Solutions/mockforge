import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PasswordRecoveryPage } from '../PasswordRecoveryPage';
import { authApi } from '../../services/authApi';

vi.mock('../../services/authApi', () => ({ authApi: { requestPasswordReset: vi.fn(), resetPassword: vi.fn() } }));
function show(reset = false, url = '/forgot-password') {
  return render(<MemoryRouter initialEntries={[url]}><PasswordRecoveryPage reset={reset} /></MemoryRouter>);
}
describe('Password recovery', () => {
  beforeEach(() => vi.clearAllMocks());
  it('validates email and gives the same confirmation without revealing accounts', async () => {
    vi.mocked(authApi.requestPasswordReset).mockResolvedValue({ success: true, message: 'sent' });
    show();
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'invalid' } });
    expect(screen.getByRole('alert')).toHaveTextContent('valid email');
    expect(screen.getByRole('button', { name: 'Send reset link' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'fixture@example.com' } });
    fireEvent.click(screen.getByRole('button', { name: 'Send reset link' }));
    expect(await screen.findByRole('status')).toHaveTextContent('If an account exists');
    expect(authApi.requestPasswordReset).toHaveBeenCalledWith('fixture@example.com');
  });
  it('rejects a missing token and offers a new reset link', () => {
    show(true, '/reset-password');
    expect(screen.getByRole('alert')).toHaveTextContent('incomplete');
    expect(screen.getByRole('link', { name: 'Request a new reset link' })).toHaveAttribute('href', '/forgot-password');
    expect(authApi.resetPassword).not.toHaveBeenCalled();
  });
  it('checks matching passwords and submits the reset link token', async () => {
    vi.mocked(authApi.resetPassword).mockResolvedValue({ success: true, message: 'reset' });
    show(true, '/reset-password?token=synthetic-test-token');
    fireEvent.change(screen.getByLabelText('New password'), { target: { value: 'Fixture-password' } });
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'wrong' } });
    expect(screen.getByRole('button', { name: 'Reset password' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'Fixture-password' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reset password' }));
    expect(await screen.findByRole('status')).toHaveTextContent('has been reset');
    expect(authApi.resetPassword).toHaveBeenCalledWith('synthetic-test-token', 'Fixture-password');
  });
  it('keeps invalid or expired token feedback visible with a recovery path', async () => {
    vi.mocked(authApi.resetPassword).mockRejectedValue(new Error('Reset link has expired'));
    show(true, '/reset-password?token=expired-fixture');
    fireEvent.change(screen.getByLabelText('New password'), { target: { value: 'Fixture-password' } });
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'Fixture-password' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reset password' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('expired'));
    expect(screen.getByRole('link', { name: 'Request a new reset link' })).toBeVisible();
  });
});
