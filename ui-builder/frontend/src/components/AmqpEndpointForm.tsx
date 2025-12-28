import { useState, useEffect } from 'react'
import { Code, Sparkles, AlertCircle, ChevronDown, ChevronRight } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import type { AmqpFormProps } from '@/types/protocol-configs'

interface JsonEditorErrors {
  staticPayload: string | null
  fakerSchema: string | null
  onConnectPayload: string | null
  onPublishPayload: string | null
}

const EXCHANGE_TYPES = [
  { value: 'direct', label: 'Direct', description: 'Route by exact routing key match' },
  { value: 'fanout', label: 'Fanout', description: 'Broadcast to all bound queues' },
  { value: 'topic', label: 'Topic', description: 'Route by routing key pattern' },
  { value: 'headers', label: 'Headers', description: 'Route by message headers' },
]

export default function AmqpEndpointForm({ config, onChange, onValidationChange }: AmqpFormProps) {
  const [payloadType, setPayloadType] = useState<'static' | 'template' | 'faker'>(
    config.payload?.type?.toLowerCase() || 'static'
  )
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [showConnectionBehavior, setShowConnectionBehavior] = useState(false)
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    staticPayload: null,
    fakerSchema: null,
    onConnectPayload: null,
    onPublishPayload: null,
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

  const updateConnectionBehavior = (updates: any) => {
    onChange({
      ...config,
      connectionBehavior: {
        ...config.connectionBehavior,
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

  return (
    <div className="space-y-6">
      {/* Exchange Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Exchange Configuration</h2>
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium">Exchange Name</label>
              <input
                type="text"
                value={config.exchange || ''}
                onChange={(e) => updateConfig({ exchange: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="my-exchange"
              />
            </div>

            <div>
              <label className="mb-2 block text-sm font-medium">Exchange Type</label>
              <select
                value={config.exchangeType || 'direct'}
                onChange={(e) => updateConfig({ exchangeType: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              >
                {EXCHANGE_TYPES.map((type) => (
                  <option key={type.value} value={type.value}>
                    {type.label} - {type.description}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div>
            <label className="mb-2 block text-sm font-medium">Routing Key</label>
            <input
              type="text"
              value={config.routingKey || ''}
              onChange={(e) => updateConfig({ routingKey: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="orders.new or orders.#"
            />
            <p className="mt-2 text-xs text-muted-foreground">
              For topic exchanges: <code className="rounded bg-muted px-1">*</code> matches one word, <code className="rounded bg-muted px-1">#</code> matches zero or more words
            </p>
          </div>

          <div>
            <label className="mb-2 block text-sm font-medium">Queue (optional)</label>
            <input
              type="text"
              value={config.queue || ''}
              onChange={(e) => updateConfig({ queue: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="my-queue"
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Queue to bind to the exchange. Leave empty for auto-generated queue.
            </p>
          </div>

          <div className="flex items-center space-x-6">
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="durable"
                checked={config.durable ?? true}
                onChange={(e) => updateConfig({ durable: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="durable" className="text-sm font-medium">
                Durable
              </label>
              <span className="text-xs text-muted-foreground">
                (Survive broker restart)
              </span>
            </div>

            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="autoDelete"
                checked={config.autoDelete ?? false}
                onChange={(e) => updateConfig({ autoDelete: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="autoDelete" className="text-sm font-medium">
                Auto-delete
              </label>
              <span className="text-xs text-muted-foreground">
                (Delete when unused)
              </span>
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
  "orderId": "{{uuid}}",
  "timestamp": "{{now}}",
  "routingKey": "{{routingKey}}",
  "exchange": "{{exchange}}"
}`}
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Available tokens: {'{{'}uuid{'}}'}, {'{{'}now{'}}'}, {'{{'}routingKey{'}}'}, {'{{'}exchange{'}}'}, {'{{'}faker.name{'}}'}
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

      {/* Connection Behavior */}
      <div className="rounded-lg border border-border bg-card p-6">
        <button
          onClick={() => setShowConnectionBehavior(!showConnectionBehavior)}
          className="flex w-full items-center justify-between"
        >
          <h2 className="text-lg font-semibold">Connection Behavior</h2>
          {showConnectionBehavior ? (
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          )}
        </button>

        {showConnectionBehavior && (
          <div className="mt-4 space-y-6">
            {/* On Connect */}
            <div>
              <div className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="onConnect"
                  checked={!!config.connectionBehavior?.onConnect?.enabled}
                  onChange={(e) =>
                    updateConnectionBehavior({
                      onConnect: {
                        ...config.connectionBehavior?.onConnect,
                        enabled: e.target.checked,
                      },
                    })
                  }
                  className="h-4 w-4 rounded border-input"
                />
                <label htmlFor="onConnect" className="text-sm font-medium">
                  On Connect
                </label>
                <span className="text-xs text-muted-foreground">
                  (Publish message when client connects)
                </span>
              </div>
              {config.connectionBehavior?.onConnect?.enabled && (
                <div className="ml-6 space-y-3">
                  <div className="grid gap-4 md:grid-cols-2">
                    <div>
                      <label className="mb-1 block text-xs text-muted-foreground">Exchange</label>
                      <input
                        type="text"
                        value={config.connectionBehavior?.onConnect?.exchange || ''}
                        onChange={(e) =>
                          updateConnectionBehavior({
                            onConnect: {
                              ...config.connectionBehavior?.onConnect,
                              exchange: e.target.value,
                            },
                          })
                        }
                        className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        placeholder="welcome-exchange"
                      />
                    </div>
                    <div>
                      <label className="mb-1 block text-xs text-muted-foreground">Routing Key</label>
                      <input
                        type="text"
                        value={config.connectionBehavior?.onConnect?.routingKey || ''}
                        onChange={(e) =>
                          updateConnectionBehavior({
                            onConnect: {
                              ...config.connectionBehavior?.onConnect,
                              routingKey: e.target.value,
                            },
                          })
                        }
                        className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        placeholder="welcome"
                      />
                    </div>
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Payload (JSON)</label>
                    <div className={cn(
                      'rounded-lg border',
                      jsonErrors.onConnectPayload ? 'border-destructive' : 'border-border'
                    )}>
                      <Editor
                        height="120px"
                        defaultLanguage="json"
                        value={JSON.stringify(config.connectionBehavior?.onConnect?.payload || { message: 'Welcome!' }, null, 2)}
                        onChange={(value) => {
                          try {
                            const payload = JSON.parse(value || '{}')
                            updateConnectionBehavior({
                              onConnect: {
                                ...config.connectionBehavior?.onConnect,
                                payload,
                              },
                            })
                            setJsonErrors((prev) => ({ ...prev, onConnectPayload: null }))
                          } catch (e) {
                            const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                            setJsonErrors((prev) => ({ ...prev, onConnectPayload: errorMessage }))
                          }
                        }}
                        theme="vs-dark"
                        options={{
                          minimap: { enabled: false },
                          fontSize: 13,
                        }}
                      />
                    </div>
                    {jsonErrors.onConnectPayload && (
                      <div className="mt-1 flex items-center space-x-2 text-xs text-destructive">
                        <AlertCircle className="h-3 w-3" />
                        <span>{jsonErrors.onConnectPayload}</span>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>

            {/* On Publish */}
            <div>
              <div className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="onPublish"
                  checked={!!config.connectionBehavior?.onPublish?.enabled}
                  onChange={(e) =>
                    updateConnectionBehavior({
                      onPublish: {
                        ...config.connectionBehavior?.onPublish,
                        enabled: e.target.checked,
                      },
                    })
                  }
                  className="h-4 w-4 rounded border-input"
                />
                <label htmlFor="onPublish" className="text-sm font-medium">
                  On Publish
                </label>
                <span className="text-xs text-muted-foreground">
                  (How to respond to published messages)
                </span>
              </div>
              {config.connectionBehavior?.onPublish?.enabled && (
                <div className="ml-6 space-y-3">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Response Type</label>
                    <select
                      value={config.connectionBehavior?.onPublish?.responseType || 'ack'}
                      onChange={(e) =>
                        updateConnectionBehavior({
                          onPublish: {
                            ...config.connectionBehavior?.onPublish,
                            responseType: e.target.value,
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    >
                      <option value="ack">Acknowledge (ack)</option>
                      <option value="nack">Negative Acknowledge (nack)</option>
                      <option value="echo">Echo message back</option>
                      <option value="custom">Custom response</option>
                    </select>
                  </div>
                  {config.connectionBehavior?.onPublish?.responseType === 'custom' && (
                    <div>
                      <label className="mb-1 block text-xs text-muted-foreground">Custom Response (JSON)</label>
                      <div className={cn(
                        'rounded-lg border',
                        jsonErrors.onPublishPayload ? 'border-destructive' : 'border-border'
                      )}>
                        <Editor
                          height="120px"
                          defaultLanguage="json"
                          value={JSON.stringify(config.connectionBehavior?.onPublish?.payload || {}, null, 2)}
                          onChange={(value) => {
                            try {
                              const payload = JSON.parse(value || '{}')
                              updateConnectionBehavior({
                                onPublish: {
                                  ...config.connectionBehavior?.onPublish,
                                  payload,
                                },
                              })
                              setJsonErrors((prev) => ({ ...prev, onPublishPayload: null }))
                            } catch (e) {
                              const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                              setJsonErrors((prev) => ({ ...prev, onPublishPayload: errorMessage }))
                            }
                          }}
                          theme="vs-dark"
                          options={{
                            minimap: { enabled: false },
                            fontSize: 13,
                          }}
                        />
                      </div>
                      {jsonErrors.onPublishPayload && (
                        <div className="mt-1 flex items-center space-x-2 text-xs text-destructive">
                          <AlertCircle className="h-3 w-3" />
                          <span>{jsonErrors.onPublishPayload}</span>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              )}
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
                <label className="mb-2 block text-sm font-medium">Prefetch Count</label>
                <input
                  type="number"
                  value={config.advanced?.prefetchCount ?? 1}
                  onChange={(e) =>
                    updateAdvanced({ prefetchCount: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="1"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Messages to prefetch
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Heartbeat (seconds)</label>
                <input
                  type="number"
                  value={config.advanced?.heartbeatInterval ?? 60}
                  onChange={(e) =>
                    updateAdvanced({ heartbeatInterval: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="60"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Connection heartbeat
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Connection Timeout (ms)</label>
                <input
                  type="number"
                  value={config.advanced?.connectionTimeout ?? 10000}
                  onChange={(e) =>
                    updateAdvanced({ connectionTimeout: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="10000"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Connection timeout
                </p>
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-3">
              <div>
                <label className="mb-2 block text-sm font-medium">Priority</label>
                <input
                  type="number"
                  value={config.advanced?.priority ?? 0}
                  onChange={(e) =>
                    updateAdvanced({ priority: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  max="9"
                  placeholder="0"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Message priority (0-9)
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Expiration</label>
                <input
                  type="text"
                  value={config.advanced?.expiration || ''}
                  onChange={(e) =>
                    updateAdvanced({ expiration: e.target.value })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  placeholder="60000"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  TTL in milliseconds
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Correlation ID</label>
                <input
                  type="text"
                  value={config.advanced?.correlationId || ''}
                  onChange={(e) =>
                    updateAdvanced({ correlationId: e.target.value })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  placeholder="Optional correlation ID"
                />
              </div>
            </div>

            <div className="flex items-center space-x-6">
              <div className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="mandatory"
                  checked={config.advanced?.mandatory ?? false}
                  onChange={(e) => updateAdvanced({ mandatory: e.target.checked })}
                  className="h-4 w-4 rounded border-input"
                />
                <label htmlFor="mandatory" className="text-sm font-medium">
                  Mandatory
                </label>
              </div>

              <div className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="immediate"
                  checked={config.advanced?.immediate ?? false}
                  onChange={(e) => updateAdvanced({ immediate: e.target.checked })}
                  className="h-4 w-4 rounded border-input"
                />
                <label htmlFor="immediate" className="text-sm font-medium">
                  Immediate
                </label>
              </div>
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
