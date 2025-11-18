/**
 * Detailed view for Contracts pillar metrics
 */

import React from 'react';
import { Card } from '../ui/Card';
import { Shield, AlertTriangle, CheckCircle, XCircle } from 'lucide-react';
import type { ContractsPillarMetrics } from '@/hooks/usePillarAnalytics';

interface ContractsPillarDetailsProps {
  data: ContractsPillarMetrics;
  isLoading?: boolean;
  onSelect?: () => void;
  isSelected?: boolean;
}

export const ContractsPillarDetails: React.FC<ContractsPillarDetailsProps> = ({
  data,
  isLoading,
  onSelect,
  isSelected,
}) => {
  if (isLoading) {
    return (
      <Card className="p-6 animate-pulse">
        <div className="h-6 bg-gray-200 dark:bg-gray-700 rounded w-32 mb-4"></div>
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
      </Card>
    );
  }

  return (
    <Card
      className={`p-6 cursor-pointer transition-all ${
        isSelected
          ? 'ring-2 ring-blue-500 shadow-lg'
          : 'hover:shadow-md'
      }`}
      onClick={onSelect}
    >
      <div className="flex items-center gap-3 mb-4">
        <Shield className="h-6 w-6 text-blue-600 dark:text-blue-400" />
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
          Contracts Pillar
        </h3>
      </div>

      <div className="space-y-4">
        {/* Validation Modes */}
        <div>
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
            Validation Modes
          </h4>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  Enforce
                </span>
              </div>
              <span className="text-sm font-semibold text-gray-900 dark:text-white">
                {data.validation_enforce_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
              <div
                className="bg-green-600 h-2 rounded-full transition-all"
                style={{ width: `${data.validation_enforce_percent}%` }}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  Warn
                </span>
              </div>
              <span className="text-sm font-semibold text-gray-900 dark:text-white">
                {data.validation_warn_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
              <div
                className="bg-yellow-600 h-2 rounded-full transition-all"
                style={{ width: `${data.validation_warn_percent}%` }}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <XCircle className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  Disabled
                </span>
              </div>
              <span className="text-sm font-semibold text-gray-900 dark:text-white">
                {data.validation_disabled_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
              <div
                className="bg-gray-600 h-2 rounded-full transition-all"
                style={{ width: `${data.validation_disabled_percent}%` }}
              />
            </div>
          </div>
        </div>

        {/* Additional Metrics */}
        <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <p className="text-gray-500 dark:text-gray-500">Drift Budgets</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.drift_budget_configured_count}
              </p>
            </div>
            <div>
              <p className="text-gray-500 dark:text-gray-500">Drift Incidents</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.drift_incidents_count}
              </p>
            </div>
            <div>
              <p className="text-gray-500 dark:text-gray-500">Sync Cycles</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.contract_sync_cycles}
              </p>
            </div>
          </div>
        </div>
      </div>
    </Card>
  );
};
