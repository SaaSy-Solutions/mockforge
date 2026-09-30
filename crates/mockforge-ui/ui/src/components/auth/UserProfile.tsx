import { useState } from 'react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import {
  BookOpen,
  ChevronsUpDown,
  CircleHelp,
  Eye,
  Languages,
  LogOut,
  Check,
  SlidersHorizontal,
  ShieldCheck,
  User as UserIcon,
  UserCog,
} from 'lucide-react';
import { cn } from '../../utils/cn';
import { useAuthStore } from '../../stores/useAuthStore';
import { useHelpStore } from '../../stores/useHelpStore';
import { useI18n } from '../../i18n/I18nProvider';
import { AccountSettings } from './AccountSettings';
import { ProfileSettings } from './ProfileSettings';
import { Preferences } from './Preferences';
import { menuContentClass, menuItemClass, menuLabelClass } from '../layout/WorkspaceSwitcher';

const roleMeta: Record<string, { icon: typeof UserIcon; className: string }> = {
  admin: {
    icon: ShieldCheck,
    className: 'text-primary',
  },
  viewer: {
    icon: Eye,
    className: 'text-info-600 dark:text-info-400',
  },
};

interface UserProfileProps {
  /** Icon-only trigger for the collapsed sidebar rail. */
  collapsed?: boolean;
}

export function UserProfile({ collapsed = false }: UserProfileProps) {
  const { user, logout } = useAuthStore();
  const openHelp = useHelpStore((state) => state.open);
  const { t, locale, supportedLocales, setLocale } = useI18n();
  const [showAccountSettings, setShowAccountSettings] = useState(false);
  const [showProfileSettings, setShowProfileSettings] = useState(false);
  const [showPreferences, setShowPreferences] = useState(false);

  if (!user) return null;

  const displayName = user.username || user.email || 'User';
  const initial = displayName.charAt(0).toUpperCase();
  const roleName = user.role || 'member';
  const role = roleMeta[roleName] ?? { icon: UserIcon, className: 'text-muted-foreground' };
  const RoleIcon = role.icon;

  return (
    <>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger
          aria-label="Account menu"
          title={collapsed ? displayName : undefined}
          className={cn(
            'flex w-full items-center gap-2.5 rounded-lg text-left outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring data-[state=open]:bg-muted',
            collapsed ? 'h-10 justify-center' : 'h-11 px-2',
          )}
        >
          <span
            aria-hidden
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-secondary text-xs font-semibold text-secondary-foreground ring-1 ring-inset ring-border"
          >
            {initial}
          </span>
          {!collapsed && (
            <>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium leading-tight text-foreground">
                  {displayName}
                </span>
                <span className="flex items-center gap-1 truncate text-[11px] leading-tight text-muted-foreground">
                  <RoleIcon className={cn('h-3 w-3', role.className)} aria-hidden />
                  <span className="capitalize">{roleName}</span>
                </span>
              </span>
              <ChevronsUpDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
            </>
          )}
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            side={collapsed ? 'right' : 'top'}
            align={collapsed ? 'end' : 'start'}
            sideOffset={6}
            className={cn(menuContentClass, 'w-64')}
          >
            <div className="px-2 py-2">
              <div className="truncate text-sm font-medium text-foreground">{displayName}</div>
              {user.email && user.email !== displayName && (
                <div className="truncate text-xs text-muted-foreground">{user.email}</div>
              )}
              <div className="mt-1.5 inline-flex items-center gap-1 rounded-full border border-border px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                <RoleIcon className={cn('h-3 w-3', role.className)} aria-hidden />
                <span className="capitalize">{roleName}</span>
              </div>
            </div>
            <DropdownMenu.Separator className="my-1 h-px bg-border" />
            <DropdownMenu.Item className={menuItemClass} onSelect={() => setShowAccountSettings(true)}>
              <UserCog className="h-4 w-4 text-muted-foreground" aria-hidden />
              {t('user.account')}
            </DropdownMenu.Item>
            <DropdownMenu.Item className={menuItemClass} onSelect={() => setShowProfileSettings(true)}>
              <UserIcon className="h-4 w-4 text-muted-foreground" aria-hidden />
              {t('user.profile')}
            </DropdownMenu.Item>
            <DropdownMenu.Item className={menuItemClass} onSelect={() => setShowPreferences(true)}>
              <SlidersHorizontal className="h-4 w-4 text-muted-foreground" aria-hidden />
              {t('user.preferences')}
            </DropdownMenu.Item>

            {supportedLocales.length > 1 && (
              <>
                <DropdownMenu.Separator className="my-1 h-px bg-border" />
                <DropdownMenu.Label className={cn(menuLabelClass, 'flex items-center gap-1.5')}>
                  <Languages className="h-3 w-3" aria-hidden />
                  {t('user.language')}
                </DropdownMenu.Label>
                {supportedLocales.map((l) => (
                  <DropdownMenu.Item key={l} className={menuItemClass} onSelect={() => setLocale(l)}>
                    <span className="flex-1 uppercase">{l}</span>
                    {l === locale && <Check className="h-4 w-4 text-primary" aria-hidden />}
                  </DropdownMenu.Item>
                ))}
              </>
            )}

            <DropdownMenu.Separator className="my-1 h-px bg-border" />
            <DropdownMenu.Item asChild className={menuItemClass}>
              <a href="https://docs.mockforge.dev/api/admin-ui-rest.html" target="_blank" rel="noopener noreferrer">
                <BookOpen className="h-4 w-4 text-muted-foreground" aria-hidden />
                {t('shell.docs')}
              </a>
            </DropdownMenu.Item>
            <DropdownMenu.Item className={menuItemClass} onSelect={() => openHelp()}>
              <CircleHelp className="h-4 w-4 text-muted-foreground" aria-hidden />
              {t('shell.help')}
            </DropdownMenu.Item>
            <DropdownMenu.Separator className="my-1 h-px bg-border" />
            <DropdownMenu.Item
              className={cn(menuItemClass, 'text-danger-600 data-[highlighted]:bg-danger-50 dark:text-danger-400 dark:data-[highlighted]:bg-danger-900/20')}
              onSelect={() => void logout()}
            >
              <LogOut className="h-4 w-4" aria-hidden />
              {t('user.signOut')}
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>

      <AccountSettings open={showAccountSettings} onOpenChange={setShowAccountSettings} />
      <ProfileSettings open={showProfileSettings} onOpenChange={setShowProfileSettings} />
      <Preferences open={showPreferences} onOpenChange={setShowPreferences} />
    </>
  );
}
