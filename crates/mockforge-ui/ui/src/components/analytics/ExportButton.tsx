/**
 * Export button for downloading analytics data
 */

import React, { useState } from 'react';
import { Download } from 'lucide-react';
import { exportToCSV, exportToJSON, downloadFile, type AnalyticsFilter } from '@/hooks/useAnalyticsV2';

interface ExportButtonProps {
  filter?: AnalyticsFilter;
}

export const ExportButton: React.FC<ExportButtonProps> = ({ filter }) => {
  const [isExporting, setIsExporting] = useState(false);
  const [showMenu, setShowMenu] = useState(false);

  const handleExportCSV = async () => {
    try {
      setIsExporting(true);
      const csv = await exportToCSV(filter);
      const timestamp = new Date().toISOString().split('T')[0];
      downloadFile(csv, `mockforge-analytics-${timestamp}.csv`, 'text/csv');
      setShowMenu(false);
    } catch (error) {
      console.error('Export failed:', error);
      alert('Failed to export data');
    } finally {
      setIsExporting(false);
    }
  };

  const handleExportJSON = async () => {
    try {
      setIsExporting(true);
      const json = await exportToJSON(filter);
      const timestamp = new Date().toISOString().split('T')[0];
      downloadFile(json, `mockforge-analytics-${timestamp}.json`, 'application/json');
      setShowMenu(false);
    } catch (error) {
      console.error('Export failed:', error);
      alert('Failed to export data');
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <div className="relative">
      <button
        onClick={() => setShowMenu(!showMenu)}
        disabled={isExporting}
        className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded-lg transition-colors"
      >
        <Download className="h-4 w-4" />
        {isExporting ? 'Exporting...' : 'Export'}
      </button>

      {showMenu && (
        <>
          <div
            className="fixed inset-0 z-10"
            onClick={() => setShowMenu(false)}
          />
          <div className="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-20">
            <button
              onClick={handleExportCSV}
              className="w-full px-4 py-2 text-left hover:bg-gray-100 dark:hover:bg-gray-700 rounded-t-lg"
            >
              Export as CSV
            </button>
            <button
              onClick={handleExportJSON}
              className="w-full px-4 py-2 text-left hover:bg-gray-100 dark:hover:bg-gray-700 rounded-b-lg"
            >
              Export as JSON
            </button>
          </div>
        </>
      )}
    </div>
  );
};
