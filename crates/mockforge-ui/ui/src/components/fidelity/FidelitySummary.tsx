import { ModernBadge } from '../ui/design-system/Badge';
import type { FidelityAssessment } from '../../services/api/fidelity';
import { FidelityResults } from './FidelityResults';

export function FidelitySummary({ report }: { report: FidelityAssessment }) {
  const sampleDrift = report.sample?.routes.filter(route => route.status === 'drift-detected').length ?? 0;
  const sampleIncomplete = report.sample?.routes.filter(route => route.status !== 'verified' && route.status !== 'drift-detected').length ?? 0;

  return <div className="mt-6 space-y-3">
      <p role="status" className="text-sm">{report.definition.routes.length} route definitions compared. <ModernBadge variant={report.definition.routes_with_drift ? 'warning' : 'outline'}>{report.definition.routes_with_drift} definitions with drift</ModernBadge></p>
      <p className="text-sm text-muted-foreground">{report.sample
        ? `${report.sample.exchanges_checked} recorded exchanges checked; latest capture ${new Date(report.sample.latest_capture_at).toLocaleString()}. Results cover only the supplied definitions and sample.`
        : 'Definition comparison only. Include a recorder export to check observed upstream traffic.'}</p>
      {report.sample && <p role="status" className="flex flex-wrap gap-2 text-sm">
        {sampleDrift > 0 && <ModernBadge variant="error">Sample drift: {sampleDrift} routes</ModernBadge>}
        {sampleIncomplete > 0 && <ModernBadge variant="warning">Sample coverage incomplete: {sampleIncomplete} routes</ModernBadge>}
        {sampleDrift === 0 && sampleIncomplete === 0 && <ModernBadge variant="success">Sampled routes verified</ModernBadge>}
      </p>}
      <FidelityResults report={report} />
    </div>;
}
