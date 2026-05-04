import React, { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Beaker, Save, CheckCircle2 } from 'lucide-react';
import { apiService } from '../../services/api';
import type {
  MockEnvironmentManagerResponse,
  MockEnvironmentResponse,
} from '../../types';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/Tabs';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { logger } from '@/utils/logger';

interface Props {
  workspaceId: string;
}

const ENV_NAMES = ['dev', 'test', 'prod'] as const;
type EnvName = (typeof ENV_NAMES)[number];

const formatJson = (value: Record<string, unknown> | null | undefined): string => {
  if (!value) return '';
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return '';
  }
};

const parseJson = (text: string): Record<string, unknown> | null | undefined => {
  const trimmed = text.trim();
  if (!trimmed) return null;
  return JSON.parse(trimmed) as Record<string, unknown>;
};

interface Drafts {
  reality: string;
  chaos: string;
  drift: string;
}

const draftsFromEnv = (env: MockEnvironmentResponse | undefined): Drafts => ({
  reality: formatJson(env?.reality_config),
  chaos: formatJson(env?.chaos_config),
  drift: formatJson(env?.drift_budget_config),
});

const WorkspaceMockEnvironmentsPanel: React.FC<Props> = ({ workspaceId }) => {
  const [data, setData] = useState<MockEnvironmentManagerResponse | null>(null);
  const [activeTab, setActiveTab] = useState<EnvName>('dev');
  const [drafts, setDrafts] = useState<Record<EnvName, Drafts>>({
    dev: { reality: '', chaos: '', drift: '' },
    test: { reality: '', chaos: '', drift: '' },
    prod: { reality: '', chaos: '', drift: '' },
  });
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const response = await apiService.listMockEnvironments(workspaceId);
      setData(response);
      const next: Record<EnvName, Drafts> = {
        dev: { reality: '', chaos: '', drift: '' },
        test: { reality: '', chaos: '', drift: '' },
        prod: { reality: '', chaos: '', drift: '' },
      };
      for (const env of response.environments) {
        const name = env.name.toLowerCase() as EnvName;
        if (ENV_NAMES.includes(name)) {
          next[name] = draftsFromEnv(env);
        }
      }
      setDrafts(next);
    } catch (err) {
      logger.error('Failed to load mock environments', err);
    }
  }, [workspaceId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleActivate = async (env: EnvName) => {
    try {
      await apiService.setActiveMockEnvironment(workspaceId, env);
      toast.success(`${env} activated`);
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to activate environment');
    }
  };

  const handleSave = async (env: EnvName) => {
    let parsed: { reality?: unknown; chaos?: unknown; drift?: unknown };
    try {
      parsed = {
        reality: parseJson(drafts[env].reality),
        chaos: parseJson(drafts[env].chaos),
        drift: parseJson(drafts[env].drift),
      };
    } catch (err) {
      toast.error(`Invalid JSON: ${err instanceof Error ? err.message : 'parse error'}`);
      return;
    }
    setSaving(true);
    try {
      await apiService.updateMockEnvironment(workspaceId, env, {
        reality_config: parsed.reality as Record<string, unknown> | null | undefined,
        chaos_config: parsed.chaos as Record<string, unknown> | null | undefined,
        drift_budget_config: parsed.drift as Record<string, unknown> | null | undefined,
      });
      toast.success(`${env} configuration saved`);
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to save configuration');
    } finally {
      setSaving(false);
    }
  };

  const updateDraft = (env: EnvName, field: keyof Drafts, value: string) => {
    setDrafts((prev) => ({ ...prev, [env]: { ...prev[env], [field]: value } }));
  };

  const isActive = (env: EnvName) => data?.active_environment?.toLowerCase() === env;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Beaker className="w-4 h-4" />
          Mock Environments
        </CardTitle>
        <CardDescription>
          Per-workspace dev/test/prod environments with reality, chaos, and drift-budget
          configuration. Switch the active mock environment to apply its policies to incoming
          requests.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as EnvName)}>
          <TabsList className="grid grid-cols-3 w-full">
            {ENV_NAMES.map((env) => (
              <TabsTrigger key={env} value={env} className="capitalize">
                {env}
                {isActive(env) && (
                  <Badge variant="secondary" className="ml-2 gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    Active
                  </Badge>
                )}
              </TabsTrigger>
            ))}
          </TabsList>
          {ENV_NAMES.map((env) => (
            <TabsContent key={env} value={env} className="space-y-4 pt-4">
              <div className="flex justify-end gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleActivate(env)}
                  disabled={isActive(env)}
                >
                  {isActive(env) ? 'Active' : 'Set Active'}
                </Button>
                <Button size="sm" onClick={() => handleSave(env)} disabled={saving}>
                  <Save className="w-4 h-4 mr-2" />
                  Save {env}
                </Button>
              </div>
              <div className="grid gap-4">
                <div>
                  <Label className="text-sm">Reality config (JSON)</Label>
                  <Textarea
                    value={drafts[env].reality}
                    onChange={(e) => updateDraft(env, 'reality', e.target.value)}
                    placeholder='{"level": 3}'
                    rows={6}
                    className="font-mono text-xs"
                  />
                </div>
                <div>
                  <Label className="text-sm">Chaos config (JSON)</Label>
                  <Textarea
                    value={drafts[env].chaos}
                    onChange={(e) => updateDraft(env, 'chaos', e.target.value)}
                    placeholder='{"enabled": false}'
                    rows={6}
                    className="font-mono text-xs"
                  />
                </div>
                <div>
                  <Label className="text-sm">Drift-budget config (JSON)</Label>
                  <Textarea
                    value={drafts[env].drift}
                    onChange={(e) => updateDraft(env, 'drift', e.target.value)}
                    placeholder='{"max_drift_pct": 5}'
                    rows={6}
                    className="font-mono text-xs"
                  />
                </div>
              </div>
            </TabsContent>
          ))}
        </Tabs>
      </CardContent>
    </Card>
  );
};

export default WorkspaceMockEnvironmentsPanel;
