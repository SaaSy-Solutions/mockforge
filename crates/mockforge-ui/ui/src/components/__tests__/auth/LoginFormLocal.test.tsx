import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { LoginForm } from '../../auth/LoginForm';
vi.mock('../../../services/authApi', () => ({ authApi: { isCloud: () => false, register: vi.fn() } }));
vi.mock('../../../stores/useAuthStore', () => ({ useAuthStore: () => ({ login: vi.fn(), setAuthenticated: vi.fn() }) }));
describe('Local admin login form', () => {
  it('offers no built-in demo accounts (#1016)', () => {
    const { container } = render(<MemoryRouter initialEntries={['/login']}><LoginForm /></MemoryRouter>);
    expect(screen.queryByRole('button', { name: /demo/i })).not.toBeInTheDocument();
    expect(container.textContent).not.toMatch(/admin123|viewer123|editor123/);
    expect(container.textContent).toContain('MOCKFORGE_ADMIN_PASSWORD');
  });
});
