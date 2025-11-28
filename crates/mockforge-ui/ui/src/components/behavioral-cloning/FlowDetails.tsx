import React from 'react';
import { ArrowLeft, Tag, FileCode, Clock, GitBranch } from 'lucide-react';
import type { Flow, FlowStep } from '../../types';
import { ModernCard, ModernBadge, PageHeader } from '../ui/DesignSystem';
import { cn } from '../../utils/cn';

interface FlowDetailsProps {
  flow: Flow;
  onBack: () => void;
  onTag: () => void;
  onCompile: () => void;
}

export function FlowDetails({ flow, onBack, onTag, onCompile }: FlowDetailsProps) {
  const steps = flow.steps || [];

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center gap-4">
        <button
          onClick={onBack}
          className="p-2 hover:bg-muted rounded-md transition-colors"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <PageHeader
          title={flow.name || `Flow ${flow.id.slice(0, 8)}`}
          description={flow.description || 'Multi-step API flow'}
        />
      </div>

      {/* Flow Metadata */}
      <ModernCard className="p-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <div className="text-sm text-muted-foreground mb-1">Flow ID</div>
            <div className="font-mono text-sm">{flow.id}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground mb-1">Steps</div>
            <div className="font-semibold">{flow.step_count}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground mb-1">Created</div>
            <div className="text-sm">{new Date(flow.created_at).toLocaleString()}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground mb-1">Tags</div>
            <div className="flex gap-1 flex-wrap">
              {flow.tags && flow.tags.length > 0 ? (
                flow.tags.map((tag) => (
                  <ModernBadge key={tag} variant="secondary" size="sm">
                    {tag}
                  </ModernBadge>
                ))
              ) : (
                <span className="text-sm text-muted-foreground">None</span>
              )}
            </div>
          </div>
        </div>
        <div className="flex gap-2 mt-6">
          <button
            onClick={onTag}
            className="px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors flex items-center gap-2"
          >
            <Tag className="h-4 w-4" />
            Tag Flow
          </button>
          <button
            onClick={onCompile}
            className="px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors flex items-center gap-2"
          >
            <FileCode className="h-4 w-4" />
            Compile to Scenario
          </button>
        </div>
      </ModernCard>

      {/* Timeline */}
      <ModernCard className="p-6">
        <h2 className="text-lg font-semibold mb-4">Flow Timeline</h2>
        {steps.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No steps recorded in this flow
          </div>
        ) : (
          <div className="space-y-4">
            {steps.map((step, index) => (
              <FlowStepItem 
                key={step.request_id} 
                step={step} 
                index={index} 
                isLast={index === steps.length - 1}
              />
            ))}
          </div>
        )}
      </ModernCard>
    </div>
  );
}

function FlowStepItem({ step, index, isLast }: { step: FlowStep; index: number; isLast: boolean }) {
  return (
    <div className="flex gap-4">
      {/* Step Number */}
      <div className="flex flex-col items-center">
        <div className="w-8 h-8 rounded-full bg-primary text-primary-foreground flex items-center justify-center font-semibold text-sm">
          {index + 1}
        </div>
        {!isLast && (
          <div className="w-0.5 h-16 bg-border mt-2" />
        )}
      </div>

      {/* Step Content */}
      <div className="flex-1 pb-4">
        <div className="flex items-center gap-3 mb-2">
          <ModernBadge variant="outline">
            {step.step_label || `Step ${index + 1}`}
          </ModernBadge>
          {step.timing_ms !== undefined && step.timing_ms !== null && (
            <div className="flex items-center gap-1 text-sm text-muted-foreground">
              <Clock className="h-3 w-3" />
              <span>{step.timing_ms}ms</span>
            </div>
          )}
        </div>
        <div className="text-sm font-mono text-muted-foreground">
          Request ID: {step.request_id}
        </div>
      </div>
    </div>
  );
}

