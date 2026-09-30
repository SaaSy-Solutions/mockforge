import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import {
  ChevronRight,
  CircleHelp,
  Lock,
  Menu,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Search,
  X,
} from 'lucide-react';
import { cn } from '../../utils/cn';
import { SimpleThemeToggle } from '../ui/ThemeToggle';
import { UserProfile } from '../auth/UserProfile';
import { HelpSupport } from '../auth/HelpSupport';
import { Logo } from '../ui/Logo';
import { useHelpStore } from '../../stores/useHelpStore';
import { usePreferencesStore } from '../../stores/usePreferencesStore';
import { useAppShortcuts } from '../../hooks/useKeyboardNavigation';
import { useSkipLinks } from '../../hooks/useFocusManagement';
import { useI18n } from '../../i18n/I18nProvider';
import { GlobalConnectionStatus } from './ConnectionStatus';
import { CommandPalette } from './CommandPalette';
import { WorkspaceSwitcher } from './WorkspaceSwitcher';
import {
  findNavItem,
  getHelpNavItems,
  getNavSections,
  recordRecentPage,
  type ResolvedNavItem,
  type ResolvedNavSection,
} from './navigation';

interface AppShellProps {
  children: React.ReactNode;
  onRefresh: () => void;
}

const OPEN_SECTIONS_KEY = 'mockforge-nav-open-sections';
// A calm first impression: the everyday groups start open, the rest stay
// folded until the user opens them (or navigates into them).
const DEFAULT_OPEN_SECTIONS = ['overview', 'mocks', 'observe'];

function readOpenSections(): Set<string> {
  try {
    const raw = localStorage.getItem(OPEN_SECTIONS_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (Array.isArray(parsed)) return new Set(parsed.filter((v): v is string => typeof v === 'string'));
  } catch {
    // fall through to defaults
  }
  return new Set(DEFAULT_OPEN_SECTIONS);
}

function writeOpenSections(open: Set<string>) {
  try {
    localStorage.setItem(OPEN_SECTIONS_KEY, JSON.stringify([...open]));
  } catch {
    // Persisting is a convenience only.
  }
}

function useIsMac() {
  const [isMac, setIsMac] = useState(false);
  useEffect(() => {
    setIsMac(/mac/i.test(navigator.userAgent));
  }, []);
  return isMac;
}

/* -------------------------------------------------------------------------- */
/* Sidebar                                                                    */
/* -------------------------------------------------------------------------- */

interface SidebarProps {
  sections: ResolvedNavSection[];
  activeId: string;
  collapsed: boolean;
  openSections: Set<string>;
  onToggleSection: (id: string) => void;
  onNavigate?: () => void;
  onOpenSearch: () => void;
  onToggleCollapsed?: () => void;
  onClose?: () => void;
}

function NavLinkItem({
  item,
  active,
  collapsed,
  onNavigate,
}: {
  item: ResolvedNavItem;
  active: boolean;
  collapsed: boolean;
  onNavigate?: () => void;
}) {
  const { t } = useI18n();
  const Icon = item.icon;
  const label = t(item.labelKey);

  const classes = cn(
    'group relative flex h-8 w-full items-center gap-2.5 rounded-md text-[13px] font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring',
    collapsed ? 'justify-center px-0' : 'px-2.5',
    item.localOnly
      ? 'cursor-not-allowed text-muted-foreground/60'
      : active
        ? 'bg-muted text-foreground'
        : 'text-muted-foreground hover:bg-muted/70 hover:text-foreground',
  );

  const content = (
    <>
      {active && !collapsed && (
        <span aria-hidden className="absolute -left-3 top-1.5 h-5 w-[3px] rounded-r-full bg-primary" />
      )}
      <Icon
        className={cn(
          'h-4 w-4 shrink-0',
          active ? 'text-primary' : 'text-muted-foreground group-hover:text-foreground',
          item.localOnly && 'opacity-60',
        )}
        aria-hidden
      />
      {!collapsed && <span className="flex-1 truncate text-left">{label}</span>}
      {!collapsed && item.localOnly && <Lock className="h-3 w-3 shrink-0" aria-hidden />}
    </>
  );

  if (item.localOnly) {
    return (
      <span
        role="link"
        aria-disabled="true"
        title={t('nav.localOnly.tooltip')}
        aria-label={`${label} — ${t('nav.localOnly.tooltip')}`}
        className={classes}
      >
        {content}
      </span>
    );
  }

  return (
    <Link
      to={'/' + item.id}
      onClick={onNavigate}
      aria-current={active ? 'page' : undefined}
      aria-label={collapsed ? label : undefined}
      title={collapsed ? label : undefined}
      className={classes}
    >
      {content}
    </Link>
  );
}

function SidebarContent({
  sections,
  activeId,
  collapsed,
  openSections,
  onToggleSection,
  onNavigate,
  onOpenSearch,
  onToggleCollapsed,
  onClose,
}: SidebarProps) {
  const { t } = useI18n();
  const isMac = useIsMac();
  const helpItems = getHelpNavItems();

  return (
    <div className="flex h-full flex-col">
      {/* Brand row */}
      <div className={cn('flex h-14 shrink-0 items-center gap-2', collapsed ? 'justify-center px-2' : 'px-4')}>
        <Link to="/dashboard" onClick={onNavigate} className="flex items-center gap-2 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <Logo variant="icon" size="sm" />
          {!collapsed && (
            <span className="text-[15px] font-semibold tracking-tight text-foreground">{t('app.brand')}</span>
          )}
        </Link>
        {onToggleCollapsed && !collapsed && (
          <button
            type="button"
            onClick={onToggleCollapsed}
            aria-label={t('a11y.collapseSidebar')}
            title={t('a11y.collapseSidebar')}
            className="ml-auto flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
          >
            <PanelLeftClose className="h-4 w-4" />
          </button>
        )}
        {onClose && (
          <button
            type="button"
            onClick={onClose}
            aria-label={t('shell.closeMenu')}
            className="ml-auto flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Workspace + search */}
      <div className={cn('space-y-1.5 pb-3', collapsed ? 'px-2' : 'px-3')}>
        <WorkspaceSwitcher collapsed={collapsed} />
        <button
          type="button"
          id="global-search-trigger"
          onClick={onOpenSearch}
          aria-label={t('shell.search')}
          title={collapsed ? t('shell.search') : undefined}
          className={cn(
            'flex h-8 w-full items-center gap-2 rounded-md border border-border bg-background text-sm text-muted-foreground outline-none transition-colors hover:border-foreground/20 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring',
            collapsed ? 'justify-center' : 'px-2.5',
          )}
        >
          <Search className="h-3.5 w-3.5 shrink-0" aria-hidden />
          {!collapsed && (
            <>
              <span className="flex-1 text-left">{t('shell.search')}…</span>
              <kbd className="hidden rounded border border-border bg-muted px-1 font-mono text-[10px] leading-4 md:inline">
                {isMac ? '⌘K' : 'Ctrl K'}
              </kbd>
            </>
          )}
        </button>
      </div>

      {/* Sections */}
      <nav
        id="main-navigation"
        aria-label={t('a11y.mainNavigation')}
        className={cn('flex-1 overflow-y-auto pb-4 custom-scrollbar', collapsed ? 'px-2' : 'px-3')}
      >
        {sections.map((section, index) => {
          const titled = Boolean(section.titleKey);
          const isOpen = collapsed || !titled || openSections.has(section.id);
          const containsActive = section.items.some((i) => i.id === activeId);
          const listId = `nav-section-${section.id}`;
          return (
            <div key={section.id} className={cn(index > 0 && (collapsed ? 'mt-2 border-t border-border pt-2' : 'mt-3'))}>
              {titled && !collapsed && (
                <button
                  type="button"
                  onClick={() => onToggleSection(section.id)}
                  aria-expanded={isOpen}
                  aria-controls={listId}
                  className="group flex h-7 w-full items-center gap-1 rounded-md px-2.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/80 outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <span className="flex-1 text-left">{t(section.titleKey!)}</span>
                  {!isOpen && containsActive && (
                    <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-primary" />
                  )}
                  <ChevronRight
                    className={cn('h-3.5 w-3.5 transition-transform duration-150', isOpen && 'rotate-90')}
                    aria-hidden
                  />
                </button>
              )}
              {isOpen && (
                <div id={listId} className="space-y-px">
                  {section.id === 'local-only' && !collapsed && (
                    <p className="px-2.5 pb-1 text-[11px] leading-snug text-muted-foreground">
                      {t('nav.localOnly.hint')}
                    </p>
                  )}
                  {section.items.map((item) => (
                    <NavLinkItem
                      key={item.id}
                      item={item}
                      active={item.id === activeId}
                      collapsed={collapsed}
                      onNavigate={onNavigate}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </nav>

      {/* Footer */}
      <div className={cn('shrink-0 space-y-px border-t border-border py-2', collapsed ? 'px-2' : 'px-3')}>
        {helpItems.map((item) => (
          <NavLinkItem
            key={item.id}
            item={item}
            active={item.id === activeId}
            collapsed={collapsed}
            onNavigate={onNavigate}
          />
        ))}
        {onToggleCollapsed && collapsed && (
          <button
            type="button"
            onClick={onToggleCollapsed}
            aria-label={t('a11y.expandSidebar')}
            title={t('a11y.expandSidebar')}
            className="flex h-8 w-full items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
          >
            <PanelLeftOpen className="h-4 w-4" />
          </button>
        )}
        <div className="pt-1">
          <UserProfile collapsed={collapsed} />
        </div>
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------- */
/* Shell                                                                      */
/* -------------------------------------------------------------------------- */

export function AppShell({ children, onRefresh }: AppShellProps) {
  const { t } = useI18n();
  const location = useLocation();
  // First path segment, so nested routes (/hosted-mocks/:id) keep their parent highlighted.
  const activeId = location.pathname.split('/').filter(Boolean)[0] ?? 'dashboard';

  const sections = useMemo(() => getNavSections(), []);
  const current = findNavItem(activeId);
  const currentSectionTitle = current?.section?.titleKey ? t(current.section.titleKey) : undefined;
  const currentLabel = current
    ? t(current.item.labelKey)
    : activeId.replace(/-/g, ' ').replace(/^\w/, (c) => c.toUpperCase());

  const [mobileOpen, setMobileOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [openSections, setOpenSections] = useState<Set<string>>(readOpenSections);

  const helpOpen = useHelpStore((state) => state.isOpen);
  const openHelp = useHelpStore((state) => state.open);
  const setHelpOpen = useHelpStore((state) => state.setOpen);
  const keyboardShortcutsEnabled = usePreferencesStore((state) => state.preferences.ui.keyboardShortcuts);
  const sidebarCollapsed = usePreferencesStore((state) => state.preferences.ui.sidebarCollapsed);
  const updateUI = usePreferencesStore((state) => state.updateUI);

  const toggleCollapsed = useCallback(
    () => updateUI({ sidebarCollapsed: !sidebarCollapsed }),
    [updateUI, sidebarCollapsed],
  );

  const toggleSection = useCallback((id: string) => {
    setOpenSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      writeOpenSections(next);
      return next;
    });
  }, []);

  // Navigating into a folded group (e.g. from ⌘K) unfolds it so the active
  // item is always visible in the sidebar.
  const activeSectionId = sections.find((s) => s.items.some((i) => i.id === activeId))?.id;
  useEffect(() => {
    if (!activeSectionId) return;
    setOpenSections((prev) => {
      if (prev.has(activeSectionId)) return prev;
      const next = new Set(prev);
      next.add(activeSectionId);
      writeOpenSections(next);
      return next;
    });
  }, [activeSectionId]);

  useEffect(() => {
    recordRecentPage(activeId);
  }, [activeId]);

  // ⌘K / Ctrl+K opens the palette — handled here (not in useAppShortcuts,
  // which only matches Ctrl) so the macOS binding works and it fires even
  // while focus is inside a text field.
  useEffect(() => {
    if (!keyboardShortcutsEnabled) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [keyboardShortcutsEnabled]);

  useAppShortcuts({
    onHelp: () => openHelp(),
    enabled: keyboardShortcutsEnabled,
  });

  const { createSkipLink } = useSkipLinks();

  const sidebarProps = {
    sections,
    activeId,
    openSections,
    onToggleSection: toggleSection,
    onOpenSearch: () => {
      setMobileOpen(false);
      setPaletteOpen(true);
    },
  };

  const iconButton =
    'flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring';

  return (
    <div className="min-h-screen bg-bg-secondary">
      <nav className="sr-only focus-within:not-sr-only" aria-label="Skip links">
        <a {...createSkipLink('main-navigation', t('a11y.skipNavigation'))} />
        <a {...createSkipLink('main-content', t('a11y.skipMain'))} />
        <a {...createSkipLink('global-search-trigger', t('a11y.skipSearch'))} />
      </nav>

      {/* Desktop sidebar */}
      <aside
        className={cn(
          'fixed inset-y-0 left-0 z-40 hidden border-r border-border bg-bg-primary transition-[width] duration-200 md:block',
          sidebarCollapsed ? 'w-14' : 'w-60',
        )}
      >
        <SidebarContent {...sidebarProps} collapsed={sidebarCollapsed} onToggleCollapsed={toggleCollapsed} />
      </aside>

      {/* Mobile drawer */}
      <DialogPrimitive.Root open={mobileOpen} onOpenChange={setMobileOpen}>
        <DialogPrimitive.Portal>
          <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/40 data-[state=open]:animate-fade-in md:hidden" />
          <DialogPrimitive.Content
            aria-describedby={undefined}
            className="fixed inset-y-0 left-0 z-50 w-72 max-w-[85vw] border-r border-border bg-bg-primary shadow-xl data-[state=open]:animate-fade-in md:hidden"
          >
            <DialogPrimitive.Title className="sr-only">{t('a11y.mainNavigation')}</DialogPrimitive.Title>
            <SidebarContent
              {...sidebarProps}
              collapsed={false}
              onNavigate={() => setMobileOpen(false)}
              onClose={() => setMobileOpen(false)}
            />
          </DialogPrimitive.Content>
        </DialogPrimitive.Portal>
      </DialogPrimitive.Root>

      <div className={cn('flex min-h-screen min-w-0 flex-col', sidebarCollapsed ? 'md:pl-14' : 'md:pl-60')}>
        <header className="sticky top-0 z-30 flex h-14 shrink-0 items-center gap-2 border-b border-border bg-bg-secondary/85 px-4 backdrop-blur supports-[backdrop-filter]:bg-bg-secondary/70 sm:px-6 lg:px-8">
          <button
            type="button"
            className={cn(iconButton, '-ml-1.5 md:hidden')}
            onClick={() => setMobileOpen(true)}
            aria-label={t('shell.openMenu')}
          >
            <Menu className="h-5 w-5" />
          </button>

          <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-1.5 text-sm">
            {currentSectionTitle && (
              <>
                <span className="hidden truncate text-muted-foreground sm:inline">{currentSectionTitle}</span>
                <ChevronRight className="hidden h-3.5 w-3.5 shrink-0 text-muted-foreground/60 sm:inline" aria-hidden />
              </>
            )}
            <span className="truncate font-medium text-foreground" aria-current="page">
              {currentLabel}
            </span>
          </nav>

          <div className="ml-auto flex items-center gap-1">
            <GlobalConnectionStatus className="mr-2 hidden sm:flex" />
            <button
              type="button"
              className={cn(iconButton, 'md:hidden')}
              onClick={() => setPaletteOpen(true)}
              aria-label={t('shell.search')}
            >
              <Search className="h-4 w-4" />
            </button>
            <button
              type="button"
              className={iconButton}
              onClick={onRefresh}
              aria-label={t('app.refresh')}
              title={t('app.refresh')}
            >
              <RefreshCw className="h-4 w-4" />
            </button>
            <SimpleThemeToggle className={iconButton} />
            <button
              type="button"
              className={iconButton}
              onClick={() => openHelp()}
              aria-label={t('shell.help')}
              title={t('shell.help')}
            >
              <CircleHelp className="h-4 w-4" />
            </button>
          </div>
        </header>

        <main id="main-content" className="flex-1" role="main" aria-label={t('a11y.mainContent')}>
          <div className="mx-auto w-full max-w-[1600px] px-4 py-6 sm:px-6 lg:px-8">{children}</div>
        </main>
      </div>

      <CommandPalette
        open={paletteOpen}
        onOpenChange={setPaletteOpen}
        onRefresh={onRefresh}
        onToggleSidebar={toggleCollapsed}
        onOpenHelp={openHelp}
      />
      <HelpSupport open={helpOpen} onOpenChange={setHelpOpen} />
    </div>
  );
}
