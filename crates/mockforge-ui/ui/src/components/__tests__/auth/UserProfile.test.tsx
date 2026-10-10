import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { UserProfile } from '../../auth/UserProfile';
import { I18nProvider } from '../../../i18n/I18nProvider';
const logout = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
vi.mock('../../../stores/useAuthStore', () => ({ useAuthStore: () => ({ user: { id: 'fixture', username: 'Fixture', role: 'admin' }, logout }) }));
vi.mock('../../auth/AccountSettings', () => ({ AccountSettings: () => null }));
vi.mock('../../auth/ProfileSettings', () => ({ ProfileSettings: () => null }));
vi.mock('../../auth/Preferences', () => ({ Preferences: () => null }));
function Path() { return <p data-testid="path">{useLocation().pathname}</p>; }
describe('Sign out navigation', () => {
  it('leaves the protected URL and shows /login immediately', async () => {
    render(<I18nProvider><MemoryRouter initialEntries={['/usage']}><UserProfile /><Path /></MemoryRouter></I18nProvider>);
    await userEvent.click(screen.getByRole('button', { name: 'Account menu' }));
    fireEvent.click(await screen.findByRole('menuitem', { name: /sign out/i }));
    expect(logout).toHaveBeenCalledOnce();
    expect(screen.getByTestId('path')).toHaveTextContent('/login');
  });
});
