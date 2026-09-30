/**
 * ⌘K command palette — the shell's single search surface.
 *
 * Jumps to pages (same matcher + keywords as the sidebar), switches
 * workspaces, runs shell actions, and hands free text to the logs / services
 * filters. Replaces the old header search box + scope dropdown.
 */
import React, { useEffect, useMemo, useRef, useState } from 'react';
import * as DialogPrimitive from '@radix-ui/react-dialog';
import { useNavigate } from 'react-router-dom';
import type { LucideIcon } from 'lucide-react';
import {
  CircleHelp,
  CornerDownLeft,
  FileText,
  FolderOpen,
  History,
  PanelLeft,
  RefreshCw,
  Search,
  Server,
  SunMoon,
} from 'lucide-react';
import { cn } from '../../utils/cn';
import { useI18n } from '../../i18n/I18nProvider';
import { useLogStore } from '../../stores/useLogStore';
import { useServiceStore } from '../../stores/useServiceStore';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { usePreferencesStore } from '../../stores/usePreferencesStore';
import { useThemePaletteStore } from '../../stores/useThemePaletteStore';
import { getHelpNavItems, getNavSections, matchesNavQuery, readRecentPages } from './navigation';

interface PaletteItem {
  key: string;
  group: 'recent' | 'pages' | 'workspaces' | 'actions';
  label: string;
  hint?: string;
  icon: LucideIcon;
  onSelect: () => void;
}

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRefresh: () => void;
  onToggleSidebar: () => void;
  onOpenHelp: () => void;
}

export function CommandPalette({
  open,
  onOpenChange,
  onRefresh,
  onToggleSidebar,
  onOpenHelp,
}: CommandPaletteProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const setLogFilter = useLogStore((s) => s.setFilter);
  const setGlobalSearch = useServiceStore((s) => s.setGlobalSearch);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const setActiveWorkspaceById = useWorkspaceStore((s) => s.setActiveWorkspaceById);
  const searchScope = usePreferencesStore((s) => s.preferences.search.defaultScope);
  const theme = useThemePaletteStore((s) => s.resolvedMode);
  const setTheme = useThemePaletteStore((s) => s.setTheme);

  useEffect(() => {
    if (open) {
      setQuery('');
      setActive(0);
    }
  }, [open]);

  const items = useMemo<PaletteItem[]>(() => {
    const close = () => onOpenChange(false);
    const q = query.trim();
    const sections = getNavSections();
    const titleOf = (sectionId: string) => {
      const section = sections.find((s) => s.id === sectionId);
      return section?.titleKey ? t(section.titleKey) : undefined;
    };
    const pages = [...sections.flatMap((s) => s.items), ...getHelpNavItems()].filter(
      (item) => !item.localOnly,
    );
    const goTo = (id: string) => () => {
      navigate('/' + id);
      close();
    };

    const result: PaletteItem[] = [];

    if (!q) {
      for (const id of readRecentPages()) {
        const page = pages.find((p) => p.id === id);
        if (!page) continue;
        result.push({
          key: 'recent:' + id,
          group: 'recent',
          label: t(page.labelKey),
          hint: titleOf(page.sectionId),
          icon: History,
          onSelect: goTo(id),
        });
      }
    }

    // Section names count as search terms too ("settings" lists every settings page).
    const pageMatches = pages.filter((p) =>
      matchesNavQuery(p, `${t(p.labelKey)} ${titleOf(p.sectionId) ?? ''}`, q),
    );
    for (const page of q ? pageMatches.slice(0, 8) : pageMatches.slice(0, 6)) {
      result.push({
        key: 'page:' + page.id,
        group: 'pages',
        label: t(page.labelKey),
        hint: titleOf(page.sectionId),
        icon: page.icon,
        onSelect: goTo(page.id),
      });
    }

    for (const ws of workspaces) {
      if (ws.id === activeWorkspace?.id) continue;
      if (q && !ws.name.toLowerCase().includes(q.toLowerCase())) continue;
      result.push({
        key: 'ws:' + ws.id,
        group: 'workspaces',
        label: `${t('shell.workspace.switch')} ${ws.name}`,
        icon: FolderOpen,
        onSelect: () => {
          void setActiveWorkspaceById(ws.id);
          close();
        },
      });
    }

    const actions: PaletteItem[] = [];
    if (q) {
      // Honour the "default search scope" preference for free-text search.
      const wantLogs = searchScope !== 'services';
      const wantServices = searchScope !== 'logs';
      if (wantLogs) {
        actions.push({
          key: 'action:logs',
          group: 'actions',
          label: `${t('shell.action.searchLogs')} “${q}”`,
          icon: FileText,
          onSelect: () => {
            setLogFilter({ path_pattern: q });
            navigate('/logs');
            close();
          },
        });
      }
      if (wantServices) {
        actions.push({
          key: 'action:services',
          group: 'actions',
          label: `${t('shell.action.searchServices')} “${q}”`,
          icon: Server,
          onSelect: () => {
            setGlobalSearch(q);
            navigate('/services');
            close();
          },
        });
      }
    }
    const shellActions: PaletteItem[] = [
      {
        key: 'action:theme',
        group: 'actions',
        label: t('shell.action.toggleTheme'),
        icon: SunMoon,
        onSelect: () => {
          setTheme(theme === 'dark' ? 'light' : 'dark');
          close();
        },
      },
      {
        key: 'action:refresh',
        group: 'actions',
        label: t('shell.action.refresh'),
        icon: RefreshCw,
        onSelect: () => {
          onRefresh();
          close();
        },
      },
      {
        key: 'action:sidebar',
        group: 'actions',
        label: t('shell.action.toggleSidebar'),
        icon: PanelLeft,
        onSelect: () => {
          onToggleSidebar();
          close();
        },
      },
      {
        key: 'action:help',
        group: 'actions',
        label: t('shell.action.help'),
        icon: CircleHelp,
        onSelect: () => {
          close();
          onOpenHelp();
        },
      },
    ];
    actions.push(
      ...shellActions.filter((a) => !q || a.label.toLowerCase().includes(q.toLowerCase())),
    );
    result.push(...actions);
    return result;
  }, [
    query,
    t,
    navigate,
    workspaces,
    activeWorkspace,
    setActiveWorkspaceById,
    searchScope,
    setLogFilter,
    setGlobalSearch,
    theme,
    setTheme,
    onRefresh,
    onToggleSidebar,
    onOpenHelp,
    onOpenChange,
  ]);

  // Keep the highlighted row in view while arrowing through long lists.
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [active]);

  const groupLabels: Record<PaletteItem['group'], string> = {
    recent: t('shell.group.recent'),
    pages: t('shell.group.pages'),
    workspaces: t('shell.group.workspaces'),
    actions: t('shell.group.actions'),
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (items.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => (i + 1) % items.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => (i - 1 + items.length) % items.length);
    } else if (e.key === 'Home') {
      e.preventDefault();
      setActive(0);
    } else if (e.key === 'End') {
      e.preventDefault();
      setActive(items.length - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      items[active]?.onSelect();
    }
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-[60] bg-black/40 backdrop-blur-[2px] data-[state=open]:animate-fade-in" />
        <DialogPrimitive.Content
          aria-describedby={undefined}
          className="fixed left-1/2 top-[12vh] z-[61] w-[min(640px,calc(100vw-2rem))] -translate-x-1/2 overflow-hidden rounded-xl border border-border bg-popover text-popover-foreground shadow-2xl data-[state=open]:animate-fade-in"
        >
          <DialogPrimitive.Title className="sr-only">{t('shell.search')}</DialogPrimitive.Title>
          <div className="flex items-center gap-2.5 border-b border-border px-4">
            <Search className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden />
            <input
              id="global-search-input"
              autoFocus
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActive(0);
              }}
              onKeyDown={onKeyDown}
              placeholder={t('shell.searchPlaceholder')}
              role="combobox"
              aria-expanded
              aria-controls="command-palette-list"
              aria-activedescendant={items[active] ? `cmd-${items[active].key}` : undefined}
              autoComplete="off"
              spellCheck={false}
              className="h-12 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
            />
            <kbd className="hidden rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground sm:inline">
              Esc
            </kbd>
          </div>

          <div
            ref={listRef}
            id="command-palette-list"
            role="listbox"
            aria-label={t('shell.search')}
            className="max-h-[min(60vh,420px)] overflow-y-auto p-1.5"
          >
            {items.length === 0 ? (
              <p className="px-3 py-8 text-center text-sm text-muted-foreground">
                {t('shell.noResults')} &ldquo;{query.trim()}&rdquo;
              </p>
            ) : (
              items.map((item, index) => {
                const Icon = item.icon;
                const showHeading = index === 0 || items[index - 1].group !== item.group;
                return (
                  <React.Fragment key={item.key}>
                    {showHeading && (
                      <div
                        role="presentation"
                        className="px-2.5 pb-1 pt-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground"
                      >
                        {groupLabels[item.group]}
                      </div>
                    )}
                    <div
                      id={`cmd-${item.key}`}
                      data-index={index}
                      role="option"
                      aria-selected={index === active}
                      onMouseMove={() => setActive(index)}
                      onClick={item.onSelect}
                      className={cn(
                        'flex cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 text-sm',
                        index === active ? 'bg-muted text-foreground' : 'text-foreground/90',
                      )}
                    >
                      <Icon
                        className={cn(
                          'h-4 w-4 shrink-0',
                          index === active ? 'text-primary' : 'text-muted-foreground',
                        )}
                        aria-hidden
                      />
                      <span className="flex-1 truncate">{item.label}</span>
                      {item.hint && (
                        <span className="shrink-0 text-xs text-muted-foreground">{item.hint}</span>
                      )}
                      {index === active && (
                        <CornerDownLeft className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
                      )}
                    </div>
                  </React.Fragment>
                );
              })
            )}
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
