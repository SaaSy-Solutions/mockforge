import type { EndpointMetrics } from '@/stores/useAnalyticsStore';

/**
 * Convert data to CSV format and trigger download
 */
export const exportToCSV = (data: unknown[], filename: string) => {
  if (!data || data.length === 0) {
    console.warn('No data to export');
    return;
  }

  // Get headers from the first object
  const headers = Object.keys(data[0] as Record<string, unknown>);

  // Create CSV content
  const csvContent = [
    headers.join(','), // Header row
    ...data.map((row) =>
      headers
        .map((header) => {
          const value = (row as Record<string, unknown>)[header];
          // Escape values containing commas or quotes
          if (typeof value === 'string' && (value.includes(',') || value.includes('"'))) {
            return `"${value.replace(/"/g, '""')}"`;
          }
          return value;
        })
        .join(',')
    ),
  ].join('\n');

  // Create blob and download
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);

  link.setAttribute('href', url);
  link.setAttribute('download', filename);
  link.style.visibility = 'hidden';

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  URL.revokeObjectURL(url);
};

/**
 * Export endpoints data to CSV
 */
export const exportEndpointsToCSV = (endpoints: EndpointMetrics[]) => {
  const timestamp = new Date().toISOString().split('T')[0];
  exportToCSV(endpoints, `mockforge-endpoints-${timestamp}.csv`);
};

/**
 * Export summary data to JSON
 */
export const exportToJSON = (data: unknown, filename: string) => {
  const jsonContent = JSON.stringify(data, null, 2);
  const blob = new Blob([jsonContent], { type: 'application/json;charset=utf-8;' });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);

  link.setAttribute('href', url);
  link.setAttribute('download', filename);
  link.style.visibility = 'hidden';

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  URL.revokeObjectURL(url);
};

/**
 * Export all analytics data to JSON
 */
export const exportAllAnalyticsToJSON = (data: unknown) => {
  const timestamp = new Date().toISOString().split('T')[0];
  exportToJSON(data, `mockforge-analytics-${timestamp}.json`);
};
