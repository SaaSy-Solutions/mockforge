/**
 * Chart.js theme bridge — pipes design tokens (CSS custom properties on
 * <html>) into Chart.js global defaults so every chart in the app reads
 * brand colors instead of Chart.js's stock palette.
 *
 * Re-runs whenever the theme palette/dark mode changes (the palette store
 * sets new CSS variables on <html>; we observe via `MutationObserver` on
 * the documentElement style attribute and class list).
 */

import { Chart } from 'chart.js';

function readVar(name: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v ? `hsl(${v})` : '';
}

function readVarRGBA(name: string, alpha: number): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v ? `hsl(${v} / ${alpha})` : '';
}

/**
 * Returns the brand palette in tuple form so chart consumers that want
 * specific colors (e.g. one series per status) can pull them out.
 */
export function getChartPalette() {
  const primaryAlpha = (a: number) => readVarRGBA('--primary', a);
  const infoAlpha = (a: number) => readVarRGBA('--info', a);
  const successAlpha = (a: number) => readVarRGBA('--success', a);
  const warningAlpha = (a: number) => readVarRGBA('--warning', a);
  const dangerAlpha = (a: number) => readVarRGBA('--danger', a) || readVarRGBA('--destructive', a);

  return {
    primary: readVar('--primary'),
    primaryAlpha,
    info: readVar('--info'),
    infoAlpha,
    success: readVar('--success'),
    successAlpha,
    warning: readVar('--warning'),
    warningAlpha,
    danger: readVar('--danger') || readVar('--destructive'),
    dangerAlpha,
    foreground: readVar('--foreground'),
    mutedForeground: readVar('--muted-foreground'),
    border: readVar('--border'),
    background: readVar('--background'),
    card: readVar('--card'),
  };
}

/**
 * Stable n-color palette for chart series. Cycles through primary +
 * status tokens; suitable for protocol/category series where each
 * series needs a distinct color but exact mapping doesn't matter.
 */
export function getSeriesPalette(): { border: string; bg: string }[] {
  const t = getChartPalette();
  return [
    { border: t.primary, bg: t.primaryAlpha(0.1) },
    { border: t.info, bg: t.infoAlpha(0.1) },
    { border: t.success, bg: t.successAlpha(0.1) },
    { border: t.warning, bg: t.warningAlpha(0.1) },
    { border: t.danger, bg: t.dangerAlpha(0.1) },
  ];
}

function applyDefaults() {
  const t = getChartPalette();
  Chart.defaults.color = t.foreground || '#1f2937';
  Chart.defaults.borderColor = t.border || 'rgba(0, 0, 0, 0.1)';
  Chart.defaults.font.family =
    'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif';

  // Tooltip
  if (Chart.defaults.plugins?.tooltip) {
    Chart.defaults.plugins.tooltip.backgroundColor = t.card || '#fff';
    Chart.defaults.plugins.tooltip.titleColor = t.foreground || '#1f2937';
    Chart.defaults.plugins.tooltip.bodyColor = t.foreground || '#1f2937';
    Chart.defaults.plugins.tooltip.borderColor = t.border || 'rgba(0,0,0,0.1)';
    Chart.defaults.plugins.tooltip.borderWidth = 1;
    Chart.defaults.plugins.tooltip.padding = 10;
    Chart.defaults.plugins.tooltip.cornerRadius = 6;
  }

  // Legend / title
  if (Chart.defaults.plugins?.legend?.labels) {
    Chart.defaults.plugins.legend.labels.color = t.foreground || '#1f2937';
  }
  if (Chart.defaults.plugins?.title) {
    Chart.defaults.plugins.title.color = t.foreground || '#1f2937';
  }

  // Default dataset color (line/bar) — primary brand
  if (t.primary) {
    Chart.defaults.backgroundColor = t.primaryAlpha(0.12);
    Chart.defaults.borderColor = t.primary;
  }
}

let _applied = false;
let _observer: MutationObserver | null = null;

/**
 * Apply Chart.js theming once and start observing token changes. Idempotent.
 * Call from app entry (main.tsx).
 */
export function initChartTheme(): void {
  if (_applied) return;
  _applied = true;

  applyDefaults();

  // Re-apply when the palette or dark class changes — those mutations land
  // on <html> via the theme palette store.
  if (typeof MutationObserver !== 'undefined' && document.documentElement) {
    _observer = new MutationObserver(() => applyDefaults());
    _observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'style'],
    });
  }
}
