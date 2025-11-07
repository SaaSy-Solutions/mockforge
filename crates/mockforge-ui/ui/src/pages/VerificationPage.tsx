import { logger } from '@/utils/logger';
import React, { useState } from 'react';
import { CheckCircle2, XCircle, Search, Play, RefreshCw, AlertCircle } from 'lucide-react';
import { verificationApi } from '../services/api';
import type { VerificationRequest, VerificationCount, VerificationResult } from '../types';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Alert,
  EmptyState,
  Section
} from '../components/ui/DesignSystem';
import { Input } from '../components/ui/input';
import { Button } from '../components/ui/button';
import { Label } from '../components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { Textarea } from '../components/ui/textarea';

type VerificationMode = 'verify' | 'never' | 'at-least' | 'sequence';

export function VerificationPage() {
  const [mode, setMode] = useState<VerificationMode>('verify');
  const [pattern, setPattern] = useState<VerificationRequest>({
    method: '',
    path: '',
    query_params: {},
    headers: {},
    body_pattern: '',
  });
  const [expectedCount, setExpectedCount] = useState<VerificationCount>({ type: 'exactly', value: 1 });
  const [minCount, setMinCount] = useState(1);
  const [sequencePatterns, setSequencePatterns] = useState<VerificationRequest[]>([
    { method: '', path: '', query_params: {}, headers: {}, body_pattern: '' }
  ]);
  const [result, setResult] = useState<VerificationResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleVerify = async () => {
    setIsLoading(true);
    setError(null);
    setResult(null);

    try {
      let verificationResult: VerificationResult;

      switch (mode) {
        case 'verify':
          verificationResult = await verificationApi.verify(pattern, expectedCount);
          break;
        case 'never':
          verificationResult = await verificationApi.verifyNever(pattern);
          break;
        case 'at-least':
          verificationResult = await verificationApi.verifyAtLeast(pattern, minCount);
          break;
        case 'sequence':
          verificationResult = await verificationApi.verifySequence(sequencePatterns);
          break;
        default:
          throw new Error('Invalid verification mode');
      }

      setResult(verificationResult);
    } catch (err: any) {
      logger.error('Verification failed', err);
      setError(err.message || 'Verification request failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCount = async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await verificationApi.count(pattern);
      setResult({
        matched: true,
        count: response.count,
        expected: { type: 'exactly', value: response.count },
        matches: [],
      });
    } catch (err: any) {
      logger.error('Count request failed', err);
      setError(err.message || 'Count request failed');
    } finally {
      setIsLoading(false);
    }
  };

  const addSequencePattern = () => {
    setSequencePatterns([...sequencePatterns, { method: '', path: '', query_params: {}, headers: {}, body_pattern: '' }]);
  };

  const removeSequencePattern = (index: number) => {
    setSequencePatterns(sequencePatterns.filter((_, i) => i !== index));
  };

  const updateSequencePattern = (index: number, field: keyof VerificationRequest, value: string) => {
    const updated = [...sequencePatterns];
    updated[index] = { ...updated[index], [field]: value };
    setSequencePatterns(updated);
  };

  return (
    <div className="space-y-8">
      <PageHeader
        title="Request Verification"
        subtitle="Verify that specific requests were made (or not made) during test execution"
      />

      <Section>
        <ModernCard>
          <div className="space-y-6">
            {/* Mode Selection */}
            <div>
              <Label htmlFor="mode">Verification Mode</Label>
              <Select value={mode} onValueChange={(value) => setMode(value as VerificationMode)}>
                <SelectTrigger id="mode">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="verify">Verify Count</SelectItem>
                  <SelectItem value="never">Verify Never</SelectItem>
                  <SelectItem value="at-least">Verify At Least</SelectItem>
                  <SelectItem value="sequence">Verify Sequence</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Pattern Configuration */}
            {mode !== 'sequence' ? (
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label htmlFor="method">HTTP Method (optional)</Label>
                    <Input
                      id="method"
                      placeholder="GET, POST, etc."
                      value={pattern.method || ''}
                      onChange={(e) => setPattern({ ...pattern, method: e.target.value || undefined })}
                    />
                  </div>
                  <div>
                    <Label htmlFor="path">Path Pattern (optional)</Label>
                    <Input
                      id="path"
                      placeholder="/api/users or /api/users/*"
                      value={pattern.path || ''}
                      onChange={(e) => setPattern({ ...pattern, path: e.target.value || undefined })}
                    />
                  </div>
                </div>

                <div>
                  <Label htmlFor="body-pattern">Body Pattern (optional, supports regex)</Label>
                  <Textarea
                    id="body-pattern"
                    placeholder='{"name":".*"}'
                    value={pattern.body_pattern || ''}
                    onChange={(e) => setPattern({ ...pattern, body_pattern: e.target.value || undefined })}
                    rows={3}
                  />
                </div>

                {mode === 'verify' && (
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <Label htmlFor="count-type">Count Type</Label>
                      <Select
                        value={expectedCount.type}
                        onValueChange={(value) => {
                          if (value === 'never' || value === 'at_least_once') {
                            setExpectedCount({ type: value } as VerificationCount);
                          } else {
                            setExpectedCount({ type: value, value: 1 } as VerificationCount);
                          }
                        }}
                      >
                        <SelectTrigger id="count-type">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="exactly">Exactly</SelectItem>
                          <SelectItem value="at_least">At Least</SelectItem>
                          <SelectItem value="at_most">At Most</SelectItem>
                          <SelectItem value="never">Never</SelectItem>
                          <SelectItem value="at_least_once">At Least Once</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    {(expectedCount.type === 'exactly' || expectedCount.type === 'at_least' || expectedCount.type === 'at_most') && (
                      <div>
                        <Label htmlFor="count-value">Count Value</Label>
                        <Input
                          id="count-value"
                          type="number"
                          min="0"
                          value={expectedCount.type !== 'never' && expectedCount.type !== 'at_least_once' ? (expectedCount as any).value || 0 : ''}
                          onChange={(e) => {
                            const value = parseInt(e.target.value, 10);
                            if (!isNaN(value)) {
                              setExpectedCount({ ...expectedCount, value } as VerificationCount);
                            }
                          }}
                        />
                      </div>
                    )}
                  </div>
                )}

                {mode === 'at-least' && (
                  <div>
                    <Label htmlFor="min-count">Minimum Count</Label>
                    <Input
                      id="min-count"
                      type="number"
                      min="0"
                      value={minCount}
                      onChange={(e) => setMinCount(parseInt(e.target.value, 10) || 0)}
                    />
                  </div>
                )}
              </div>
            ) : (
              <div className="space-y-4">
                <div className="flex justify-between items-center">
                  <Label>Request Sequence Patterns</Label>
                  <Button type="button" variant="outline" size="sm" onClick={addSequencePattern}>
                    Add Pattern
                  </Button>
                </div>
                {sequencePatterns.map((seqPattern, index) => (
                  <ModernCard key={index} className="p-4">
                    <div className="space-y-3">
                      <div className="flex justify-between items-center">
                        <span className="text-sm font-medium">Pattern {index + 1}</span>
                        {sequencePatterns.length > 1 && (
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            onClick={() => removeSequencePattern(index)}
                          >
                            Remove
                          </Button>
                        )}
                      </div>
                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <Label htmlFor={`seq-method-${index}`}>Method</Label>
                          <Input
                            id={`seq-method-${index}`}
                            placeholder="GET, POST, etc."
                            value={seqPattern.method || ''}
                            onChange={(e) => updateSequencePattern(index, 'method', e.target.value)}
                          />
                        </div>
                        <div>
                          <Label htmlFor={`seq-path-${index}`}>Path</Label>
                          <Input
                            id={`seq-path-${index}`}
                            placeholder="/api/users"
                            value={seqPattern.path || ''}
                            onChange={(e) => updateSequencePattern(index, 'path', e.target.value)}
                          />
                        </div>
                      </div>
                    </div>
                  </ModernCard>
                ))}
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex gap-3">
              <Button
                onClick={handleVerify}
                disabled={isLoading}
                className="flex items-center gap-2"
              >
                {isLoading ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : (
                  <Play className="h-4 w-4" />
                )}
                {mode === 'never' ? 'Verify Never' : mode === 'at-least' ? 'Verify At Least' : mode === 'sequence' ? 'Verify Sequence' : 'Verify'}
              </Button>
              {mode !== 'sequence' && (
                <Button
                  onClick={handleCount}
                  disabled={isLoading}
                  variant="outline"
                  className="flex items-center gap-2"
                >
                  <Search className="h-4 w-4" />
                  Get Count
                </Button>
              )}
            </div>
          </div>
        </ModernCard>
      </Section>

      {/* Error Display */}
      {error && (
        <Alert variant="error" className="flex items-center gap-2">
          <AlertCircle className="h-5 w-5" />
          <span>{error}</span>
        </Alert>
      )}

      {/* Result Display */}
      {result && (
        <Section>
          <ModernCard>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold">Verification Result</h3>
                {result.matched ? (
                  <ModernBadge variant="success" className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4" />
                    Passed
                  </ModernBadge>
                ) : (
                  <ModernBadge variant="error" className="flex items-center gap-2">
                    <XCircle className="h-4 w-4" />
                    Failed
                  </ModernBadge>
                )}
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Actual Count</Label>
                  <div className="text-2xl font-bold">{result.count}</div>
                </div>
                <div>
                  <Label>Expected</Label>
                  <div className="text-sm">
                    {result.expected.type === 'exactly' && `Exactly ${(result.expected as any).value}`}
                    {result.expected.type === 'at_least' && `At least ${(result.expected as any).value}`}
                    {result.expected.type === 'at_most' && `At most ${(result.expected as any).value}`}
                    {result.expected.type === 'never' && 'Never'}
                    {result.expected.type === 'at_least_once' && 'At least once'}
                  </div>
                </div>
              </div>

              {result.error_message && (
                <Alert variant="error">
                  <AlertCircle className="h-5 w-5" />
                  <span>{result.error_message}</span>
                </Alert>
              )}

              {result.matches && result.matches.length > 0 && (
                <div>
                  <Label>Matching Requests ({result.matches.length})</Label>
                  <div className="mt-2 space-y-2 max-h-96 overflow-y-auto">
                    {result.matches.map((match, index) => (
                      <ModernCard key={index} className="p-3">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-3">
                            <ModernBadge variant={match.status_code >= 200 && match.status_code < 300 ? 'success' : 'error'}>
                              {match.method}
                            </ModernBadge>
                            <span className="font-mono text-sm">{match.path}</span>
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {new Date(match.timestamp).toLocaleString()}
                          </div>
                        </div>
                      </ModernCard>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </ModernCard>
        </Section>
      )}

      {/* Empty State */}
      {!result && !error && !isLoading && (
        <EmptyState
          icon={<Search className="h-12 w-12" />}
          title="No verification performed"
          description="Configure a verification pattern and click Verify to check if requests match your criteria."
        />
      )}
    </div>
  );
}
