import { logger } from '@/utils/logger';
import React, { useEffect, useState } from 'react';
import { RequestPanel } from '../components/playground/RequestPanel';
import { ResponsePanel } from '../components/playground/ResponsePanel';
import { HistoryPanel } from '../components/playground/HistoryPanel';
import { GraphQLIntrospection } from '../components/playground/GraphQLIntrospection';
import { CodeSnippetGenerator } from '../components/playground/CodeSnippetGenerator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/Tabs';
import { usePlaygroundStore } from '../stores/usePlaygroundStore';

/**
 * Playground Page
 *
 * Main page for the GraphQL + REST Playground feature.
 * Provides a three-panel layout:
 * - Left: Request builder (REST/GraphQL)
 * - Right: Response viewer
 * - Bottom: History panel
 */
export function PlaygroundPage() {
  const { loadEndpoints } = usePlaygroundStore();
  const [showHistory, setShowHistory] = useState(true);

  // Load endpoints on mount
  useEffect(() => {
    loadEndpoints();
  }, [loadEndpoints]);

  const { protocol } = usePlaygroundStore();

  return (
    <div className="h-full flex flex-col p-6 space-y-4">
      <div className="flex-1 grid grid-cols-2 gap-4 min-h-0">
        {/* Left Panel - Request */}
        <div className="flex flex-col min-h-0">
          <RequestPanel />
        </div>

        {/* Right Panel - Response */}
        <div className="flex flex-col min-h-0">
          <ResponsePanel />
        </div>
      </div>

      {/* Middle Panel - Additional Tools */}
      <div className="grid grid-cols-2 gap-4 h-80 flex-shrink-0">
        {/* GraphQL Introspection (only for GraphQL) */}
        {protocol === 'graphql' && (
          <div className="flex flex-col min-h-0">
            <GraphQLIntrospection />
          </div>
        )}

        {/* Code Snippet Generator */}
        <div className="flex flex-col min-h-0">
          <CodeSnippetGenerator />
        </div>
      </div>

      {/* Bottom Panel - History (collapsible) */}
      {showHistory && (
        <div className="h-64 flex-shrink-0">
          <HistoryPanel />
        </div>
      )}

      {/* Toggle History Button */}
      <div className="flex justify-center">
        <button
          onClick={() => setShowHistory(!showHistory)}
          className="text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          {showHistory ? 'Hide History' : 'Show History'}
        </button>
      </div>
    </div>
  );
}
