import { useState, useEffect } from 'react'
import { Plus, X, Code, Sparkles, AlertCircle, ChevronDown, ChevronRight } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import type { MqttFormProps } from '@/types/protocol-configs'

interface JsonEditorErrors {
  staticPayload: string | null
  fakerSchema: string | null
  onConnectPayload: string | null
  onSubscribePayload: string | null
  onPublishPayload: string | null
}

const QOS_LEVELS = [
  { value: 0, label: '0 - At most once (fire and forget)' },
  { value: 1, label: '1 - At least once (acknowledged delivery)' },
  { value: 2, label: '2 - Exactly once (assured delivery)' },
]

export default function MqttEndpointForm({ config, onChange, onValidationChange }: MqttFormProps) {
  const [payloadType, setPayloadType] = useState<'static' | 'template' | 'faker'>(
    config.payload?.type?.toLowerCase() || 'static'
  )
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [showConnectionBehavior, setShowConnectionBehavior] = useState(false)
  const [showSubscriptionMatching, setShowSubscriptionMatching] = useState(false)
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    staticPayload: null,
    fakerSchema: null,
    onConnectPayload: null,
    onSubscribePayload: null,
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

  const updateSubscriptionMatching = (updates: any) => {
    onChange({
      ...config,
      subscriptionMatching: {
        ...config.subscriptionMatching,
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

  const addTopicFilter = () => {
    const filters = config.subscriptionMatching?.topicFilters || []
    updateSubscriptionMatching({
      topicFilters: [...filters, { pattern: '', qos: 0 }],
    })
  }

  const updateTopicFilter = (index: number, field: string, value: any) => {
    const filters = [...(config.subscriptionMatching?.topicFilters || [])]
    filters[index] = { ...filters[index], [field]: value }
    updateSubscriptionMatching({ topicFilters: filters })
  }

  const removeTopicFilter = (index: number) => {
    const filters = [...(config.subscriptionMatching?.topicFilters || [])]
    filters.splice(index, 1)
    updateSubscriptionMatching({ topicFilters: filters })
  }

  return (
    <div className="space-y-6">
      {/* Topic Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Topic Configuration</h2>
        <div className="space-y-4">
          <div>
            <label className="mb-2 block text-sm font-medium">Topic Pattern</label>
            <input
              type="text"
              value={config.topicPattern || ''}
              onChange={(e) => updateConfig({ topicPattern: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="sensors/+/temperature or devices/#"
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Supports MQTT wildcards: <code className="rounded bg-muted px-1">+</code> (single level) and <code className="rounded bg-muted px-1">#</code> (multi-level)
            </p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium">QoS Level</label>
              <select
                value={config.qos ?? 0}
                onChange={(e) => updateConfig({ qos: parseInt(e.target.value) })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              >
                {QOS_LEVELS.map((level) => (
                  <option key={level.value} value={level.value}>
                    {level.label}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex items-center space-x-2 pt-8">
              <input
                type="checkbox"
                id="retained"
                checked={config.retained ?? false}
                onChange={(e) => updateConfig({ retained: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="retained" className="text-sm font-medium">
                Retained Message
              </label>
              <span className="text-xs text-muted-foreground">
                (Message persists for new subscribers)
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
  "deviceId": "{{topic.1}}",
  "temperature": {{faker.number.float({"min": 20, "max": 30})}},
  "timestamp": "{{now}}",
  "messageId": "{{uuid}}"
}`}
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Available tokens: {'{{'}uuid{'}}'}, {'{{'}now{'}}'}, {'{{'}rand.int{'}}'}, {'{{'}faker.name{'}}'}, {'{{'}topic.N{'}}'}  (Nth segment of topic), etc.
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
                  (Message to send when client connects)
                </span>
              </div>
              {config.connectionBehavior?.onConnect?.enabled && (
                <div className="ml-6 space-y-3">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Topic</label>
                    <input
                      type="text"
                      value={config.connectionBehavior?.onConnect?.topic || ''}
                      onChange={(e) =>
                        updateConnectionBehavior({
                          onConnect: {
                            ...config.connectionBehavior?.onConnect,
                            topic: e.target.value,
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      placeholder="system/welcome"
                    />
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

            {/* On Subscribe */}
            <div>
              <div className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="onSubscribe"
                  checked={!!config.connectionBehavior?.onSubscribe?.enabled}
                  onChange={(e) =>
                    updateConnectionBehavior({
                      onSubscribe: {
                        ...config.connectionBehavior?.onSubscribe,
                        enabled: e.target.checked,
                      },
                    })
                  }
                  className="h-4 w-4 rounded border-input"
                />
                <label htmlFor="onSubscribe" className="text-sm font-medium">
                  On Subscribe
                </label>
                <span className="text-xs text-muted-foreground">
                  (Response when client subscribes to topic)
                </span>
              </div>
              {config.connectionBehavior?.onSubscribe?.enabled && (
                <div className="ml-6 space-y-3">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Payload (JSON)</label>
                    <div className={cn(
                      'rounded-lg border',
                      jsonErrors.onSubscribePayload ? 'border-destructive' : 'border-border'
                    )}>
                      <Editor
                        height="120px"
                        defaultLanguage="json"
                        value={JSON.stringify(config.connectionBehavior?.onSubscribe?.payload || { status: 'subscribed' }, null, 2)}
                        onChange={(value) => {
                          try {
                            const payload = JSON.parse(value || '{}')
                            updateConnectionBehavior({
                              onSubscribe: {
                                ...config.connectionBehavior?.onSubscribe,
                                payload,
                              },
                            })
                            setJsonErrors((prev) => ({ ...prev, onSubscribePayload: null }))
                          } catch (e) {
                            const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                            setJsonErrors((prev) => ({ ...prev, onSubscribePayload: errorMessage }))
                          }
                        }}
                        theme="vs-dark"
                        options={{
                          minimap: { enabled: false },
                          fontSize: 13,
                        }}
                      />
                    </div>
                    {jsonErrors.onSubscribePayload && (
                      <div className="mt-1 flex items-center space-x-2 text-xs text-destructive">
                        <AlertCircle className="h-3 w-3" />
                        <span>{jsonErrors.onSubscribePayload}</span>
                      </div>
                    )}
                  </div>
                  <div className="flex items-center space-x-2">
                    <input
                      type="checkbox"
                      id="sendRetained"
                      checked={config.connectionBehavior?.onSubscribe?.sendRetained ?? true}
                      onChange={(e) =>
                        updateConnectionBehavior({
                          onSubscribe: {
                            ...config.connectionBehavior?.onSubscribe,
                            sendRetained: e.target.checked,
                          },
                        })
                      }
                      className="h-4 w-4 rounded border-input"
                    />
                    <label htmlFor="sendRetained" className="text-xs text-muted-foreground">
                      Send retained message on subscribe
                    </label>
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
                      value={config.connectionBehavior?.onPublish?.responseType || 'echo'}
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
                      <option value="echo">Echo (return received message)</option>
                      <option value="ack">Acknowledge (send confirmation)</option>
                      <option value="transform">Transform (modify and respond)</option>
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
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Response Topic (optional)</label>
                    <input
                      type="text"
                      value={config.connectionBehavior?.onPublish?.responseTopic || ''}
                      onChange={(e) =>
                        updateConnectionBehavior({
                          onPublish: {
                            ...config.connectionBehavior?.onPublish,
                            responseTopic: e.target.value,
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      placeholder="Leave empty to respond on same topic"
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Subscription Matching */}
      <div className="rounded-lg border border-border bg-card p-6">
        <button
          onClick={() => setShowSubscriptionMatching(!showSubscriptionMatching)}
          className="flex w-full items-center justify-between"
        >
          <h2 className="text-lg font-semibold">Subscription Matching</h2>
          {showSubscriptionMatching ? (
            <ChevronDown className="h-5 w-5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-5 w-5 text-muted-foreground" />
          )}
        </button>

        {showSubscriptionMatching && (
          <div className="mt-4 space-y-4">
            {/* Echo Published Messages */}
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="echoPublished"
                checked={config.subscriptionMatching?.echoPublished ?? false}
                onChange={(e) =>
                  updateSubscriptionMatching({ echoPublished: e.target.checked })
                }
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="echoPublished" className="text-sm font-medium">
                Echo published messages back to subscribers
              </label>
            </div>

            {/* Topic Filters */}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <label className="text-sm font-medium">Topic Filters</label>
                <button
                  onClick={addTopicFilter}
                  className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
                >
                  <Plus className="h-3 w-3" />
                  <span>Add Filter</span>
                </button>
              </div>
              {config.subscriptionMatching?.topicFilters?.length > 0 && (
                <div className="space-y-2">
                  {config.subscriptionMatching.topicFilters.map((filter: any, index: number) => (
                    <div key={index} className="flex items-center space-x-2">
                      <input
                        type="text"
                        value={filter.pattern}
                        onChange={(e) => updateTopicFilter(index, 'pattern', e.target.value)}
                        placeholder="Topic filter pattern"
                        className="flex-1 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      />
                      <select
                        value={filter.qos ?? 0}
                        onChange={(e) => updateTopicFilter(index, 'qos', parseInt(e.target.value))}
                        className="w-24 rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      >
                        <option value={0}>QoS 0</option>
                        <option value={1}>QoS 1</option>
                        <option value={2}>QoS 2</option>
                      </select>
                      <button
                        onClick={() => removeTopicFilter(index)}
                        className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
              <p className="mt-2 text-xs text-muted-foreground">
                Define additional topic patterns this mock should match against
              </p>
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
                <label className="mb-2 block text-sm font-medium">Session Expiry (seconds)</label>
                <input
                  type="number"
                  value={config.advanced?.sessionExpiryInterval ?? 0}
                  onChange={(e) =>
                    updateAdvanced({ sessionExpiryInterval: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="0 (no expiry)"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  0 = session ends on disconnect
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Keep Alive (seconds)</label>
                <input
                  type="number"
                  value={config.advanced?.keepAliveInterval ?? 60}
                  onChange={(e) =>
                    updateAdvanced({ keepAliveInterval: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="60"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Client ping interval
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Max Packet Size (bytes)</label>
                <input
                  type="number"
                  value={config.advanced?.maxPacketSize ?? 0}
                  onChange={(e) =>
                    updateAdvanced({ maxPacketSize: parseInt(e.target.value) })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  min="0"
                  placeholder="0 (unlimited)"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  0 = no limit
                </p>
              </div>
            </div>

            {/* Clean Session */}
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="cleanSession"
                checked={config.advanced?.cleanSession ?? true}
                onChange={(e) => updateAdvanced({ cleanSession: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="cleanSession" className="text-sm font-medium">
                Clean Session
              </label>
              <span className="text-xs text-muted-foreground">
                (Discard session state on connect)
              </span>
            </div>

            {/* Message Expiry */}
            <div>
              <label className="mb-2 block text-sm font-medium">Message Expiry (seconds)</label>
              <input
                type="number"
                value={config.advanced?.messageExpiryInterval ?? 0}
                onChange={(e) =>
                  updateAdvanced({ messageExpiryInterval: parseInt(e.target.value) })
                }
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring md:w-48"
                min="0"
                placeholder="0 (no expiry)"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                How long messages should be retained (0 = no expiry)
              </p>
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
