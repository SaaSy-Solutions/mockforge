import { useState, useEffect } from 'react'
import { Plus, X, AlertCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { SmtpFormProps } from '@/types/protocol-configs'

interface Credential {
  username: string
  password: string
}

interface EmailPattern {
  pattern: string
  type: 'allow' | 'block'
}

const TLS_OPTIONS = [
  { id: 'none', label: 'None', description: 'No encryption' },
  { id: 'starttls', label: 'STARTTLS', description: 'Upgrade connection to TLS' },
  { id: 'implicit', label: 'Implicit TLS', description: 'TLS from the start' },
]

const AUTH_MECHANISMS = ['PLAIN', 'LOGIN', 'CRAM-MD5']

const DEFAULT_RESPONSES = [
  { id: 'accept', label: 'Accept', description: 'Accept all messages (250 OK)' },
  { id: 'reject', label: 'Reject', description: 'Reject messages (550 Rejected)' },
  { id: 'bounce', label: 'Bounce', description: 'Bounce messages (452 Try again)' },
]

export default function SmtpEndpointForm({ config, onChange, onValidationChange }: SmtpFormProps) {
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [validationErrors, setValidationErrors] = useState<string[]>([])

  // Validate configuration
  useEffect(() => {
    const errors: string[] = []

    // Validate port
    const port = config.port || 25
    if (port < 1 || port > 65535) {
      errors.push('Port must be between 1 and 65535')
    }

    // Validate max messages
    if (config.messageStorage?.enabled && config.messageStorage?.maxMessages < 1) {
      errors.push('Max messages must be at least 1')
    }

    // Validate latency
    if (config.behavior?.latency?.enabled) {
      if (config.behavior.latency.minMs < 0) {
        errors.push('Min latency must be non-negative')
      }
      if (config.behavior.latency.maxMs < config.behavior.latency.minMs) {
        errors.push('Max latency must be greater than or equal to min latency')
      }
    }

    setValidationErrors(errors)
    onValidationChange?.(errors.length === 0)
  }, [config, onValidationChange])

  const updateConfig = (updates: any) => {
    onChange({
      ...config,
      ...updates,
    })
  }

  const updateAuth = (updates: any) => {
    onChange({
      ...config,
      authentication: {
        ...config.authentication,
        ...updates,
      },
    })
  }

  const updateMessageHandling = (updates: any) => {
    onChange({
      ...config,
      messageHandling: {
        ...config.messageHandling,
        ...updates,
      },
    })
  }

  const updateBehavior = (updates: any) => {
    onChange({
      ...config,
      behavior: {
        ...config.behavior,
        ...updates,
      },
    })
  }

  const updateMessageStorage = (updates: any) => {
    onChange({
      ...config,
      messageStorage: {
        ...config.messageStorage,
        ...updates,
      },
    })
  }

  // Credentials management
  const addCredential = () => {
    const credentials = config.authentication?.credentials || []
    updateAuth({
      credentials: [...credentials, { username: '', password: '' }],
    })
  }

  const updateCredential = (index: number, field: keyof Credential, value: string) => {
    const credentials = [...(config.authentication?.credentials || [])]
    credentials[index] = { ...credentials[index], [field]: value }
    updateAuth({ credentials })
  }

  const removeCredential = (index: number) => {
    const credentials = [...(config.authentication?.credentials || [])]
    credentials.splice(index, 1)
    updateAuth({ credentials })
  }

  // Sender filter management
  const addSenderFilter = () => {
    const senderFilters = config.messageHandling?.senderFilters || []
    updateMessageHandling({
      senderFilters: [...senderFilters, { pattern: '', type: 'allow' }],
    })
  }

  const updateSenderFilter = (index: number, field: keyof EmailPattern, value: string) => {
    const senderFilters = [...(config.messageHandling?.senderFilters || [])]
    senderFilters[index] = { ...senderFilters[index], [field]: value }
    updateMessageHandling({ senderFilters })
  }

  const removeSenderFilter = (index: number) => {
    const senderFilters = [...(config.messageHandling?.senderFilters || [])]
    senderFilters.splice(index, 1)
    updateMessageHandling({ senderFilters })
  }

  // Recipient filter management
  const addRecipientFilter = () => {
    const recipientFilters = config.messageHandling?.recipientFilters || []
    updateMessageHandling({
      recipientFilters: [...recipientFilters, { pattern: '', type: 'allow' }],
    })
  }

  const updateRecipientFilter = (index: number, field: keyof EmailPattern, value: string) => {
    const recipientFilters = [...(config.messageHandling?.recipientFilters || [])]
    recipientFilters[index] = { ...recipientFilters[index], [field]: value }
    updateMessageHandling({ recipientFilters })
  }

  const removeRecipientFilter = (index: number) => {
    const recipientFilters = [...(config.messageHandling?.recipientFilters || [])]
    recipientFilters.splice(index, 1)
    updateMessageHandling({ recipientFilters })
  }

  return (
    <div className="space-y-6">
      {/* Validation Errors */}
      {validationErrors.length > 0 && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4">
          <div className="flex items-center space-x-2 text-destructive">
            <AlertCircle className="h-5 w-5" />
            <span className="font-medium">Configuration Errors</span>
          </div>
          <ul className="mt-2 list-inside list-disc text-sm text-destructive">
            {validationErrors.map((error, index) => (
              <li key={index}>{error}</li>
            ))}
          </ul>
        </div>
      )}

      {/* Server Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Server Configuration</h2>
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium">Port</label>
              <input
                type="number"
                value={config.port || 25}
                onChange={(e) => updateConfig({ port: parseInt(e.target.value) || 25 })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="25"
                min="1"
                max="65535"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Common ports: 25 (SMTP), 587 (Submission), 465 (SMTPS)
              </p>
            </div>
            <div>
              <label className="mb-2 block text-sm font-medium">Hostname (HELO/EHLO)</label>
              <input
                type="text"
                value={config.hostname || ''}
                onChange={(e) => updateConfig({ hostname: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="localhost"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Server hostname used in SMTP greeting
              </p>
            </div>
          </div>

          {/* TLS/SSL Settings */}
          <div>
            <label className="mb-2 block text-sm font-medium">TLS/SSL Mode</label>
            <div className="grid gap-2 md:grid-cols-3">
              {TLS_OPTIONS.map((option) => (
                <button
                  key={option.id}
                  onClick={() => updateConfig({ tlsMode: option.id })}
                  className={cn(
                    'rounded-lg border-2 p-3 text-left transition-all',
                    (config.tlsMode || 'none') === option.id
                      ? 'border-primary bg-primary/5'
                      : 'border-border bg-background hover:border-primary/50'
                  )}
                >
                  <div className="font-medium">{option.label}</div>
                  <div className="text-xs text-muted-foreground">{option.description}</div>
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Authentication Mock */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Authentication</h2>
        <div className="space-y-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="auth-enabled"
              checked={config.authentication?.enabled || false}
              onChange={(e) => {
                if (e.target.checked) {
                  updateAuth({
                    enabled: true,
                    mechanisms: ['PLAIN'],
                    credentials: [],
                  })
                } else {
                  updateAuth({ enabled: false })
                }
              }}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="auth-enabled" className="text-sm font-medium">
              Enable Authentication
            </label>
          </div>

          {config.authentication?.enabled && (
            <div className="space-y-4 pl-6">
              {/* Auth Mechanisms */}
              <div>
                <label className="mb-2 block text-sm font-medium">Auth Mechanisms</label>
                <div className="flex flex-wrap gap-2">
                  {AUTH_MECHANISMS.map((mechanism) => {
                    const isSelected = (config.authentication?.mechanisms || []).includes(mechanism)
                    return (
                      <button
                        key={mechanism}
                        onClick={() => {
                          const mechanisms = config.authentication?.mechanisms || []
                          if (isSelected) {
                            updateAuth({
                              mechanisms: mechanisms.filter((m: string) => m !== mechanism),
                            })
                          } else {
                            updateAuth({
                              mechanisms: [...mechanisms, mechanism],
                            })
                          }
                        }}
                        className={cn(
                          'rounded-lg px-3 py-1.5 text-sm font-medium transition-colors',
                          isSelected
                            ? 'bg-primary text-primary-foreground'
                            : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
                        )}
                      >
                        {mechanism}
                      </button>
                    )
                  })}
                </div>
              </div>

              {/* Accepted Credentials */}
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <label className="text-sm font-medium">Accepted Credentials</label>
                  <button
                    onClick={addCredential}
                    className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
                  >
                    <Plus className="h-3 w-3" />
                    <span>Add Credential</span>
                  </button>
                </div>
                {(!config.authentication?.credentials || config.authentication.credentials.length === 0) && (
                  <p className="text-sm text-muted-foreground">
                    No credentials configured. Any login will be accepted.
                  </p>
                )}
                {config.authentication?.credentials?.length > 0 && (
                  <div className="space-y-2">
                    {config.authentication.credentials.map((cred: Credential, index: number) => (
                      <div key={index} className="flex items-center space-x-2">
                        <input
                          type="text"
                          value={cred.username}
                          onChange={(e) => updateCredential(index, 'username', e.target.value)}
                          placeholder="Username"
                          className="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                        <input
                          type="password"
                          value={cred.password}
                          onChange={(e) => updateCredential(index, 'password', e.target.value)}
                          placeholder="Password"
                          className="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                        <button
                          onClick={() => removeCredential(index)}
                          className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                        >
                          <X className="h-4 w-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Message Handling */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Message Handling</h2>
        <div className="space-y-4">
          {/* Accept All Toggle */}
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="accept-all"
              checked={config.messageHandling?.acceptAll !== false}
              onChange={(e) => updateMessageHandling({ acceptAll: e.target.checked })}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="accept-all" className="text-sm font-medium">
              Accept All Messages
            </label>
            <span className="text-xs text-muted-foreground">
              (When disabled, use filters below)
            </span>
          </div>

          {!config.messageHandling?.acceptAll && (
            <div className="space-y-4 pl-6">
              {/* Sender Filters */}
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <label className="text-sm font-medium">Sender Filters</label>
                  <button
                    onClick={addSenderFilter}
                    className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
                  >
                    <Plus className="h-3 w-3" />
                    <span>Add Filter</span>
                  </button>
                </div>
                {(!config.messageHandling?.senderFilters || config.messageHandling.senderFilters.length === 0) && (
                  <p className="text-sm text-muted-foreground">
                    No sender filters. All senders will be processed.
                  </p>
                )}
                {config.messageHandling?.senderFilters?.length > 0 && (
                  <div className="space-y-2">
                    {config.messageHandling.senderFilters.map((filter: EmailPattern, index: number) => (
                      <div key={index} className="flex items-center space-x-2">
                        <select
                          value={filter.type}
                          onChange={(e) => updateSenderFilter(index, 'type', e.target.value)}
                          className="rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        >
                          <option value="allow">Allow</option>
                          <option value="block">Block</option>
                        </select>
                        <input
                          type="text"
                          value={filter.pattern}
                          onChange={(e) => updateSenderFilter(index, 'pattern', e.target.value)}
                          placeholder="*@example.com"
                          className="flex-1 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                        <button
                          onClick={() => removeSenderFilter(index)}
                          className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                        >
                          <X className="h-4 w-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
                <p className="mt-1 text-xs text-muted-foreground">
                  Use * as wildcard (e.g., *@domain.com, user@*)
                </p>
              </div>

              {/* Recipient Filters */}
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <label className="text-sm font-medium">Recipient Filters</label>
                  <button
                    onClick={addRecipientFilter}
                    className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
                  >
                    <Plus className="h-3 w-3" />
                    <span>Add Filter</span>
                  </button>
                </div>
                {(!config.messageHandling?.recipientFilters || config.messageHandling.recipientFilters.length === 0) && (
                  <p className="text-sm text-muted-foreground">
                    No recipient filters. All recipients will be processed.
                  </p>
                )}
                {config.messageHandling?.recipientFilters?.length > 0 && (
                  <div className="space-y-2">
                    {config.messageHandling.recipientFilters.map((filter: EmailPattern, index: number) => (
                      <div key={index} className="flex items-center space-x-2">
                        <select
                          value={filter.type}
                          onChange={(e) => updateRecipientFilter(index, 'type', e.target.value)}
                          className="rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        >
                          <option value="allow">Allow</option>
                          <option value="block">Block</option>
                        </select>
                        <input
                          type="text"
                          value={filter.pattern}
                          onChange={(e) => updateRecipientFilter(index, 'pattern', e.target.value)}
                          placeholder="*@example.com"
                          className="flex-1 rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                        <button
                          onClick={() => removeRecipientFilter(index)}
                          className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                        >
                          <X className="h-4 w-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
                <p className="mt-1 text-xs text-muted-foreground">
                  Use * as wildcard (e.g., *@domain.com, user@*)
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Response Behavior */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Response Behavior</h2>
        <div className="space-y-4">
          {/* Default Response */}
          <div>
            <label className="mb-2 block text-sm font-medium">Default Response</label>
            <div className="grid gap-2 md:grid-cols-3">
              {DEFAULT_RESPONSES.map((response) => (
                <button
                  key={response.id}
                  onClick={() => updateBehavior({ defaultResponse: response.id })}
                  className={cn(
                    'rounded-lg border-2 p-3 text-left transition-all',
                    (config.behavior?.defaultResponse || 'accept') === response.id
                      ? 'border-primary bg-primary/5'
                      : 'border-border bg-background hover:border-primary/50'
                  )}
                >
                  <div className="font-medium">{response.label}</div>
                  <div className="text-xs text-muted-foreground">{response.description}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Custom SMTP Response Code */}
          <div>
            <label className="mb-2 flex items-center space-x-2">
              <input
                type="checkbox"
                checked={config.behavior?.customResponseCode?.enabled || false}
                onChange={(e) => {
                  if (e.target.checked) {
                    updateBehavior({
                      customResponseCode: {
                        enabled: true,
                        code: 250,
                        message: 'OK',
                      },
                    })
                  } else {
                    updateBehavior({
                      customResponseCode: { enabled: false },
                    })
                  }
                }}
                className="h-4 w-4 rounded border-input"
              />
              <span className="text-sm font-medium">Custom SMTP Response Code</span>
            </label>
            {config.behavior?.customResponseCode?.enabled && (
              <div className="mt-2 grid gap-4 md:grid-cols-2">
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">Response Code</label>
                  <input
                    type="number"
                    value={config.behavior.customResponseCode.code || 250}
                    onChange={(e) =>
                      updateBehavior({
                        customResponseCode: {
                          ...config.behavior.customResponseCode,
                          code: parseInt(e.target.value) || 250,
                        },
                      })
                    }
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    min="200"
                    max="599"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">Response Message</label>
                  <input
                    type="text"
                    value={config.behavior.customResponseCode.message || ''}
                    onChange={(e) =>
                      updateBehavior({
                        customResponseCode: {
                          ...config.behavior.customResponseCode,
                          message: e.target.value,
                        },
                      })
                    }
                    placeholder="OK"
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                </div>
              </div>
            )}
          </div>

          {/* Latency Simulation */}
          <div>
            <label className="mb-2 flex items-center space-x-2">
              <input
                type="checkbox"
                checked={config.behavior?.latency?.enabled || false}
                onChange={(e) => {
                  if (e.target.checked) {
                    updateBehavior({
                      latency: {
                        enabled: true,
                        minMs: 100,
                        maxMs: 500,
                      },
                    })
                  } else {
                    updateBehavior({
                      latency: { enabled: false },
                    })
                  }
                }}
                className="h-4 w-4 rounded border-input"
              />
              <span className="text-sm font-medium">Simulate Latency</span>
            </label>
            {config.behavior?.latency?.enabled && (
              <div className="mt-2 grid gap-4 md:grid-cols-2">
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">Min Latency (ms)</label>
                  <input
                    type="number"
                    value={config.behavior.latency.minMs || 0}
                    onChange={(e) =>
                      updateBehavior({
                        latency: {
                          ...config.behavior.latency,
                          minMs: parseInt(e.target.value) || 0,
                        },
                      })
                    }
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    min="0"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">Max Latency (ms)</label>
                  <input
                    type="number"
                    value={config.behavior.latency.maxMs || 0}
                    onChange={(e) =>
                      updateBehavior({
                        latency: {
                          ...config.behavior.latency,
                          maxMs: parseInt(e.target.value) || 0,
                        },
                      })
                    }
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    min="0"
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Message Storage */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Message Storage</h2>
        <div className="space-y-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="storage-enabled"
              checked={config.messageStorage?.enabled !== false}
              onChange={(e) => {
                if (e.target.checked) {
                  updateMessageStorage({
                    enabled: true,
                    maxMessages: 100,
                    apiPath: '/api/smtp/messages',
                  })
                } else {
                  updateMessageStorage({ enabled: false })
                }
              }}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="storage-enabled" className="text-sm font-medium">
              Store Received Messages
            </label>
          </div>

          {config.messageStorage?.enabled !== false && (
            <div className="space-y-4 pl-6">
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <label className="mb-2 block text-sm font-medium">Max Messages</label>
                  <input
                    type="number"
                    value={config.messageStorage?.maxMessages || 100}
                    onChange={(e) =>
                      updateMessageStorage({
                        maxMessages: parseInt(e.target.value) || 100,
                      })
                    }
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    min="1"
                  />
                  <p className="mt-1 text-xs text-muted-foreground">
                    Oldest messages are removed when limit is reached
                  </p>
                </div>
                <div>
                  <label className="mb-2 block text-sm font-medium">API Path</label>
                  <input
                    type="text"
                    value={config.messageStorage?.apiPath || '/api/smtp/messages'}
                    onChange={(e) =>
                      updateMessageStorage({
                        apiPath: e.target.value,
                      })
                    }
                    className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    placeholder="/api/smtp/messages"
                  />
                  <p className="mt-1 text-xs text-muted-foreground">
                    Endpoint to retrieve stored messages
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Advanced Settings */}
      <div className="rounded-lg border border-border bg-card p-6">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Advanced Settings</h2>
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="text-sm text-primary hover:underline"
          >
            {showAdvanced ? 'Hide' : 'Show'}
          </button>
        </div>

        {showAdvanced && (
          <div className="space-y-4">
            {/* Max Message Size */}
            <div>
              <label className="mb-2 block text-sm font-medium">Max Message Size (bytes)</label>
              <input
                type="number"
                value={config.maxMessageSize || 10485760}
                onChange={(e) => updateConfig({ maxMessageSize: parseInt(e.target.value) || 10485760 })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring md:w-48"
                min="1024"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Default: 10MB (10485760 bytes)
              </p>
            </div>

            {/* Max Recipients */}
            <div>
              <label className="mb-2 block text-sm font-medium">Max Recipients per Message</label>
              <input
                type="number"
                value={config.maxRecipients || 100}
                onChange={(e) => updateConfig({ maxRecipients: parseInt(e.target.value) || 100 })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring md:w-48"
                min="1"
              />
            </div>

            {/* Connection Timeout */}
            <div>
              <label className="mb-2 block text-sm font-medium">Connection Timeout (seconds)</label>
              <input
                type="number"
                value={config.connectionTimeout || 300}
                onChange={(e) => updateConfig({ connectionTimeout: parseInt(e.target.value) || 300 })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring md:w-48"
                min="1"
              />
              <p className="mt-1 text-xs text-muted-foreground">
                Default: 300 seconds (5 minutes)
              </p>
            </div>

            {/* Require HELO/EHLO */}
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="require-helo"
                checked={config.requireHelo !== false}
                onChange={(e) => updateConfig({ requireHelo: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="require-helo" className="text-sm font-medium">
                Require HELO/EHLO Command
              </label>
            </div>

            {/* Enable VRFY */}
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="enable-vrfy"
                checked={config.enableVrfy || false}
                onChange={(e) => updateConfig({ enableVrfy: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="enable-vrfy" className="text-sm font-medium">
                Enable VRFY Command
              </label>
              <span className="text-xs text-muted-foreground">
                (Verify email address)
              </span>
            </div>

            {/* Enable EXPN */}
            <div className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="enable-expn"
                checked={config.enableExpn || false}
                onChange={(e) => updateConfig({ enableExpn: e.target.checked })}
                className="h-4 w-4 rounded border-input"
              />
              <label htmlFor="enable-expn" className="text-sm font-medium">
                Enable EXPN Command
              </label>
              <span className="text-xs text-muted-foreground">
                (Expand mailing list)
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
