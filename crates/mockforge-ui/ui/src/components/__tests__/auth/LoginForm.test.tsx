import { describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { LoginForm } from '../../auth/LoginForm';
vi.mock('../../../services/authApi', () => ({ authApi: { isCloud: () => true, register: vi.fn() } }));
vi.mock('../../../stores/useAuthStore', () => ({ useAuthStore: () => ({ login: vi.fn(), setAuthenticated: vi.fn() }) }));
describe('Cloud auth form', () => {
  it('offers password recovery from sign in', () => {
    render(<MemoryRouter initialEntries={['/login']}><LoginForm /></MemoryRouter>);
    expect(screen.getByRole('link', { name: 'Forgot password?' })).toHaveAttribute('href', '/forgot-password');
  });
  it('shows invalid email feedback and blocks account creation until valid', () => {
    render(<MemoryRouter initialEntries={['/register']}><LoginForm /></MemoryRouter>);
    fireEvent.change(screen.getByLabelText('Username'), { target: { value: 'fixture' } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'Fixture-password' } });
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'invalid' } });
    expect(screen.getByRole('alert')).toHaveTextContent('valid email');
    expect(screen.getByLabelText('Email')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByRole('button', { name: 'Create Account' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'fixture@example.com' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Account' })).toBeEnabled();
  });
});
