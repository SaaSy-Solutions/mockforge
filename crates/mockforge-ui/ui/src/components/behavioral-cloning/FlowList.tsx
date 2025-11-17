import React from 'react';
import { Eye, Tag, FileCode, Clock, GitBranch } from 'lucide-react';
import type { Flow } from '../../types';
import { ModernCard, ModernBadge } from '../ui/DesignSystem';
import { cn } from '../../utils/cn';

interface FlowListProps {
  flows: Flow[];
  onView: (flow: Flow) => void;
  onTag: (flow: Flow) => void;
  onCompile: (flow: Flow) => void;
}

export function FlowList({ flows, onView, onTag, onCompile }: FlowListProps) {
  if (flows.length === 0) {
    return (
      <ModernCard className="p-12 text-center">
        <GitBranch className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
        <h3 className="text-lg font-semibold mb-2">No flows recorded yet</h3>
        <p className="text-muted-foreground">
          Start recording requests to capture multi-step flows automatically.
        </p>
      </ModernCard>
    );
  }

  return (
    <div className="space-y-4">
      {flows.map((flow) => (
        <ModernCard key={flow.id} className="p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-2">
                <h3 className="text-lg font-semibold">
                  {flow.name || `Flow ${flow.id.slice(0, 8)}`}
                </h3>
                {flow.tags && flow.tags.length > 0 && (
                  <div className="flex gap-2">
                    {flow.tags.map((tag) => (
                      <ModernBadge key={tag} variant="secondary" size="sm">
                        {tag}
                      </ModernBadge>
                    ))}
                  </div>
                )}
              </div>
              {flow.description && (
                <p className="text-sm text-muted-foreground mb-3">{flow.description}</p>
              )}
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <div className="flex items-center gap-1">
                  <GitBranch className="h-4 w-4" />
                  <span>{flow.step_count} steps</span>
                </div>
                <div className="flex items-center gap-1">
                  <Clock className="h-4 w-4" />
                  <span>{new Date(flow.created_at).toLocaleString()}</span>
                </div>
              </div>
            </div>
            <div className="flex gap-2 ml-4">
              <button
                onClick={() => onView(flow)}
                className="px-3 py-2 text-sm font-medium text-primary hover:bg-primary/10 rounded-md transition-colors flex items-center gap-2"
              >
                <Eye className="h-4 w-4" />
                View
              </button>
              <button
                onClick={() => onTag(flow)}
                className="px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded-md transition-colors flex items-center gap-2"
              >
                <Tag className="h-4 w-4" />
                Tag
              </button>
              <button
                onClick={() => onCompile(flow)}
                className="px-3 py-2 text-sm font-medium text-primary hover:bg-primary/10 rounded-md transition-colors flex items-center gap-2"
              >
                <FileCode className="h-4 w-4" />
                Compile
              </button>
            </div>
          </div>
        </ModernCard>
      ))}
    </div>
  );
}

