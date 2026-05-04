/**
 * CloudAIQuotaBanner — read-only AI quota strip for cloud mode (#1).
 *
 * Calls GET /api/v1/ai-studio/quota and renders a one-line summary:
 * provider badge (BYOK / platform / disabled), used / limit, and a
 * subtle warning when the quota is exhausted. Used at the top of any
 * AI surface so the user sees their state before issuing a prompt.
 *
 * Hides itself in self-hosted mode.
 */
import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { Sparkles, Lock, AlertCircle } from 'lucide-react';
import { isCloudMode } from '../../utils/cloudMode';
import { aiStudioApi, type AiQuotaResponse } from '../../services/api/aiStudio';

export const CloudAIQuotaBanner: React.FC = () => {
    if (!isCloudMode()) return null;
    return <Banner />;
};

const Banner: React.FC = () => {
    const quotaQuery = useQuery({
        queryKey: ['cloud', 'ai-studio', 'quota'],
        queryFn: () => aiStudioApi.getQuota(),
        refetchInterval: 60_000,
    });

    if (quotaQuery.isLoading) {
        return (
            <div className="bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-700 rounded-lg p-3 mb-4 text-xs text-gray-500 italic">
                Loading AI quota…
            </div>
        );
    }
    if (quotaQuery.isError || !quotaQuery.data) {
        return null; // Soft fail — don't block the page if the endpoint is down.
    }

    const q: AiQuotaResponse = quotaQuery.data;

    if (q.provider === 'disabled' || !q.call_allowed) {
        return (
            <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-900/30 rounded-lg p-3 mb-4 text-sm text-yellow-800 dark:text-yellow-300 flex items-center gap-3">
                <AlertCircle className="w-4 h-4 shrink-0" />
                <div className="flex-1">
                    {q.provider === 'disabled'
                        ? 'AI is not available on this plan. Add a BYOK key in Settings or upgrade to Pro/Team.'
                        : 'Monthly AI quota exhausted. Upgrade your plan or add a BYOK key for unlimited usage.'}
                </div>
                <UsageDigest q={q} />
            </div>
        );
    }

    const Icon = q.provider === 'byok' ? Lock : Sparkles;
    return (
        <div className="bg-gradient-to-r from-blue-50 to-purple-50 dark:from-blue-900/20 dark:to-purple-900/20 border border-blue-200 dark:border-blue-900/30 rounded-lg p-3 mb-4 text-sm text-blue-900 dark:text-blue-100 flex items-center gap-3">
            <Icon className="w-4 h-4 shrink-0 text-blue-600 dark:text-blue-400" />
            <ProviderBadge provider={q.provider} />
            <span className="flex-1 text-blue-700 dark:text-blue-300">
                {q.provider === 'byok'
                    ? 'Calls are billed against your BYOK key — no platform quota.'
                    : 'Calls bill against the platform quota included with your plan.'}
            </span>
            <UsageDigest q={q} />
        </div>
    );
};

const ProviderBadge: React.FC<{ provider: AiQuotaResponse['provider'] }> = ({ provider }) => {
    const styles: Record<string, string> = {
        byok: 'bg-purple-100 text-purple-700 border-purple-200 dark:bg-purple-900/30 dark:text-purple-300 dark:border-purple-700',
        platform:
            'bg-blue-100 text-blue-700 border-blue-200 dark:bg-blue-900/30 dark:text-blue-300 dark:border-blue-700',
        disabled:
            'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600',
    };
    return (
        <span
            className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${styles[provider]}`}
        >
            {provider}
        </span>
    );
};

const UsageDigest: React.FC<{ q: AiQuotaResponse }> = ({ q }) => {
    const unlimited = q.tokens_limit === -1;
    return (
        <span className="text-xs font-mono whitespace-nowrap">
            {q.tokens_used_this_period.toLocaleString()}
            {' / '}
            {unlimited ? '∞' : q.tokens_limit.toLocaleString()} tokens
        </span>
    );
};
