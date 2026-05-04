/**
 * Overview cards for pillar usage metrics
 */

import React from 'react';
import { Card } from '../ui/Card';
import { Sparkles, Shield, Code, Cloud, Brain } from 'lucide-react';
import type { PillarUsageMetrics } from '@/hooks/usePillarAnalytics';

interface PillarOverviewCardsProps {
  data: PillarUsageMetrics | null | undefined;
  isLoading?: boolean;
}

const pillars = [
  {
    id: 'reality',
    name: 'Reality',
    icon: Sparkles,
    color: 'text-purple-600 dark:text-purple-400',
    bgColor: 'bg-purple-50 dark:bg-purple-900/20',
    getValue: (data: PillarUsageMetrics | null | undefined) => {
      if (!data?.reality) return null;
      return `${data.reality.blended_reality_percent.toFixed(1)}%`;
    },
    getLabel: () => 'Blended Reality',
  },
  {
    id: 'contracts',
    name: 'Contracts',
    icon: Shield,
    color: 'text-info-600 dark:text-info-400',
    bgColor: 'bg-info-50 dark:bg-info-900/20',
    getValue: (data: PillarUsageMetrics | null | undefined) => {
      if (!data?.contracts) return null;
      return `${data.contracts.validation_enforce_percent.toFixed(1)}%`;
    },
    getLabel: () => 'Enforcement Mode',
  },
  {
    id: 'devx',
    name: 'DevX',
    icon: Code,
    color: 'text-success-600 dark:text-success-400',
    bgColor: 'bg-success-50 dark:bg-success-900/20',
    getValue: (data: PillarUsageMetrics | null | undefined) => {
      if (!data?.devx) return null;
      return data.devx.sdk_installations.toString();
    },
    getLabel: () => 'SDK Installations',
  },
  {
    id: 'cloud',
    name: 'Cloud',
    icon: Cloud,
    color: 'text-orange-600 dark:text-orange-400',
    bgColor: 'bg-orange-50 dark:bg-orange-900/20',
    getValue: (data: PillarUsageMetrics | null | undefined) => {
      if (!data?.cloud) return null;
      return data.cloud.shared_scenarios_count.toString();
    },
    getLabel: () => 'Shared Scenarios',
  },
  {
    id: 'ai',
    name: 'AI',
    icon: Brain,
    color: 'text-pink-600 dark:text-pink-400',
    bgColor: 'bg-pink-50 dark:bg-pink-900/20',
    getValue: (data: PillarUsageMetrics | null | undefined) => {
      if (!data?.ai) return null;
      return data.ai.ai_generated_mocks.toString();
    },
    getLabel: () => 'AI Mocks',
  },
];

export const PillarOverviewCards: React.FC<PillarOverviewCardsProps> = ({
  data,
  isLoading,
}) => {
  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        {pillars.map((pillar) => (
          <Card key={pillar.id} className="p-6 animate-pulse">
            <div className="h-4 bg-muted rounded w-24 mb-2"></div>
            <div className="h-8 bg-muted rounded w-16"></div>
          </Card>
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
      {pillars.map((pillar) => {
        const Icon = pillar.icon;
        const value = pillar.getValue(data);
        const label = pillar.getLabel();

        return (
          <Card
            key={pillar.id}
            className={`p-6 hover:shadow-lg transition-shadow ${pillar.bgColor}`}
          >
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-medium text-muted-foreground">
                {pillar.name}
              </h3>
              <Icon className={`h-5 w-5 ${pillar.color}`} />
            </div>
            <div className="space-y-1">
              <p className="text-2xl font-bold text-foreground">
                {value ?? 'N/A'}
              </p>
              <p className="text-xs text-muted-foreground">
                {label}
              </p>
            </div>
          </Card>
        );
      })}
    </div>
  );
};
