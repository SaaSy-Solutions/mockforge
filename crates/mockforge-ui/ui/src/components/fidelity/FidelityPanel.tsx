import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { Input } from '../ui/input';
import { Section } from '../ui/design-system/PageHeader';
import { FidelitySummary } from './FidelitySummary';
import { useFidelityComparison } from './useFidelityComparison';

export function FidelityPanel() {
  const { mock, source, sample, report, error, pending, update, load, compare } = useFidelityComparison();
  return <Section title="Mock fidelity" subtitle="Compare the definition behind your mock with its current API contract.">
    <form onSubmit={compare} className="space-y-4" aria-busy={pending}>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="fidelity-mock">Mock definition</Label>
          <Input type="file" accept=".json,.yaml,.yml" aria-label="Load mock definition" onChange={e => void load(e.target.files?.[0], 'mock')} />
          <Textarea id="fidelity-mock" required rows={6} value={mock} onChange={e => update('mock', e.target.value)} placeholder="Paste the OpenAPI definition used to generate the mock" />
        </div>
        <div className="space-y-2">
          <Label htmlFor="fidelity-source">Current source contract</Label>
          <Input type="file" accept=".json,.yaml,.yml" aria-label="Load current source contract" onChange={e => void load(e.target.files?.[0], 'source')} />
          <Textarea id="fidelity-source" required rows={6} value={source} onChange={e => update('source', e.target.value)} placeholder="Paste the current upstream OpenAPI contract" />
        </div>
      </div>
      <p className="text-sm text-muted-foreground">OpenAPI or Swagger 2, JSON or YAML. Up to 2 MiB per input.</p>
      <details>
        <summary className="cursor-pointer text-sm font-medium focus-visible:outline focus-visible:outline-2">Include a recorded upstream sample</summary>
        <div className="mt-3 space-y-2">
          <Label htmlFor="fidelity-sample">Flight Recorder export (optional)</Label>
          <Input type="file" accept=".json,.jsonl" aria-label="Load recorded sample" onChange={e => void load(e.target.files?.[0], 'sample')} />
          <Textarea id="fidelity-sample" rows={4} value={sample} onChange={e => update('sample', e.target.value)} placeholder="Paste a JSONL export or JSON array of recorded exchanges" />
          <p className="text-sm text-muted-foreground">Use a fresh HTTP capture, up to 500 exchanges. Routes without traffic remain not sampled. Unsupported sample media remains not verified.</p>
        </div>
      </details>
      <Button type="submit" disabled={pending || !mock.trim() || !source.trim()}>{pending ? 'Comparing…' : 'Compare fidelity'}</Button>
      {pending && <p role="status" className="text-sm text-muted-foreground">Checking route definitions and recorded traffic…</p>}
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    </form>
    {report && <FidelitySummary report={report} />}
  </Section>;
}
