import { useMemo } from 'react';
import { Bar } from 'react-chartjs-2';
import { getChartPalette } from '../../utils/chartTheme';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend,
  type ChartOptions,
  type TooltipItem,
} from 'chart.js';
import type { LatencyMetrics } from '../../types';

// Register Chart.js components
ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend
);

interface LatencyHistogramProps {
  metrics: LatencyMetrics[];
  selectedService?: string;
  onServiceChange: (service: string) => void;
}

export function LatencyHistogram({ metrics, selectedService, onServiceChange }: LatencyHistogramProps) {
  const selectedMetric = selectedService ? metrics.find(m => m.service === selectedService) : metrics[0];
  const histogramData = selectedMetric?.histogram || [];

  // Color bars based on latency ranges, reading from brand tokens.
  const palette = getChartPalette();
  const getBarColor = (range: string) => {
    const numValue = parseInt(range.split('-')[0] || '0');
    if (numValue < 100) return palette.success;
    if (numValue < 500) return palette.warning;
    return palette.danger;
  };

  const chartData = useMemo(() => ({
    labels: histogramData.map(d => d.range || ''),
    datasets: [
      {
        label: 'Request Count',
        data: histogramData.map(d => d.count || 0),
        backgroundColor: histogramData.map(d => getBarColor(d.range || '')),
        borderColor: histogramData.map(d => getBarColor(d.range || '')),
        borderWidth: 0,
        borderRadius: 3,
        maxBarThickness: 36,
      },
    ],
  }), [histogramData]);

  const chartOptions = useMemo((): ChartOptions<'bar'> => {
    const grid = { display: true, color: palette.border || 'rgba(0, 0, 0, 0.06)' };
    const ticks = { color: palette.mutedForeground || undefined, font: { size: 11 } };
    return {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          callbacks: {
            title: (items: TooltipItem<'bar'>[]) => `${items[0]?.label ?? ''} ms`,
            label: (context: TooltipItem<'bar'>) => `${context.parsed.y ?? 0} requests`,
          },
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: { ...ticks, maxRotation: 0, autoSkip: true },
          title: { display: true, text: 'Response time (ms)', color: ticks.color, font: { size: 11 } },
        },
        y: {
          beginAtZero: true,
          grid,
          border: { display: false },
          ticks: { ...ticks, precision: 0 },
          title: { display: true, text: 'Requests', color: ticks.color, font: { size: 11 } },
        },
      },
    };
    // Palette is read from CSS vars; recompute when the theme changes them.
  }, [palette.border, palette.mutedForeground]);

  // Some producers send `p50`, others only `p50_response_time`.
  const pct = (short?: number, long?: number) => {
    const v = short ?? long;
    return typeof v === 'number' && Number.isFinite(v) ? `${Math.round(v)}` : '—';
  };
  const percentiles = selectedMetric
    ? [
        { label: 'p50', value: pct(selectedMetric.p50, selectedMetric.p50_response_time) },
        { label: 'p95', value: pct(selectedMetric.p95, selectedMetric.p95_response_time) },
        { label: 'p99', value: pct(selectedMetric.p99, selectedMetric.p99_response_time) },
        { label: 'max', value: pct(undefined, selectedMetric.max_response_time) },
      ]
    : [];

  return (
    <div className="rounded-xl border border-border bg-card shadow-xs">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-3.5">
        <div>
          <h3 className="text-sm font-semibold text-foreground">Response time distribution</h3>
          {selectedMetric && (
            <p className="text-xs text-muted-foreground">
              {typeof selectedMetric.total_requests === 'number'
                ? `${selectedMetric.total_requests.toLocaleString()} requests sampled`
                : selectedMetric.service}
              {selectedMetric.route ? ` · ${selectedMetric.route}` : ''}
            </p>
          )}
        </div>
        <div className="flex items-center gap-4">
          <dl className="flex items-center gap-4">
            {percentiles.map((p) => (
              <div key={p.label} className="text-right">
                <dt className="text-[11px] uppercase tracking-wider text-muted-foreground">{p.label}</dt>
                <dd className="font-mono text-sm font-semibold tabular-nums text-foreground">
                  {p.value}
                  {p.value !== '—' && <span className="ml-0.5 text-xs font-normal text-muted-foreground">ms</span>}
                </dd>
              </div>
            ))}
          </dl>
          {metrics.length > 1 && (
            <select
              value={selectedService || ''}
              onChange={(e) => onServiceChange(e.target.value)}
              aria-label="Service"
              className="h-8 rounded-lg border border-input bg-background px-2 text-sm"
            >
              <option value="">All services</option>
              {metrics.map(metric => (
                <option key={metric.service} value={metric.service}>
                  {metric.service}
                </option>
              ))}
            </select>
          )}
        </div>
      </div>

      <div className="h-64 px-4 py-4">
        {histogramData.length > 0 ? (
          <Bar data={chartData} options={chartOptions} />
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
            No latency data yet — it appears once requests have been served.
          </div>
        )}
      </div>
    </div>
  );
}
