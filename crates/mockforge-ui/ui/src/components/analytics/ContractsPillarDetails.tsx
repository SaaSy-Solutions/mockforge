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
        <div className="h-6 bg-muted rounded w-32 mb-4"></div>
        <div className="space-y-2">
          <div className="h-4 bg-muted rounded"></div>
          <div className="h-4 bg-muted rounded"></div>
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
        <Shield className="h-6 w-6 text-info-600 dark:text-info-400" />
        <h3 className="text-lg font-semibold text-foreground">
          Contracts Pillar
        </h3>
      </div>

      <div className="space-y-4">
        {/* Validation Modes */}
        <div>
          <h4 className="text-sm font-medium text-foreground mb-3">
            Validation Modes
          </h4>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <CheckCircle className="h-4 w-4 text-success-600 dark:text-success-400" />
                <span className="text-sm text-muted-foreground">
                  Enforce
                </span>
              </div>
              <span className="text-sm font-semibold text-foreground">
                {data.validation_enforce_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div
                className="bg-success-600 h-2 rounded-full transition-all"
                style={{ width: `${data.validation_enforce_percent}%` }}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <AlertTriangle className="h-4 w-4 text-warning-600 dark:text-warning-400" />
                <span className="text-sm text-muted-foreground">
                  Warn
                </span>
              </div>
              <span className="text-sm font-semibold text-foreground">
                {data.validation_warn_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div
                className="bg-warning h-2 rounded-full transition-all"
                style={{ width: `${data.validation_warn_percent}%` }}
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <XCircle className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">
                  Disabled
                </span>
              </div>
              <span className="text-sm font-semibold text-foreground">
                {data.validation_disabled_percent.toFixed(1)}%
              </span>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div
                className="bg-gray-600 h-2 rounded-full transition-all"
                style={{ width: `${data.validation_disabled_percent}%` }}
              />
            </div>
          </div>
        </div>

        {/* Additional Metrics */}
        <div className="pt-4 border-t border-border">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <p className="text-muted-foreground">Drift Budgets</p>
              <p className="font-semibold text-foreground">
                {data.drift_budget_configured_count}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground">Drift Incidents</p>
              <p className="font-semibold text-foreground">
                {data.drift_incidents_count}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground">Sync Cycles</p>
              <p className="font-semibold text-foreground">
                {data.contract_sync_cycles}
              </p>
            </div>
          </div>
        </div>
      </div>
    </Card>
  );
};
