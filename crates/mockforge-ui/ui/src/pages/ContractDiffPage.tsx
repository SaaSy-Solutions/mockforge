import { logger } from '@/utils/logger';
import React, { useState, useEffect } from 'react';
import {
  FileText,
  Search,
  Play,
  RefreshCw,
  AlertCircle,
  CheckCircle2,
  XCircle,
  TrendingUp,
  FileCode,
  Download,
  Upload,
  Filter,
  Plus,
  Network
} from 'lucide-react';
import { contractDiffApi, type CapturedRequest, type ContractDiffResult, type AnalyzeRequestPayload } from '../services/api';
import { protocolContractsApi, type ProtocolType } from '../services/protocolContractsApi';
import { ProtocolContractEditor } from '../components/ProtocolContractEditor';
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
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { AIStudioNav } from '../components/ai/AIStudioNav';

// Confidence indicator component
function ConfidenceIndicator({ confidence }: { confidence: number }) {
  const percentage = Math.round(confidence * 100);
  const color = confidence >= 0.8 ? 'text-green-600' : confidence >= 0.5 ? 'text-yellow-600' : 'text-red-600';
  const bgColor = confidence >= 0.8 ? 'bg-green-100' : confidence >= 0.5 ? 'bg-yellow-100' : 'bg-red-100';

  return (
    <div className={`inline-flex items-center px-2 py-1 rounded-full ${bgColor}`}>
      <span className={`text-sm font-medium ${color}`}>{percentage}%</span>
    </div>
  );
}

// Severity badge component
function SeverityBadge({ severity }: { severity: string }) {
  const colors: Record<string, { bg: string; text: string }> = {
    critical: { bg: 'bg-red-100', text: 'text-red-800' },
    high: { bg: 'bg-orange-100', text: 'text-orange-800' },
    medium: { bg: 'bg-yellow-100', text: 'text-yellow-800' },
    low: { bg: 'bg-blue-100', text: 'text-blue-800' },
  };

  const style = colors[severity] || colors.low;

  return (
    <span className={`px-2 py-1 rounded text-xs font-medium ${style.bg} ${style.text}`}>
      {severity.toUpperCase()}
    </span>
  );
}

// Mismatch table component
function MismatchTable({ mismatches }: { mismatches: ContractDiffResult['mismatches'] }) {
  if (mismatches.length === 0) {
    return (
      <EmptyState
        icon={CheckCircle2}
        title="No Mismatches"
        description="All requests match the contract specification"
      />
    );
  }

  // Check if any mismatch has classification metadata
  const hasClassification = mismatches.some(m =>
    m.context && (m.context.is_additive !== undefined || m.context.is_breaking !== undefined)
  );

  // Check if any mismatch has schema format or protocol info
  const hasSchemaFormat = mismatches.some(m => m.context?.schema_format);
  const hasProtocolInfo = mismatches.some(m => m.context?.service || m.context?.method);

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b border-gray-200">
            <th className="text-left p-3 font-semibold text-sm text-gray-700">Path</th>
            <th className="text-left p-3 font-semibold text-sm text-gray-700">Type</th>
            {hasClassification && (
              <th className="text-left p-3 font-semibold text-sm text-gray-700">Classification</th>
            )}
            {hasSchemaFormat && (
              <th className="text-left p-3 font-semibold text-sm text-gray-700">Schema Format</th>
            )}
            {hasProtocolInfo && (
              <th className="text-left p-3 font-semibold text-sm text-gray-700">Protocol Info</th>
            )}
            <th className="text-left p-3 font-semibold text-sm text-gray-700">Severity</th>
            <th className="text-left p-3 font-semibold text-sm text-gray-700">Confidence</th>
            <th className="text-left p-3 font-semibold text-sm text-gray-700">Description</th>
          </tr>
        </thead>
        <tbody>
          {mismatches.map((mismatch, idx) => {
            const isAdditive = mismatch.context?.is_additive === true;
            const isBreaking = mismatch.context?.is_breaking === true;
            const changeCategory = mismatch.context?.change_category as string | undefined;
            const schemaFormat = mismatch.context?.schema_format as string | undefined;
            const service = mismatch.context?.service as string | undefined;
            const method = mismatch.context?.method as string | undefined;
            const fieldName = mismatch.context?.field_name as string | undefined;

            return (
              <tr key={idx} className="border-b border-gray-100 hover:bg-gray-50">
                <td className="p-3 text-sm font-mono text-gray-900">{mismatch.path}</td>
                <td className="p-3 text-sm text-gray-600">{mismatch.mismatch_type}</td>
                {hasClassification && (
                  <td className="p-3">
                    <div className="flex items-center gap-2 flex-wrap">
                      {isBreaking && (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-300">
                          Breaking
                        </span>
                      )}
                      {isAdditive && (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-300">
                          Additive
                        </span>
                      )}
                      {changeCategory && (
                        <span className="text-xs text-gray-500 dark:text-gray-400">
                          {changeCategory.replace(/_/g, ' ')}
                        </span>
                      )}
                    </div>
                  </td>
                )}
                {hasSchemaFormat && (
                  <td className="p-3">
                    {schemaFormat && (
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-300">
                        {schemaFormat.replace(/_/g, ' ').toUpperCase()}
                      </span>
                    )}
                  </td>
                )}
                {hasProtocolInfo && (
                  <td className="p-3">
                    <div className="flex flex-col gap-1 text-xs">
                      {service && (
                        <span className="text-gray-600 dark:text-gray-400">
                          <span className="font-semibold">Service:</span> {service}
                        </span>
                      )}
                      {method && (
                        <span className="text-gray-600 dark:text-gray-400">
                          <span className="font-semibold">Method:</span> {method}
                        </span>
                      )}
                      {fieldName && (
                        <span className="text-gray-600 dark:text-gray-400">
                          <span className="font-semibold">Field:</span> {fieldName}
                        </span>
                      )}
                    </div>
                  </td>
                )}
                <td className="p-3">
                  <SeverityBadge severity={mismatch.severity} />
                </td>
                <td className="p-3">
                  <ConfidenceIndicator confidence={mismatch.confidence} />
                </td>
                <td className="p-3 text-sm text-gray-700">
                  <div>{mismatch.description}</div>
                  {mismatch.expected && (
                    <div className="mt-1 text-xs text-gray-500">
                      <span className="font-semibold">Expected:</span> {mismatch.expected}
                    </div>
                  )}
                  {mismatch.actual && (
                    <div className="mt-1 text-xs text-gray-500">
                      <span className="font-semibold">Actual:</span> {mismatch.actual}
                    </div>
                  )}
                  {mismatch.context?.old_type && mismatch.context?.new_type && (
                    <div className="mt-1 text-xs text-gray-500">
                      <span className="font-semibold">Type Change:</span> {mismatch.context.old_type} → {mismatch.context.new_type}
                    </div>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

// Diff viewer component
function DiffViewer({ mismatch }: { mismatch: ContractDiffResult['mismatches'][0] }) {
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        {mismatch.expected && (
          <div>
            <Label className="text-xs font-semibold text-gray-600 mb-2 block">Expected</Label>
            <div className="bg-green-50 border border-green-200 rounded p-3 font-mono text-xs">
              <pre className="whitespace-pre-wrap">{mismatch.expected}</pre>
            </div>
          </div>
        )}
        {mismatch.actual && (
          <div>
            <Label className="text-xs font-semibold text-gray-600 mb-2 block">Actual</Label>
            <div className="bg-red-50 border border-red-200 rounded p-3 font-mono text-xs">
              <pre className="whitespace-pre-wrap">{mismatch.actual}</pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Recommendations component
function RecommendationsList({ recommendations }: { recommendations: ContractDiffResult['recommendations'] }) {
  if (recommendations.length === 0) {
    return (
      <EmptyState
        icon={FileText}
        title="No Recommendations"
        description="No AI recommendations available"
      />
    );
  }

  return (
    <div className="space-y-3">
      {recommendations.map((rec, idx) => (
        <div key={idx} className="border border-gray-200 rounded-lg p-4 bg-white">
          <div className="flex items-start justify-between mb-2">
            <div className="flex-1">
              <p className="text-sm text-gray-900">{rec.recommendation}</p>
              {rec.suggested_fix && (
                <div className="mt-2 p-2 bg-blue-50 border border-blue-200 rounded">
                  <p className="text-xs font-semibold text-blue-900 mb-1">Suggested Fix:</p>
                  <p className="text-xs text-blue-800">{rec.suggested_fix}</p>
                </div>
              )}
            </div>
            <ConfidenceIndicator confidence={rec.confidence} />
          </div>
        </div>
      ))}
    </div>
  );
}

// Correction proposals component
function CorrectionProposals({ corrections }: { corrections: ContractDiffResult['corrections'] }) {
  if (corrections.length === 0) {
    return (
      <EmptyState
        icon={FileCode}
        title="No Corrections"
        description="No correction proposals available"
      />
    );
  }

  return (
    <div className="space-y-3">
      {corrections.map((correction, idx) => (
        <div key={idx} className="border border-gray-200 rounded-lg p-4 bg-white">
          <div className="flex items-start justify-between mb-2">
            <div className="flex-1">
              <p className="text-sm font-semibold text-gray-900 mb-1">{correction.description}</p>
              <p className="text-xs text-gray-600 font-mono mb-2">Path: {correction.path}</p>
              <div className="flex items-center gap-2">
                <ModernBadge variant="outline">{correction.operation}</ModernBadge>
                {correction.value && (
                  <div className="text-xs text-gray-600">
                    Value: <code className="bg-gray-100 px-1 rounded">{JSON.stringify(correction.value)}</code>
                  </div>
                )}
              </div>
            </div>
            <ConfidenceIndicator confidence={correction.confidence} />
          </div>
        </div>
      ))}
    </div>
  );
}

export function ContractDiffPage() {
  const queryClient = useQueryClient();
  const [selectedCapture, setSelectedCapture] = useState<string | null>(null);
  const [analysisResult, setAnalysisResult] = useState<ContractDiffResult | null>(null);
  const [specPath, setSpecPath] = useState('');
  const [specContent, setSpecContent] = useState('');
  const [filterSource, setFilterSource] = useState<string>('all');
  const [filterMethod, setFilterMethod] = useState<string>('all');
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [selectedProtocol, setSelectedProtocol] = useState<'http' | ProtocolType>('http');
  const [showProtocolEditor, setShowProtocolEditor] = useState(false);

  // Fetch captured requests
  const { data: capturesData, isLoading: capturesLoading, refetch: refetchCaptures } = useQuery({
    queryKey: ['contract-diff-captures', filterSource, filterMethod],
    queryFn: async () => {
      const params: any = {};
      if (filterSource !== 'all') params.source = filterSource;
      if (filterMethod !== 'all') params.method = filterMethod;
      return contractDiffApi.getCapturedRequests(params);
    },
  });

  // Fetch statistics
  const { data: statsData } = useQuery({
    queryKey: ['contract-diff-statistics'],
    queryFn: () => contractDiffApi.getStatistics(),
    refetchInterval: 5000, // Refresh every 5 seconds
  });

  // Fetch protocol contracts
  const { data: protocolContractsData } = useQuery({
    queryKey: ['protocol-contracts', selectedProtocol !== 'http' ? selectedProtocol : undefined],
    queryFn: () => {
      if (selectedProtocol === 'http') {
        return Promise.resolve({ contracts: [], total: 0 });
      }
      return protocolContractsApi.listContracts(selectedProtocol as ProtocolType);
    },
    enabled: selectedProtocol !== 'http',
  });

  // Analyze mutation
  const analyzeMutation = useMutation({
    mutationFn: async ({ captureId, payload }: { captureId: string; payload: AnalyzeRequestPayload }) => {
      return contractDiffApi.analyzeCapturedRequest(captureId, payload);
    },
    onSuccess: (data) => {
      setAnalysisResult(data.result);
      queryClient.invalidateQueries({ queryKey: ['contract-diff-captures'] });
      queryClient.invalidateQueries({ queryKey: ['contract-diff-statistics'] });
    },
    onError: (error: Error) => {
      logger.error('Analysis failed', error);
      alert(`Analysis failed: ${error.message}`);
    },
  });

  const handleAnalyze = async () => {
    if (!selectedCapture) {
      alert('Please select a captured request');
      return;
    }

    if (!specPath && !specContent) {
      alert('Please provide either a spec path or spec content');
      return;
    }

    setIsAnalyzing(true);
    try {
      const payload: AnalyzeRequestPayload = {
        spec_path: specPath || undefined,
        spec_content: specContent || undefined,
        config: {
          llm_provider: 'openai',
          confidence_threshold: 0.5,
        },
      };

      await analyzeMutation.mutateAsync({ captureId: selectedCapture, payload });
    } catch (error) {
      // Error handled by mutation
    } finally {
      setIsAnalyzing(false);
    }
  };

  const captures = capturesData?.captures || [];
  const statistics = statsData?.statistics;

  // Get unique sources and methods for filters
  const sources = Array.from(new Set(captures.map(c => c.source))).filter(Boolean);
  const methods = Array.from(new Set(captures.map(c => c.method))).filter(Boolean);

  return (
    <div className="space-y-6 p-6">
      <AIStudioNav currentPage="Contract Diff" showQuickActions={false} />
      <div className="flex items-center justify-between">
        <PageHeader
          title="Contract Diff Analysis"
          description="Analyze front-end requests against backend contract specifications"
          icon={FileText}
        />
        <div className="flex items-center gap-4">
          <Select value={selectedProtocol} onValueChange={(value) => setSelectedProtocol(value as 'http' | ProtocolType)}>
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="http">HTTP/REST</SelectItem>
              <SelectItem value="grpc">gRPC</SelectItem>
              <SelectItem value="websocket">WebSocket</SelectItem>
              <SelectItem value="mqtt">MQTT</SelectItem>
              <SelectItem value="kafka">Kafka</SelectItem>
            </SelectContent>
          </Select>
          {selectedProtocol !== 'http' && (
            <Button onClick={() => setShowProtocolEditor(true)}>
              <Plus className="w-4 h-4 mr-2" />
              New {selectedProtocol.toUpperCase()} Contract
            </Button>
          )}
        </div>
      </div>

      {/* Protocol Contract Editor Modal */}
      {showProtocolEditor && (
        <ModernCard className="p-6">
          <ProtocolContractEditor
            onClose={() => setShowProtocolEditor(false)}
            onSuccess={() => {
              setShowProtocolEditor(false);
              queryClient.invalidateQueries({ queryKey: ['protocol-contracts'] });
            }}
          />
        </ModernCard>
      )}

      {/* Protocol-specific content */}
      {selectedProtocol !== 'http' && (
        <Section title={`${selectedProtocol.toUpperCase()} Contracts`}>
          {protocolContractsData?.contracts.length === 0 ? (
            <EmptyState
              icon={Network}
              title="No Contracts"
              description={`No ${selectedProtocol.toUpperCase()} contracts found. Create one to get started.`}
            />
          ) : (
            <div className="space-y-4">
              {protocolContractsData?.contracts.map((contract) => (
                <ModernCard key={contract.contract_id} className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-semibold">{contract.contract_id}</h3>
                      <p className="text-sm text-gray-600">Version: {contract.version}</p>
                    </div>
                    <ModernBadge variant="outline">{contract.protocol.toUpperCase()}</ModernBadge>
                  </div>
                </ModernCard>
              ))}
            </div>
          )}
        </Section>
      )}

      {/* HTTP/REST specific content */}
      {selectedProtocol === 'http' && (
        <>
          {/* Statistics Cards */}
      {statistics && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <ModernCard>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600">Total Captures</p>
                <p className="text-2xl font-bold text-gray-900">{statistics.total_captures}</p>
              </div>
              <FileText className="w-8 h-8 text-blue-500" />
            </div>
          </ModernCard>
          <ModernCard>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600">Analyzed</p>
                <p className="text-2xl font-bold text-gray-900">{statistics.analyzed_captures}</p>
              </div>
              <CheckCircle2 className="w-8 h-8 text-green-500" />
            </div>
          </ModernCard>
          <ModernCard>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600">Sources</p>
                <p className="text-2xl font-bold text-gray-900">{Object.keys(statistics.sources).length}</p>
              </div>
              <TrendingUp className="w-8 h-8 text-purple-500" />
            </div>
          </ModernCard>
          <ModernCard>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600">Methods</p>
                <p className="text-2xl font-bold text-gray-900">{Object.keys(statistics.methods).length}</p>
              </div>
              <Filter className="w-8 h-8 text-orange-500" />
            </div>
          </ModernCard>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left Column: Captured Requests */}
        <Section title="Captured Requests">
          <div className="space-y-4">
            {/* Filters */}
            <div className="grid grid-cols-2 gap-4">
              <div>
                <Label>Source</Label>
                <Select value={filterSource} onValueChange={setFilterSource}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Sources</SelectItem>
                    {sources.map(source => (
                      <SelectItem key={source} value={source}>{source}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div>
                <Label>Method</Label>
                <Select value={filterMethod} onValueChange={setFilterMethod}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Methods</SelectItem>
                    {methods.map(method => (
                      <SelectItem key={method} value={method}>{method}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            {/* Request List */}
            <div className="border border-gray-200 rounded-lg divide-y divide-gray-200 max-h-96 overflow-y-auto">
              {capturesLoading ? (
                <div className="p-4 text-center text-gray-500">Loading...</div>
              ) : captures.length === 0 ? (
                <div className="p-4 text-center text-gray-500">No captured requests</div>
              ) : (
                captures.map(capture => (
                  <div
                    key={capture.id}
                    onClick={() => setSelectedCapture(capture.id || null)}
                    className={`p-4 cursor-pointer hover:bg-gray-50 transition-colors ${
                      selectedCapture === capture.id ? 'bg-blue-50 border-l-4 border-blue-500' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <ModernBadge variant="outline">{capture.method}</ModernBadge>
                          <span className="text-sm font-mono text-gray-900">{capture.path}</span>
                        </div>
                        <div className="flex items-center gap-2 text-xs text-gray-500">
                          <span>{capture.source}</span>
                          {capture.analyzed && (
                            <ModernBadge variant="success" size="sm">Analyzed</ModernBadge>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>

            <Button onClick={() => refetchCaptures()} variant="outline" className="w-full">
              <RefreshCw className="w-4 h-4 mr-2" />
              Refresh
            </Button>
          </div>
        </Section>

        {/* Right Column: Analysis Configuration */}
        <Section title="Analysis Configuration">
          <div className="space-y-4">
            <div>
              <Label>Contract Spec Path</Label>
              <Input
                placeholder="/path/to/openapi.yaml"
                value={specPath}
                onChange={(e) => setSpecPath(e.target.value)}
              />
            </div>
            <div>
              <Label>Or Contract Spec Content (YAML/JSON)</Label>
              <Textarea
                placeholder="Paste OpenAPI spec content here..."
                value={specContent}
                onChange={(e) => setSpecContent(e.target.value)}
                rows={8}
                className="font-mono text-xs"
              />
            </div>
            <Button
              onClick={handleAnalyze}
              disabled={!selectedCapture || isAnalyzing || (!specPath && !specContent)}
              className="w-full"
            >
              <Play className="w-4 h-4 mr-2" />
              {isAnalyzing ? 'Analyzing...' : 'Analyze Request'}
            </Button>
          </div>
        </Section>
      </div>

      {/* Analysis Results */}
      {analysisResult && (
        <div className="space-y-6">
          <Section title="Analysis Results">
            <div className="space-y-4">
              {/* Overall Status */}
              <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                <div className="flex items-center gap-4">
                  {analysisResult.matches ? (
                    <CheckCircle2 className="w-6 h-6 text-green-500" />
                  ) : (
                    <XCircle className="w-6 h-6 text-red-500" />
                  )}
                  <div>
                    <p className="font-semibold text-gray-900">
                      {analysisResult.matches ? 'Contract Matches' : 'Contract Mismatches Detected'}
                    </p>
                    <p className="text-sm text-gray-600">
                      {analysisResult.mismatches.length} mismatch(es) found
                    </p>
                    {/* Show protocol and schema format info if available */}
                    {(analysisResult.metadata?.contract_format || selectedProtocol !== 'http') && (
                      <div className="flex items-center gap-2 mt-1">
                        {selectedProtocol !== 'http' && (
                          <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-300">
                            <Network className="w-3 h-3 mr-1" />
                            {selectedProtocol.toUpperCase()}
                          </span>
                        )}
                        {analysisResult.metadata?.contract_format && (
                          <span className="text-xs text-gray-500 dark:text-gray-400">
                            Format: {analysisResult.metadata.contract_format}
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                </div>
                <ConfidenceIndicator confidence={analysisResult.confidence} />
              </div>

              {/* Mismatches Table */}
              <div>
                <h3 className="text-lg font-semibold mb-3">Mismatches</h3>
                <MismatchTable mismatches={analysisResult.mismatches} />
              </div>

              {/* Recommendations */}
              {analysisResult.recommendations.length > 0 && (
                <div>
                  <h3 className="text-lg font-semibold mb-3">AI Recommendations</h3>
                  <RecommendationsList recommendations={analysisResult.recommendations} />
                </div>
              )}

              {/* Correction Proposals */}
              {analysisResult.corrections.length > 0 && (
                <div>
                  <h3 className="text-lg font-semibold mb-3">Correction Proposals</h3>
                  <CorrectionProposals corrections={analysisResult.corrections} />
                  <div className="mt-4">
                    <Button
                      variant="outline"
                      onClick={async () => {
                        if (!selectedCapture) return;
                        try {
                          const payload: AnalyzeRequestPayload = {
                            spec_path: specPath || undefined,
                            spec_content: specContent || undefined,
                            config: {
                              llm_provider: 'openai',
                              confidence_threshold: 0.5,
                            },
                          };
                          const result = await contractDiffApi.generatePatchFile(selectedCapture, payload);
                          const blob = new Blob([JSON.stringify(result.patch_file, null, 2)], { type: 'application/json' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          a.download = `contract-patch-${selectedCapture}.json`;
                          document.body.appendChild(a);
                          a.click();
                          document.body.removeChild(a);
                          URL.revokeObjectURL(url);
                        } catch (error: any) {
                          alert(`Failed to generate patch: ${error.message}`);
                        }
                      }}
                    >
                      <Download className="w-4 h-4 mr-2" />
                      Download Patch File
                    </Button>
                  </div>
                </div>
              )}
            </div>
          </Section>
        </div>
      )}
        </>
      )}
    </div>
  );
}
