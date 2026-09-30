import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useNavigate } from 'react-router-dom';
import { Check, ChevronsUpDown, FolderOpen, Settings2 } from 'lucide-react';
import { cn } from '../../utils/cn';
import { useI18n } from '../../i18n/I18nProvider';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';

export const menuContentClass =
  'z-[70] min-w-[14rem] overflow-hidden rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-lg data-[state=open]:animate-fade-in';
export const menuItemClass =
  'relative flex h-8 cursor-pointer select-none items-center gap-2 rounded-md px-2 text-sm outline-none data-[highlighted]:bg-muted data-[disabled]:pointer-events-none data-[disabled]:opacity-50';
export const menuLabelClass =
  'px-2 pb-1 pt-1.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground';

function WorkspaceMark({ name, className }: { name?: string; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn(
        'flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-primary/10 text-xs font-semibold text-primary ring-1 ring-inset ring-primary/20',
        className,
      )}
    >
      {name ? name.charAt(0).toUpperCase() : <FolderOpen className="h-3.5 w-3.5" />}
    </span>
  );
}

export function WorkspaceSwitcher({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const setActiveWorkspaceById = useWorkspaceStore((s) => s.setActiveWorkspaceById);

  const name = activeWorkspace?.name;

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        aria-label={t('workspace.selector.label')}
        title={collapsed ? name ?? t('shell.workspace.none') : undefined}
        className={cn(
          'flex w-full items-center gap-2.5 rounded-lg text-left outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring data-[state=open]:bg-muted',
          collapsed ? 'h-9 justify-center' : 'h-10 px-2',
        )}
      >
        <WorkspaceMark name={name} />
        {!collapsed && (
          <>
            <span className="min-w-0 flex-1">
              <span className="block text-[11px] leading-tight text-muted-foreground">
                {t('workspace.selector.label')}
              </span>
              <span className="block truncate text-sm font-medium leading-tight text-foreground">
                {name ?? t('shell.workspace.none')}
              </span>
            </span>
            <ChevronsUpDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden />
          </>
        )}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          side={collapsed ? 'right' : 'bottom'}
          sideOffset={6}
          className={cn(menuContentClass, 'w-[var(--radix-dropdown-menu-trigger-width)] min-w-[15rem]')}
        >
          <DropdownMenu.Label className={menuLabelClass}>{t('shell.group.workspaces')}</DropdownMenu.Label>
          {workspaces.length === 0 && (
            <p className="px-2 py-1.5 text-sm text-muted-foreground">{t('shell.workspace.none')}</p>
          )}
          <div className="max-h-64 overflow-y-auto">
            {workspaces.map((ws) => (
              <DropdownMenu.Item
                key={ws.id}
                className={menuItemClass}
                onSelect={() => {
                  if (ws.id !== activeWorkspace?.id) void setActiveWorkspaceById(ws.id);
                }}
              >
                <WorkspaceMark name={ws.name} className="h-5 w-5 text-[10px]" />
                <span className="flex-1 truncate">{ws.name}</span>
                {ws.id === activeWorkspace?.id && <Check className="h-4 w-4 text-primary" aria-hidden />}
              </DropdownMenu.Item>
            ))}
          </div>
          <DropdownMenu.Separator className="my-1 h-px bg-border" />
          <DropdownMenu.Item className={menuItemClass} onSelect={() => navigate('/workspaces')}>
            <Settings2 className="h-4 w-4 text-muted-foreground" aria-hidden />
            {t('shell.workspace.manage')}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
