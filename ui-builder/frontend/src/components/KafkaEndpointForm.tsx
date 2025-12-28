import { useState, useEffect } from 'react'
import { Plus, X, Code, Sparkles, AlertCircle, ChevronDown, ChevronRight } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import type { KafkaFormProps } from '@/types/protocol-configs'

interface JsonEditorErrors {
  staticPayload: string | null
  fakerSchema: string | null
}

const ACKS_OPTIONS = [
  { value: 'none', label: 'None (0)', description: 'Fire and forget' },
  { value: 'leader', label: 'Leader (1)', description: 'Wait for leader ack' },
  { value: 'all', label: 'All (-1)', description: 'Wait for all replicas' },
]

const COMPRESSION_OPTIONS = [
  { value: 'none', label: 'None' },
  { value: 'gzip', label: 'GZIP' },
  { value: 'snappy', label: 'Snappy' },
  { value: 'lz4', label: 'LZ4' },
  { value: 'zstd', label: 'Zstandard' },
]

const OFFSET_RESET_OPTIONS = [
  { value: 'earliest', label: 'Earliest', description: 'Start from beginning' },
  { value: 'latest', label: 'Latest', description: 'Start from end' },
]

export default function KafkaEndpointForm({ config, onChange, onValidationChange }: KafkaFormProps) {
  const [payloadType, setPayloadType] = useState<'static' | 'template' | 'faker'>(
    config.payload?.type?.toLowerCase() || 'static'
  )
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [showProducerConfig, setShowProducerConfig] = useState(false)
  const [showConsumerConfig, setShowConsumerConfig] = useState(false)
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    staticPayload: null,
    fakerSchema: null,
  })

  // Report validation state to parent when errors change
  useEffect(() => {
    const hasErrors = Object.values(jsonErrors).some((error) => error !== null)
    onValidationChange?.(!hasErrors)
  }, [jsonErrors, onValidationChange])

  const updateConfig = (updates: any) => {
    onChange({
      ...config,
      ...updates,
    })
  }

  const updatePayload = (updates: any) => {
    onChange({
      ...config,
      payload: {
        ...config.payload,
        ...updates,
      },
    })
  }

  const updateProducerConfig = (updates: any) => {
    onChange({
      ...config,
      producerConfig: {
        ...config.producerConfig,
        ...updates,
      },
    })
  }

  const updateConsumerConfig = (updates: any) => {
    onChange({
      ...config,
      consumerConfig: {
        ...config.consumerConfig,
        ...updates,
      },
    })
  }

  const updateAdvanced = (updates: any) => {
    onChange({
      ...config,
      advanced: {
        ...config.advanced,
        ...updates,
      },
    })
  }

  const addHeader = () => {
    const headers = config.advanced?.headers || []
    updateAdvanced({
      headers: [...headers, { key: '', value: '' }],
    })
  }

  const updateHeader = (index: number, field: string, value: string) => {
    const headers = [...(config.advanced?.headers || [])]
    headers[index] = { ...headers[index], [field]: value }
    updateAdvanced({ headers })
  }

  const removeHeader = (index: number) => {
    const headers = [...(config.advanced?.headers || [])]
    headers.splice(index, 1)
    updateAdvanced({ headers })
  }

  return (
    <div className="space-y-6">
      {/* Topic Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Topic Configuration</h2>
        <div className="space-y-4">
          <div>
            <label className="mb-2 block text-sm font-medium">Topic Name</label>
            <input
              type="text"
              value={config.topic || ''}
              onChange={(e) => updateConfig({ topic: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="my-topic"
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium">Partition (optional)</label>
              <input
                type="number"
                value={config.partition ?? ''}
                onChange={(e) => updateConfig({ partition: e.target.value ? parseInt(e.target.value) : undefined })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                min="0"
                placeholder="Auto-assign"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Leave empty for automatic partition assignment
              </p>
            </div>

            <div>
              <label className="mb-2 block text-sm font-medium">Message Key (optional)</label>
              <input
                type="text"
                value={config.key || ''}
                onChange={(e) => updateConfig({ key: e.target.value || undefined })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="message-key"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Used for partition routing
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Message Payload Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Message Payload</h2>

        {/* Payload Type Selector */}
        <div className="mb-4 flex space-x-2">
          {[
            { id: 'static', label: 'Static', icon: Code },
            { id: 'template', label: 'Template', icon: Code },
            { id: 'faker', label: 'Faker', icon: Sparkles },
          ].map((type) => {
            const Icon = type.icon
            return (
              <button
                key={type.id}
                onClick={() => {
                  setPayloadType(type.id as any)
                  if (type.id === 'static') {
                    updatePayload({ type: 'Static', content: {} })
                  } else if (type.id === 'template') {
                    updatePayload({ type: 'Template', template: '' })
                  } else if (type.id === 'faker') {
                    updatePayload({ type: 'Faker', schema: {} })
                  }
                }}
                className={cn(
                  'inline-flex items-center space-x-2 rounded-lg px-3 py-2 text-sm font-medium',
                  payloadType === type.id
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
                )}
              >
                <Icon className="h-4 w-4" />
                <span>{type.label}</span>
              </button>
            )
          })}
        </div>

        {/* Payload Editor */}
        {payloadType === 'static' && (
          <div>
            <div className={cn(
              'rounded-lg border',
              jsonErrors.staticPayload ? 'border-destructive' : 'border-border'
            )}>
              <Editor
                height="250px"
                defaultLanguage="json"
                value={JSON.stringify(config.payload?.content || {}, null, 2)}
                onChange={(value) => {
                  try {
                    const content = JSON.parse(value || '{}')
                    updatePayload({ type: 'Static', content })
                    setJsonErrors((prev) => ({ ...prev, staticPayload: null }))
                  } catch (e) {
                    const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                    setJsonErrors((prev) => ({ ...prev, staticPayload: errorMessage }))
                  }
                }}
                theme="vs-dark"
                options={{
                  minimap: { enabled: false },
                  fontSize: 13,
                }}
              />
            </div>
            {jsonErrors.staticPayload && (
              <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                <AlertCircle className="h-4 w-4" />
                <span>Invalid JSON: {jsonErrors.staticPayload}</span>
              </div>
            )}
          </div>
        )}

        {payloadType === 'template' && (
          <div>
            <textarea
              value={config.payload?.template || ''}
              onChange={(e) => updatePayload({ type: 'Template', template: e.target.value })}
              className="w-full rounded-lg border border-input bg-background p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              rows={8}
              placeholder={`{
  "eventId": "{{uuid}}",
  "timestamp": "{{now}}",
  "topic": "{{topic}}",
  "partition": {{partition}}
}`}
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Available tokens: {'{{'}uuid{'}}'}, {'{{'}now{'}}'}, {'{{'}topic{'}}'}, {'{{'}partition{'}}'}, {'{{'}offset{'}}'}, {'{{'}faker.name{'}}'}
            </p>
          </div>
        )}

        {payloadType === 'faker' && (
          <div>
            <div className={cn(
              'rounded-lg border',
              jsonErrors.fakerSchema ? 'border-destructive' : 'border-border'
            )}>
              <Editor
                height="250px"
                defaultLanguage="json"
                value={JSON.stringify(config.payload?.schema || {}, null, 2)}
                onChange={(value) => {
                  try {
                    const schema = JSON.parse(value || '{}')
                    updatePayload({ type: 'Faker', schema })
                    setJsonErrors((prev) => ({ ...prev, fakerSchema: null }))
                  } catch (e) {
                    const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                    setJsonErrors((prev) => ({ ...prev, fakerSchema: errorMessage }))
                  }
                }}
                theme="vs-dark"
                options={{
                  minimap: { enabled: false },
                  fontSize: 13,
                }}
              />
            </div>
            {jsonErrors.fakerSchema && (
              <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                <AlertCircle className="h-4 w-4" />
                <span>Invalid JSON: {jsonErrors.fakerSchema}</span>
              </div>
            )}
            <p className="mt-2 text-xs text-muted-foreground">
              Define a schema using Faker.js types like name, email, uuid, number.float, etc.
            </p>
          </div>
        )}
      </div>

      {/* Producer Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <button
          onClick={() => setShowProducerConfig(!showProducerConfig)}
          className="flex w-full items-center justify-between"
        >
          <h2 className="text-lg font-semibold">Producer Configuration</h2>
          {showProducerConfig ? (
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          )}
        </button>

        {showProducerConfig && (
          <div className="mt-4 space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-2 block text-sm font-medium">Acknowledgment</label>
                <select
                  value={config.producerConfig?.acks || 'all'}
                  onChange={(e) => updateProducerConfig({ acks: e.target.value })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  {ACKS_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label} - {option.description}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Compression</label>
                <select
                  value={config.producerConfig?.compression || 'none'}
                  onChange={(e) => updateProducerConfig({ compression: e.target.value })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  {COMPRESSION_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-2 block text-sm font-medium">Batch Size (bytes)</label>
                <input
                  type="number"
                  value={config.producerConfig?.batchSize ?? 16384}
                  onChange={(e) => updateProducerConfig({ batchSize: parseInt(e.target.value) })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Max batch size before sending
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Linger (ms)</label>
                <input
                  type="number"
                  value={config.producerConfig?.lingerMs ?? 0}
                  onChange={(e) => updateProducerConfig({ lingerMs: parseInt(e.target.value) })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Time to wait before sending batch
                </p>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Consumer Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <button
          onClick={() => setShowConsumerConfig(!showConsumerConfig)}
          className="flex w-full items-center justify-between"
        >
          <h2 className="text-lg font-semibold">Consumer Configuration</h2>
          {showConsumerConfig ? (
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          )}
        </button>

        {showConsumerConfig && (
          <div className="mt-4 space-y-4">
            <div>
              <label className="mb-2 block text-sm font-medium">Consumer Group ID</label>
              <input
                type="text"
                value={config.consumerConfig?.groupId || ''}
                onChange={(e) => updateConsumerConfig({ groupId: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="my-consumer-group"
              />
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <label className="mb-2 block text-sm font-medium">Auto Offset Reset</label>
                <select
                  value={config.consumerConfig?.autoOffsetReset || 'latest'}
                  onChange={(e) => updateConsumerConfig({ autoOffsetReset: e.target.value })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  {OFFSET_RESET_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label} - {option.description}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Auto Commit Interval (ms)</label>
                <input
                  type="number"
                  value={config.consumerConfig?.autoCommitIntervalMs ?? 5000}
                  onChange={(e) => updateConsumerConfig({ autoCommitIntervalMs: parseInt(e.target.value) })}
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  disabled={!config.consumerConfig?.enableAutoCommit}
                />
              </div>
            </div>

            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="enableAutoCommit"
                checked={config.consumerConfig?.enableAutoCommit ?? true}
                onChange={(e) => updateConsumerConfig({ enableAutoCommit: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="enableAutoCommit" className="text-sm font-medium">
                Enable Auto Commit
              </label>
            </div>
          </div>
        )}
      </div>

      {/* Advanced Settings */}
      <div className="rounded-lg border border-border bg-card p-6">
        <button
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="flex w-full items-center justify-between"
        >
          <h2 className="text-lg font-semibold">Advanced Settings</h2>
          {showAdvanced ? (
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          )}
        </button>

        {showAdvanced && (
          <div className="mt-4 space-y-4">
            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <label className="mb-2 block text-sm font-medium">Partitions</label>
                <input
                  type="number"
                  value={config.advanced?.partitions ?? 3}
                  onChange={(e) =>
                    updateAdvanced({ partitions: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="1"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Number of partitions
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Replication Factor</label>
                <input
                  type="number"
                  value={config.advanced?.replicationFactor ?? 1}
                  onChange={(e) =>
                    updateAdvanced({ replicationFactor: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="1"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Number of replicas
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Retention (ms)</label>
                <input
                  type="number"
                  value={config.advanced?.retentionMs ?? 604800000}
                  onChange={(e) =>
                    updateAdvanced({ retentionMs: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Message retention time
                </p>
              </div>
            </div>

            {/* Message Headers */}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <label className="text-sm font-medium">Message Headers</label>
                <button
                  onClick={addHeader}
                  className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
                >
                  <Plus className="h-3 w-3" />
                  <span>Add Header</span>
                </button>
              </div>
              {config.advanced?.headers?.length > 0 && (
                <div className="space-y-2">
                  {config.advanced.headers.map((header: any, index: number) => (
                    <div key={index} className="flex items-center space-x-2">
                      <input
                        type="text"
                        value={header.key}
                        onChange={(e) => updateHeader(index, 'key', e.target.value)}
                        placeholder="Header key"
                        className="flex-1 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      />
                      <input
                        type="text"
                        value={header.value}
                        onChange={(e) => updateHeader(index, 'value', e.target.value)}
                        placeholder="Header value"
                        className="flex-1 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      />
                      <button
                        onClick={() => removeHeader(index)}
                        className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Latency Simulation */}
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.advanced?.latency}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateAdvanced({
                        latency: { minMs: 0, maxMs: 100 },
                      })
                    } else {
                      const { latency, ...rest } = config.advanced || {}
                      onChange({ ...config, advanced: rest })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Simulate Latency</span>
              </label>
              {config.advanced?.latency && (
                <div className="mt-2 grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Min Latency (ms)</label>
                    <input
                      type="number"
                      value={config.advanced.latency.minMs ?? 0}
                      onChange={(e) =>
                        updateAdvanced({
                          latency: {
                            ...config.advanced.latency,
                            minMs: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                      min="0"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Max Latency (ms)</label>
                    <input
                      type="number"
                      value={config.advanced.latency.maxMs ?? 100}
                      onChange={(e) =>
                        updateAdvanced({
                          latency: {
                            ...config.advanced.latency,
                            maxMs: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                      min="0"
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
