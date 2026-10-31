import { Button } from '../ui/button';
import type { FidelityAssessment } from '../../services/api/fidelity';

type Definition = FidelityAssessment['definition']['routes'][number];
type Sample = NonNullable<FidelityAssessment['sample']>['routes'][number];

interface Props {
  definition?: Definition;
  sample?: Sample;
  open: boolean;
  onToggle: () => void;
}

export function FidelityRouteDetails({ definition, sample, open, onToggle }: Props) {
  const details = (definition?.deltas.length ?? 0) + (sample?.findings.length ?? 0) + (sample?.notes.length ?? 0);
  if (!details) return <span className="text-muted-foreground">No differences</span>;
  return <>
    <Button type="button" size="sm" variant="ghost" aria-expanded={open} onClick={onToggle}>{open ? 'Hide details' : 'View details'}</Button>
    {open && <div className="mt-2 max-w-xl space-y-3">
      {definition?.deltas.map((delta, index) => <div key={`${delta.location}-${index}`}>
        <p className="break-all font-mono text-xs">{delta.location || 'Entire operation'} ({delta.change})</p>
        <pre className="mt-1 max-h-64 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-muted p-2 text-xs">{JSON.stringify({ mock: delta.mock, source: delta.source }, null, 2)}</pre>
      </div>)}
      {sample?.findings.map((finding, i) => <p key={`finding-${i}`} className="break-words text-xs">{finding}</p>)}
      {sample?.notes.map((note, i) => <p key={`note-${i}`} className="break-words text-xs">{note}</p>)}
    </div>}
  </>;
}
