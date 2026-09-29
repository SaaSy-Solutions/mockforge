import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ArrowDown, ArrowUp, Edit3, Layers, Plus, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { Link } from 'react-router-dom';
import { PageHeader, Alert, EmptyState } from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import { Switch } from '../components/ui/switch';
import { Badge } from '../components/ui/Badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/Dialog';
import { isCloudMode } from '../utils/cloudMode';
import { overridesApi, type OverrideRule, type RuntimeSync } from '../services/api/overrides';
import { emptyDraft, fromDraft, toDraft, type PatchDraft, type RuleDraft } from './overrides/ruleDraft';

const SAVE_MESSAGES: Record<RuntimeSync, (count: number) => string> = {
  applied: (n) => `Saved ${n} rule${n === 1 ? '' : 's'}. The mock is using them now.`,
  outdated: () =>
    'Saved. This hosted mock runs an older MockForge version, so the rules apply after its next redeploy.',
  unreachable: () => 'Saved. The mock could not be reached; the rules apply when it next starts.',
};

const selectClass =
  'h-9 rounded-md border border-border bg-bg-primary px-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring';

function RuleSummary({ rule }: { rule: OverrideRule }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1.5">
      <span className="truncate font-medium text-foreground">{rule.name || 'Untitled rule'}</span>
      <div className="flex flex-wrap items-center gap-1.5">
        {rule.targets.map((target) => (
          <Badge key={target} variant="outline" className="font-mono text-xs">
            {target}
          </Badge>
        ))}
        {rule.when && (
          <Badge variant="info" className="font-mono text-xs">
            when {rule.when}
          </Badge>
        )}
        <span className="text-xs text-muted-foreground">
          {rule.patch.length} patch op{rule.patch.length === 1 ? '' : 's'}
          {rule.mode === 'merge' ? ', merge mode' : ''}
        </span>
      </div>
    </div>
  );
}

function PatchRow({
  op,
  index,
  onChange,
  onRemove,
}: {
  op: PatchDraft;
  index: number;
  onChange: (next: PatchDraft) => void;
  onRemove: () => void;
}) {
  return (
    <div className="space-y-2 rounded-md border border-border p-3">
      <div className="flex items-center gap-2">
        <select
          aria-label={`Patch ${index + 1} operation`}
          className={selectClass}
          value={op.op}
          onChange={(e) => onChange({ ...op, op: e.target.value as PatchDraft['op'] })}
        >
          <option value="replace">replace</option>
          <option value="add">add</option>
          <option value="remove">remove</option>
        </select>
        <Input
          aria-label={`Patch ${index + 1} path`}
          className="flex-1 font-mono"
          placeholder="/user/tier (empty = whole body)"
          value={op.path}
          onChange={(e) => onChange({ ...op, path: e.target.value })}
        />
        <Button variant="ghost" size="sm" onClick={onRemove} aria-label={`Remove patch ${index + 1}`}>
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
      {op.op !== 'remove' && (
        <Textarea
          aria-label={`Patch ${index + 1} value`}
          className="font-mono text-sm"
          rows={2}
          placeholder={'"gold"  or  {"id": "{{uuid}}"}  or  []'}
          value={op.value}
          onChange={(e) => onChange({ ...op, value: e.target.value })}
        />
      )}
    </div>
  );
}

function RuleEditor({
  initial,
  onCancel,
  onDone,
}: {
  initial: RuleDraft;
  onCancel: () => void;
  onDone: (rule: OverrideRule) => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const set = <K extends keyof RuleDraft>(key: K, value: RuleDraft[K]) =>
    setDraft((d) => ({ ...d, [key]: value }));

  const submit = () => {
    const result = fromDraft(draft);
    if ('error' in result) {
      setError(result.error);
      return;
    }
    onDone(result.rule);
  };

  return (
    <div className="space-y-4">
      <label className="block space-y-1">
        <span className="text-sm font-medium">Name</span>
        <Input value={draft.name} placeholder="VIP users get gold tier" onChange={(e) => set('name', e.target.value)} />
      </label>
      <label className="block space-y-1">
        <span className="text-sm font-medium">Targets, one per line</span>
        <Textarea
          className="font-mono text-sm"
          rows={3}
          value={draft.targets}
          placeholder={'operation:getUser\ntag:Users\npath:^/users/\n*'}
          onChange={(e) => set('targets', e.target.value)}
        />
        <span className="text-xs text-muted-foreground">
          operation:&lt;operationId&gt;, tag:&lt;OpenAPI tag&gt;, path:&lt;regex on the path template&gt;,
          regex:&lt;regex on the operationId&gt;, or * for everything.
        </span>
      </label>
      <label className="block space-y-1">
        <span className="text-sm font-medium">Only when (optional)</span>
        <Input
          className="font-mono"
          value={draft.when}
          placeholder="header[x-scenario]=vip"
          onChange={(e) => set('when', e.target.value)}
        />
        <span className="text-xs text-muted-foreground">
          header[name]=value, query[name]=value, $.request.body.field == 'x', AND(...), OR(...), NOT(...)
        </span>
      </label>
      <div className="space-y-2">
        <span className="text-sm font-medium">Patch the response body</span>
        {draft.patch.map((op, index) => (
          <PatchRow
            key={index}
            op={op}
            index={index}
            onChange={(next) => set('patch', draft.patch.map((p, i) => (i === index ? next : p)))}
            onRemove={() => set('patch', draft.patch.filter((_, i) => i !== index))}
          />
        ))}
        <Button
          variant="outline"
          size="sm"
          onClick={() => set('patch', [...draft.patch, { op: 'replace', path: '', value: '' }])}
        >
          <Plus className="mr-1 h-4 w-4" /> Add operation
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-6">
        <label className="flex items-center gap-2 text-sm">
          <span>Mode</span>
          <select className={selectClass} value={draft.mode} onChange={(e) => set('mode', e.target.value as RuleDraft['mode'])}>
            <option value="replace">replace</option>
            <option value="merge">merge (deep-merge objects)</option>
          </select>
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Switch checked={draft.postTemplating} onCheckedChange={(v) => set('postTemplating', v)} />
          Expand {'{{tokens}}'} on every response
        </label>
      </div>
      {error && <Alert type="error" title="Fix this rule" message={error} />}
      <DialogFooter>
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={submit}>Done</Button>
      </DialogFooter>
    </div>
  );
}

export function OverridesPage() {
  const cloud = isCloudMode();
  const queryClient = useQueryClient();
  const [deploymentId, setDeploymentId] = useState<string | null>(null);
  const [rules, setRules] = useState<OverrideRule[] | null>(null);
  const [editing, setEditing] = useState<{ index: number | null; draft: RuleDraft } | null>(null);

  const hostedMocks = useQuery({
    queryKey: ['overrides', 'hosted-mocks'],
    queryFn: overridesApi.listHostedMocks,
    enabled: cloud,
  });

  useEffect(() => {
    if (!cloud || deploymentId || !hostedMocks.data?.length) return;
    const active = hostedMocks.data.find((d) => d.status === 'active') ?? hostedMocks.data[0];
    setDeploymentId(active.id);
  }, [cloud, deploymentId, hostedMocks.data]);

  const ready = !cloud || deploymentId !== null;
  const saved = useQuery({
    queryKey: ['overrides', 'rules', deploymentId],
    queryFn: () => overridesApi.list(deploymentId),
    enabled: ready,
  });

  useEffect(() => {
    if (saved.data) setRules(saved.data);
  }, [saved.data]);

  const dirty = useMemo(
    () => rules !== null && saved.data !== undefined && JSON.stringify(rules) !== JSON.stringify(saved.data),
    [rules, saved.data],
  );

  const save = useMutation({
    mutationFn: (next: OverrideRule[]) => overridesApi.save(deploymentId, next),
    onSuccess: (result) => {
      queryClient.setQueryData(['overrides', 'rules', deploymentId], result.rules);
      setRules(result.rules);
      const message = SAVE_MESSAGES[result.runtime](result.rules.length);
      if (result.runtime === 'applied') toast.success(message);
      else toast.warning(message);
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : 'Failed to save rules'),
  });

  const update = (next: OverrideRule[]) => setRules(next);
  const move = (index: number, delta: number) => {
    if (!rules) return;
    const target = index + delta;
    if (target < 0 || target >= rules.length) return;
    const next = [...rules];
    [next[index], next[target]] = [next[target], next[index]];
    update(next);
  };

  const header = (
    <PageHeader
      title="Response Overrides"
      subtitle="Patch generated responses without editing your spec. Rules run top to bottom after the response is generated."
      action={
        <div className="flex items-center gap-2">
          {dirty && (
            <Button variant="outline" onClick={() => setRules(saved.data ?? [])} disabled={save.isPending}>
              Discard
            </Button>
          )}
          <Button onClick={() => rules && save.mutate(rules)} disabled={!dirty || save.isPending}>
            {save.isPending ? 'Saving…' : 'Save changes'}
          </Button>
        </div>
      }
    />
  );

  if (cloud && hostedMocks.isSuccess && hostedMocks.data.length === 0) {
    return (
      <div className="space-y-6 p-6">
        {header}
        <EmptyState
          icon={<Layers className="h-12 w-12" />}
          title="No hosted mocks yet"
          description="Overrides apply to a running mock. Deploy a hosted mock first, then come back to patch its responses."
          action={
            <Link to="/hosted-mocks">
              <Button>Go to Hosted Mocks</Button>
            </Link>
          }
        />
      </div>
    );
  }

  const loadError = hostedMocks.error ?? saved.error;

  return (
    <div className="space-y-6 p-6">
      {header}

      {cloud && (hostedMocks.data?.length ?? 0) > 0 && (
        <label className="flex items-center gap-2 text-sm">
          <span className="text-muted-foreground">Hosted mock</span>
          <select
            className={selectClass}
            value={deploymentId ?? ''}
            onChange={(e) => {
              if (dirty && !window.confirm('Discard unsaved rule changes?')) return;
              setRules(null);
              setDeploymentId(e.target.value);
            }}
          >
            {hostedMocks.data?.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.status})
              </option>
            ))}
          </select>
        </label>
      )}

      {loadError && (
        <Alert
          type="error"
          title="Could not load rules"
          message={loadError instanceof Error ? loadError.message : 'Request failed'}
        />
      )}

      {rules === null && !loadError && <p className="text-sm text-muted-foreground">Loading rules…</p>}

      {rules !== null && rules.length === 0 && (
        <EmptyState
          icon={<Layers className="h-12 w-12" />}
          title="No override rules"
          description="Add a rule to change a field, drop a key, or return a different body for matching requests."
          action={
            <Button onClick={() => setEditing({ index: null, draft: emptyDraft() })}>
              <Plus className="mr-1 h-4 w-4" /> Add rule
            </Button>
          }
        />
      )}

      {rules !== null && rules.length > 0 && (
        <div className="space-y-3">
          <ul className="divide-y divide-border rounded-xl border border-border bg-bg-primary">
            {rules.map((rule, index) => (
              <li key={index} className="flex items-center gap-4 px-4 py-3">
                <Switch
                  aria-label={`${rule.enabled ? 'Disable' : 'Enable'} ${rule.name || 'rule'}`}
                  checked={rule.enabled}
                  onCheckedChange={(enabled) =>
                    update(rules.map((r, i) => (i === index ? { ...r, enabled } : r)))
                  }
                />
                <RuleSummary rule={rule} />
                <div className="flex shrink-0 items-center gap-1">
                  <Button variant="ghost" size="sm" aria-label="Move up" disabled={index === 0} onClick={() => move(index, -1)}>
                    <ArrowUp className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label="Move down"
                    disabled={index === rules.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDown className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`Edit ${rule.name || 'rule'}`}
                    onClick={() => setEditing({ index, draft: toDraft(rule) })}
                  >
                    <Edit3 className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`Delete ${rule.name || 'rule'}`}
                    onClick={() => update(rules.filter((_, i) => i !== index))}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </li>
            ))}
          </ul>
          <Button variant="outline" onClick={() => setEditing({ index: null, draft: emptyDraft() })}>
            <Plus className="mr-1 h-4 w-4" /> Add rule
          </Button>
        </div>
      )}

      <Dialog open={editing !== null} onOpenChange={(open) => !open && setEditing(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{editing?.index === null ? 'Add rule' : 'Edit rule'}</DialogTitle>
            <DialogDescription>Changes are kept locally until you press Save changes.</DialogDescription>
          </DialogHeader>
          {editing && (
            <RuleEditor
              initial={editing.draft}
              onCancel={() => setEditing(null)}
              onDone={(rule) => {
                const current = rules ?? [];
                update(
                  editing.index === null
                    ? [...current, rule]
                    : current.map((r, i) => (i === editing.index ? rule : r)),
                );
                setEditing(null);
              }}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
