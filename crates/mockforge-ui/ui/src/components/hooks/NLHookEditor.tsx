//! Natural Language Hook Editor Component
//!
//! This component allows users to describe hook logic in natural language
//! and see the transpiled hook configuration.

import React, { useState } from 'react';
import { Loader2, CheckCircle2, XCircle, Copy, Download, Code2 } from 'lucide-react';
import { Button } from '../ui/button';
import { cn } from '../../utils/cn';

interface NLHookEditorProps {
  onHookGenerated?: (hook: HookResult) => void;
  className?: string;
}

export interface HookResult {
  description: string;
  hookYaml?: string;
  hookJson?: any;
  error?: string;
}

export function NLHookEditor({ onHookGenerated, className }: NLHookEditorProps) {
  const [description, setDescription] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [result, setResult] = useState<HookResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showYaml, setShowYaml] = useState(true);

  const processDescription = async () => {
    if (!description.trim() || isProcessing) return;

    setIsProcessing(true);
    setError(null);
    setResult(null);

    try {
      const response = await fetch('/api/v2/voice/transpile-hook', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ description }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const responseData = await response.json();

      // Handle ApiResponse wrapper
      const data = responseData.data || responseData;

      const hookResult: HookResult = {
        description,
        hookYaml: data.hook_yaml || undefined,
        hookJson: data.hook_json || undefined,
        error: data.error || undefined,
      };

      setResult(hookResult);
      onHookGenerated?.(hookResult);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to transpile hook';
      setError(errorMessage);
      setResult({
        description,
        error: errorMessage,
      });
    } finally {
      setIsProcessing(false);
    }
  };

  const handleTextSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    processDescription();
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(
      () => {
        // Could show a toast notification here
        console.log('Copied to clipboard');
      },
      (err) => {
        console.error('Failed to copy:', err);
      }
    );
  };

  const downloadHook = (content: string, format: 'yaml' | 'json') => {
    const blob = new Blob([content], { type: format === 'yaml' ? 'text/yaml' : 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `hook.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <div className={cn('flex flex-col gap-4', className)}>
      <div className="space-y-2">
        <label htmlFor="hook-description" className="block text-sm font-medium text-gray-700">
          Describe your hook logic in natural language
        </label>
        <textarea
          id="hook-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="e.g., For users flagged as VIP, webhooks should fire instantly but payments fail 5% of the time"
          className="w-full min-h-[120px] px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          disabled={isProcessing}
        />
        <p className="text-xs text-gray-500">
          Describe conditions, actions, and timing constraints for your hook
        </p>
      </div>

      <div className="flex gap-2">
        <Button
          onClick={processDescription}
          disabled={!description.trim() || isProcessing}
          className="flex items-center gap-2"
        >
          {isProcessing ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Transpiling...
            </>
          ) : (
            <>
              <Code2 className="w-4 h-4" />
              Transpile Hook
            </>
          )}
        </Button>
      </div>

      {error && (
        <div className="p-4 bg-red-50 border border-red-200 rounded-md flex items-start gap-3">
          <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h3 className="text-sm font-medium text-red-800">Error</h3>
            <p className="text-sm text-red-700 mt-1">{error}</p>
          </div>
        </div>
      )}

      {result && !result.error && (result.hookYaml || result.hookJson) && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold text-gray-900">Transpiled Hook</h3>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowYaml(!showYaml)}
                className="flex items-center gap-2"
              >
                {showYaml ? 'Show JSON' : 'Show YAML'}
              </Button>
            </div>
          </div>

          <div className="relative">
            <pre className="p-4 bg-gray-50 border border-gray-200 rounded-md overflow-x-auto text-sm">
              <code>{showYaml ? result.hookYaml : JSON.stringify(result.hookJson, null, 2)}</code>
            </pre>
            <div className="absolute top-2 right-2 flex gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() =>
                  copyToClipboard(showYaml ? result.hookYaml! : JSON.stringify(result.hookJson, null, 2))
                }
                className="h-8 w-8 p-0"
                title="Copy to clipboard"
              >
                <Copy className="w-4 h-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() =>
                  downloadHook(
                    showYaml ? result.hookYaml! : JSON.stringify(result.hookJson, null, 2),
                    showYaml ? 'yaml' : 'json'
                  )
                }
                className="h-8 w-8 p-0"
                title="Download"
              >
                <Download className="w-4 h-4" />
              </Button>
            </div>
          </div>

          <div className="p-4 bg-green-50 border border-green-200 rounded-md flex items-start gap-3">
            <CheckCircle2 className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h3 className="text-sm font-medium text-green-800">Hook Generated Successfully</h3>
              <p className="text-sm text-green-700 mt-1">
                The hook configuration has been generated. You can copy it or download it to use in your chaos
                orchestration scenarios.
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

