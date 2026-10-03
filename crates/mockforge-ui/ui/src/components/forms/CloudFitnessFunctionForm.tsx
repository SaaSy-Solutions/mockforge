import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { fetchJsonWithErrorBody } from '../../services/api/client';
import { cloudContractApi, type CreateFitnessFunctionRequest } from '../../services/api/cloudContract';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
type Kind = 'latency_threshold' | 'error_rate' | 'contract_stability';
interface InitialFunction { name: string; description: string; config: Record<string, unknown>; function_type: { type: string }; enabled: boolean }
export function CloudFitnessFunctionForm({ initial, workspaceId, saving, onSave, onCancel }: {
  initial: InitialFunction | null; workspaceId: string; saving: boolean;
  onSave: (request: CreateFitnessFunctionRequest) => void; onCancel: () => void;
}) {
  const config = initial?.config ?? {};
  const [name, setName] = useState(initial?.name ?? '');
  const [description, setDescription] = useState(initial?.description ?? '');
  const [kind, setKind] = useState<Kind>((initial?.function_type.type as Kind) ?? 'latency_threshold');
  const [deploymentId, setDeploymentId] = useState(String(config.deployment_id ?? ''));
  const [serviceId, setServiceId] = useState(String(config.monitored_service_id ?? ''));
  const [thresholdMs, setThresholdMs] = useState(String(config.threshold_ms ?? 500));
  const [errorPercent, setErrorPercent] = useState(String(Number(config.threshold_rate ?? 0.01) * 100));
  const [maxBreaking, setMaxBreaking] = useState(String(config.max_breaking ?? 0));
  const [windowMinutes, setWindowMinutes] = useState(String(config.window_minutes ?? 5));
  const [percentile, setPercentile] = useState(String(config.percentile ?? 'p95'));
  const deployments = useQuery({ queryKey: ['cloud', 'fitness', 'deployments'],
    queryFn: () => fetchJsonWithErrorBody('/api/v1/hosted-mocks') as Promise<{ id: string; name: string }[]> });
  const services = useQuery({ queryKey: ['cloud', 'fitness', 'monitored-services', workspaceId],
    queryFn: () => cloudContractApi.listMonitoredServices(workspaceId), enabled: kind === 'contract_stability' && !!workspaceId });
  useEffect(() => { if (!deploymentId && deployments.data?.[0]) setDeploymentId(deployments.data[0].id); }, [deployments.data, deploymentId]);
  useEffect(() => { if (!serviceId && services.data?.[0]) setServiceId(services.data[0].id); }, [services.data, serviceId]);
  const error = deployments.error || services.error;
  const unavailable = kind === 'contract_stability' ? !serviceId : !deploymentId;
  const selectClass = 'w-full rounded-md border border-input bg-background px-3 py-2';
  return <form className="space-y-4" onSubmit={(event) => {
    event.preventDefault();
    if (unavailable || saving) return;
    const { function_type: _legacyType, ...existing } = config;
    onSave({ name: name.trim(), kind, config: { ...existing, description, enabled: initial?.enabled ?? true,
      scope: { type: 'workspace', workspace_id: workspaceId }, window_minutes: Number(windowMinutes),
      ...(kind === 'latency_threshold' ? { deployment_id: deploymentId, threshold_ms: Number(thresholdMs), percentile }
        : kind === 'error_rate' ? { deployment_id: deploymentId, threshold_rate: Number(errorPercent) / 100 }
        : { monitored_service_id: serviceId, max_breaking: Number(maxBreaking) }) } });
  }}>
    {error && <p role="alert" className="text-danger-600">{error.message}</p>}
    <div><Label htmlFor="cloud-fitness-name">Name</Label><Input id="cloud-fitness-name" required value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g., API Latency Budget" /></div>
    <div><Label htmlFor="cloud-fitness-description">Description</Label><Textarea id="cloud-fitness-description" value={description} onChange={(event) => setDescription(event.target.value)} placeholder="Describe what this fitness function checks..." /></div>
    <div><Label htmlFor="cloud-fitness-kind">Function Type</Label><select id="cloud-fitness-kind" className={selectClass} value={kind} onChange={(event) => setKind(event.target.value as Kind)}>
      <option value="latency_threshold">Latency threshold</option><option value="error_rate">Error rate</option><option value="contract_stability">Contract stability</option>
    </select></div>
    {kind === 'contract_stability' ? <>
      <div><Label htmlFor="cloud-fitness-service">Monitored service</Label><select id="cloud-fitness-service" required className={selectClass} value={serviceId} onChange={(event) => setServiceId(event.target.value)}><option value="">Select a monitored service</option>{services.data?.map((service) => <option key={service.id} value={service.id}>{service.name}</option>)}</select></div>
      <div><Label htmlFor="cloud-fitness-breaking">Maximum breaking changes</Label><Input id="cloud-fitness-breaking" type="number" min="0" step="1" required value={maxBreaking} onChange={(event) => setMaxBreaking(event.target.value)} /></div>
    </> : <div><Label htmlFor="cloud-fitness-deployment">Hosted deployment</Label><select id="cloud-fitness-deployment" required className={selectClass} value={deploymentId} onChange={(event) => setDeploymentId(event.target.value)}><option value="">Select a deployment</option>{deployments.data?.map((deployment) => <option key={deployment.id} value={deployment.id}>{deployment.name}</option>)}</select></div>}
    {kind === 'latency_threshold' && <>
      <div><Label htmlFor="cloud-fitness-latency">Maximum latency (ms)</Label><Input id="cloud-fitness-latency" type="number" min="0.1" step="any" required value={thresholdMs} onChange={(event) => setThresholdMs(event.target.value)} /></div>
      <div><Label htmlFor="cloud-fitness-percentile">Latency percentile</Label><select id="cloud-fitness-percentile" className={selectClass} value={percentile} onChange={(event) => setPercentile(event.target.value)}>{['p50', 'p95', 'p99', 'max', 'avg'].map((value) => <option key={value} value={value}>{value}</option>)}</select></div>
    </>}
    {kind === 'error_rate' && <div><Label htmlFor="cloud-fitness-rate">Maximum error rate (%)</Label><Input id="cloud-fitness-rate" type="number" min="0" max="100" step="any" required value={errorPercent} onChange={(event) => setErrorPercent(event.target.value)} /></div>}
    <div><Label htmlFor="cloud-fitness-window">Measurement window (minutes)</Label><Input id="cloud-fitness-window" type="number" min="1" max="1440" step="1" required value={windowMinutes} onChange={(event) => setWindowMinutes(event.target.value)} /></div>
    {unavailable && !error && <p role="status" className="text-muted-foreground">{kind === 'contract_stability' ? 'Add a monitored service in Cloud Contract to evaluate contract stability.' : deployments.isLoading ? 'Loading deployments…' : 'Deploy a hosted mock to measure latency or error rate.'}</p>}
    <div className="flex justify-end gap-2"><Button type="button" variant="outline" onClick={onCancel} disabled={saving}>Cancel</Button><Button type="submit" disabled={saving || unavailable || !name.trim()}>{saving ? 'Saving…' : `${initial ? 'Update' : 'Create'} Fitness Function`}</Button></div>
  </form>;
}
