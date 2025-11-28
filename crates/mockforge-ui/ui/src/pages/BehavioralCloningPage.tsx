import React, { useState, useEffect } from 'react';
import { GitBranch, Play, Tag, FileCode, Download, Eye } from 'lucide-react';
import { apiService } from '../services/api';
import type { Flow, Scenario } from '../types';
import { FlowList } from '../components/behavioral-cloning/FlowList';
import { FlowDetails } from '../components/behavioral-cloning/FlowDetails';
import { ScenarioList } from '../components/behavioral-cloning/ScenarioList';
import { TagFlowModal } from '../components/behavioral-cloning/TagFlowModal';
import { CompileFlowModal } from '../components/behavioral-cloning/CompileFlowModal';
import {
  PageHeader,
  ModernCard,
  Alert,
  Section,
  ModernBadge,
} from '../components/ui/DesignSystem';
import { logger } from '../utils/logger';

type View = 'flows' | 'scenarios';

export function BehavioralCloningPage() {
  const [view, setView] = useState<View>('flows');
  const [flows, setFlows] = useState<Flow[]>([]);
  const [scenarios, setScenarios] = useState<Scenario[]>([]);
  const [selectedFlow, setSelectedFlow] = useState<Flow | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tagModalOpen, setTagModalOpen] = useState(false);
  const [compileModalOpen, setCompileModalOpen] = useState(false);
  const [flowToTag, setFlowToTag] = useState<Flow | null>(null);
  const [flowToCompile, setFlowToCompile] = useState<Flow | null>(null);

  useEffect(() => {
    if (view === 'flows') {
      loadFlows();
    } else {
      loadScenarios();
    }
  }, [view]);

  const loadFlows = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await apiService.getFlows({ limit: 100 });
      setFlows(response.flows);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load flows';
      setError(errorMessage);
      logger.error('Failed to load flows', { error: err });
    } finally {
      setLoading(false);
    }
  };

  const loadScenarios = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await apiService.getScenarios({ limit: 100 });
      setScenarios(response.scenarios);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load scenarios';
      setError(errorMessage);
      logger.error('Failed to load scenarios', { error: err });
    } finally {
      setLoading(false);
    }
  };

  const handleViewFlow = async (flow: Flow) => {
    try {
      const detailedFlow = await apiService.getFlow(flow.id);
      setSelectedFlow(detailedFlow);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load flow details';
      setError(errorMessage);
      logger.error('Failed to load flow details', { error: err });
    }
  };

  const handleTagFlow = (flow: Flow) => {
    setFlowToTag(flow);
    setTagModalOpen(true);
  };

  const handleCompileFlow = (flow: Flow) => {
    setFlowToCompile(flow);
    setCompileModalOpen(true);
  };

  const handleTagged = () => {
    setTagModalOpen(false);
    setFlowToTag(null);
    loadFlows();
  };

  const handleCompiled = () => {
    setCompileModalOpen(false);
    setFlowToCompile(null);
    loadFlows();
    loadScenarios();
  };

  if (selectedFlow) {
    return (
      <FlowDetails
        flow={selectedFlow}
        onBack={() => setSelectedFlow(null)}
        onTag={() => handleTagFlow(selectedFlow)}
        onCompile={() => handleCompileFlow(selectedFlow)}
      />
    );
  }

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Behavioral Cloning"
        description="Record multi-step API flows and replay them as named scenarios"
        icon={<GitBranch className="h-6 w-6" />}
      />

      {error && (
        <Alert variant="error" title="Error">
          {error}
        </Alert>
      )}

      {/* View Toggle */}
      <Section>
        <div className="flex gap-2 border-b">
          <button
            onClick={() => setView('flows')}
            className={`px-4 py-2 font-medium border-b-2 transition-colors ${
              view === 'flows'
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            <div className="flex items-center gap-2">
              <GitBranch className="h-4 w-4" />
              Flows ({flows.length})
            </div>
          </button>
          <button
            onClick={() => setView('scenarios')}
            className={`px-4 py-2 font-medium border-b-2 transition-colors ${
              view === 'scenarios'
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            <div className="flex items-center gap-2">
              <Play className="h-4 w-4" />
              Scenarios ({scenarios.length})
            </div>
          </button>
        </div>
      </Section>

      {/* Content */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <div className="text-muted-foreground">Loading...</div>
        </div>
      ) : view === 'flows' ? (
        <FlowList
          flows={flows}
          onView={handleViewFlow}
          onTag={handleTagFlow}
          onCompile={handleCompileFlow}
        />
      ) : (
        <ScenarioList scenarios={scenarios} onRefresh={loadScenarios} />
      )}

      {/* Modals */}
      {tagModalOpen && flowToTag && (
        <TagFlowModal
          flow={flowToTag}
          onClose={() => {
            setTagModalOpen(false);
            setFlowToTag(null);
          }}
          onTagged={handleTagged}
        />
      )}

      {compileModalOpen && flowToCompile && (
        <CompileFlowModal
          flow={flowToCompile}
          onClose={() => {
            setCompileModalOpen(false);
            setFlowToCompile(null);
          }}
          onCompiled={handleCompiled}
        />
      )}
    </div>
  );
}

