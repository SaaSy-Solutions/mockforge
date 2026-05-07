/**
 * Cloud Plugins — manage plugins attached to hosted-mock deployments.
 *
 * Phase 3.1 scaffold (read-only). Lists each deployment in the org with
 * its attached plugins. Attach / permission-edit / detach land in 3.2 –
 * 3.4. The control-plane endpoints come from PR #395; until that merges
 * `listAttachments` returns [] for every deployment and the page shows
 * the empty state.
 */
import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { Puzzle, ExternalLink, AlertCircle } from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { authenticatedFetch } from '../utils/apiClient';
import {
  cloudPluginsApi,
  type PluginAttachment,
} from '../services/api/cloudPlugins';
import { useI18n } from '../i18n/I18nProvider';

interface DeploymentSummary {
  id: string;
  name: string;
  slug: string;
  status: string;
  region?: string;
}

async function fetchDeployments(): Promise<DeploymentSummary[]> {
  const response = await authenticatedFetch('/api/v1/hosted-mocks');
  if (!response.ok) {
    throw new Error(`Failed to load deployments (HTTP ${response.status})`);
  }
  const data = await response.json();
  const list = (data?.data ?? data) as unknown;
  return Array.isArray(list) ? (list as DeploymentSummary[]) : [];
}

export const CloudPluginsPage: React.FC = () => {
  if (!isCloudMode()) {
    return <LocalModeNotice />;
  }
  return <CloudView />;
};

const LocalModeNotice: React.FC = () => {
  const navigate = useNavigate();
  return (
    <div className="p-6 max-w-7xl mx-auto">
      <div className="rounded-lg border border-blue-200 bg-blue-50 p-4 text-sm text-blue-800 dark:border-blue-900 dark:bg-blue-900/20 dark:text-blue-300">
        Cloud plugin management only applies to cloud-hosted deployments. For
        plugins on a local MockForge runtime, open the{' '}
        <button
          type="button"
          className="font-medium underline"
          onClick={() => navigate('/plugins')}
        >
          local Plugins page
        </button>
        .
      </div>
    </div>
  );
};

const CloudView: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const deploymentsQuery = useQuery({
    queryKey: ['cloud', 'plugins', 'deployments'],
    queryFn: fetchDeployments,
  });

  return (
    <div className="p-6 max-w-7xl mx-auto space-y-6">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold flex items-center gap-2">
            <Puzzle className="w-6 h-6" />
            {t('page.cloudPlugins.title')}
          </h1>
          <p className="text-sm text-muted-foreground mt-1 max-w-3xl">
            {t('page.cloudPlugins.subtitle')}
          </p>
        </div>
        <button
          type="button"
          onClick={() => navigate('/plugin-registry')}
          className="inline-flex items-center gap-1.5 rounded-md border border-input bg-background px-3 py-2 text-sm hover:bg-accent"
        >
          {t('page.cloudPlugins.browseRegistry')}
          <ExternalLink className="w-4 h-4" />
        </button>
      </header>

      <BetaBanner />

      {deploymentsQuery.isLoading && (
        <div className="text-sm text-muted-foreground">
          {t('app.loading')}
        </div>
      )}

      {deploymentsQuery.isError && (
        <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800 dark:border-red-900 dark:bg-red-900/20 dark:text-red-300">
          <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
          <span>{(deploymentsQuery.error as Error).message}</span>
        </div>
      )}

      {deploymentsQuery.data && deploymentsQuery.data.length === 0 && (
        <EmptyState onCreate={() => navigate('/hosted-mocks')} />
      )}

      {deploymentsQuery.data && deploymentsQuery.data.length > 0 && (
        <div className="space-y-4">
          {deploymentsQuery.data.map((deployment) => (
            <DeploymentRow key={deployment.id} deployment={deployment} />
          ))}
        </div>
      )}
    </div>
  );
};

const BetaBanner: React.FC = () => (
  <div className="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900 dark:border-amber-900 dark:bg-amber-900/20 dark:text-amber-200">
    <p className="font-medium">Cloud plugin runtime — beta</p>
    <p className="mt-1">
      Cloud plugin management is rolling out alongside the cloud plugin
      runtime. Attach, permission, and detach controls land in the next
      sub-PRs of Phase 3. Today this page lists deployments and any plugins
      already wired up.
    </p>
  </div>
);

const EmptyState: React.FC<{ onCreate: () => void }> = ({ onCreate }) => (
  <div className="rounded-lg border border-dashed border-input p-8 text-center">
    <Puzzle className="w-8 h-8 mx-auto text-muted-foreground" />
    <h3 className="mt-3 font-medium">No hosted-mock deployments yet</h3>
    <p className="mt-1 text-sm text-muted-foreground">
      Create a hosted mock first — plugins attach per deployment.
    </p>
    <button
      type="button"
      onClick={onCreate}
      className="mt-4 inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground hover:bg-primary/90"
    >
      Go to Hosted Mocks
    </button>
  </div>
);

const DeploymentRow: React.FC<{ deployment: DeploymentSummary }> = ({
  deployment,
}) => {
  const attachmentsQuery = useQuery({
    queryKey: ['cloud', 'plugins', 'attachments', deployment.id],
    queryFn: () => cloudPluginsApi.listAttachments(deployment.id),
  });

  return (
    <section className="rounded-lg border border-input bg-card">
      <header className="flex items-center justify-between border-b border-input px-4 py-3">
        <div>
          <h2 className="font-medium">{deployment.name}</h2>
          <p className="text-xs text-muted-foreground">
            {deployment.slug}
            {deployment.region ? ` · ${deployment.region}` : ''} ·{' '}
            <StatusBadge status={deployment.status} />
          </p>
        </div>
        <span className="text-xs text-muted-foreground">
          {attachmentsQuery.data?.length ?? 0} plugin
          {(attachmentsQuery.data?.length ?? 0) === 1 ? '' : 's'}
        </span>
      </header>

      {attachmentsQuery.isLoading && (
        <p className="px-4 py-3 text-xs text-muted-foreground">Loading…</p>
      )}
      {attachmentsQuery.isError && (
        <p className="px-4 py-3 text-xs text-red-700 dark:text-red-400">
          {(attachmentsQuery.error as Error).message}
        </p>
      )}
      {attachmentsQuery.data && attachmentsQuery.data.length === 0 && (
        <p className="px-4 py-3 text-xs text-muted-foreground">
          No plugins attached to this deployment.
        </p>
      )}
      {attachmentsQuery.data && attachmentsQuery.data.length > 0 && (
        <ul className="divide-y divide-input">
          {attachmentsQuery.data.map((a) => (
            <AttachmentRow key={a.id} attachment={a} />
          ))}
        </ul>
      )}
    </section>
  );
};

const AttachmentRow: React.FC<{ attachment: PluginAttachment }> = ({
  attachment,
}) => (
  <li className="flex items-center justify-between gap-4 px-4 py-3 text-sm">
    <div className="min-w-0">
      <p className="font-medium truncate">
        {attachment.plugin_name ?? attachment.plugin_id}
      </p>
      <p className="text-xs text-muted-foreground truncate">
        v{attachment.plugin_version ?? '—'} · attached{' '}
        {new Date(attachment.attached_at).toLocaleString()}
      </p>
    </div>
    <span
      className={
        attachment.enabled
          ? 'inline-flex items-center rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300'
          : 'inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground'
      }
    >
      {attachment.enabled ? 'Enabled' : 'Disabled'}
    </span>
  </li>
);

const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
  const tone =
    status === 'active'
      ? 'text-emerald-700 dark:text-emerald-400'
      : status === 'failed'
        ? 'text-red-700 dark:text-red-400'
        : 'text-muted-foreground';
  return <span className={tone}>{status}</span>;
};
