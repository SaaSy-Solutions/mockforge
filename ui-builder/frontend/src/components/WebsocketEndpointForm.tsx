import { useState, useEffect, useMemo } from 'react'
import { AlertCircle } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { z } from 'zod'
import { cn } from '@/lib/utils'
import EditorSkeleton from '@/components/EditorSkeleton'
import type { WebsocketFormProps } from '@/types/protocol-configs'

// Validation schema for WebSocket configuration
const websocketConfigSchema = z.object({
  path: z.string().min(1, 'Path is required').regex(/^\//, 'Path must start with /'),
})

interface JsonEditorErrors {
  onConnect: string | null
  onMessageSend: string | null
  onMessageBroadcast: string | null
}

export default function WebsocketEndpointForm({ config, onChange, onValidationChange }: WebsocketFormProps) {
  const [activeTab, setActiveTab] = useState<'connect' | 'message' | 'disconnect'>('connect')
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    onConnect: null,
    onMessageSend: null,
    onMessageBroadcast: null,
  })
  const [touched, setTouched] = useState<Set<string>>(new Set())

  // Validate path field
  const pathError = useMemo((): string | undefined => {
    const result = websocketConfigSchema.safeParse(config)
    if (result.success) return undefined

    const pathIssue = result.error.issues.find(i => i.path[0] === 'path')
    return pathIssue?.message
  }, [config])

  // Report validation state to parent when errors change
  useEffect(() => {
    const hasJsonErrors = Object.values(jsonErrors).some((error) => error !== null)
    const hasPathError = !!pathError
    onValidationChange?.(!hasJsonErrors && !hasPathError)
  }, [jsonErrors, pathError, onValidationChange])

  const handleBlur = (field: string) => {
    setTouched(prev => new Set([...prev, field]))
  }

  const getPathError = (): string | undefined => {
    return touched.has('path') ? pathError : undefined
  }

  const updateAction = (event: 'on_connect' | 'on_message' | 'on_disconnect', action: any) => {
    onChange({
      ...config,
      [event]: action,
    })
  }

  return (
    <div className="space-y-6">
      {/* Path Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">WebSocket Configuration</h2>
        <div>
          <label htmlFor="ws-path" className="mb-2 block text-sm font-medium">
            Path <span className="text-destructive">*</span>
          </label>
          <input
            id="ws-path"
            type="text"
            value={config.path}
            onChange={(e) => onChange({ ...config, path: e.target.value })}
            onBlur={() => handleBlur('path')}
            aria-invalid={!!getPathError()}
            aria-describedby={getPathError() ? 'ws-path-error' : undefined}
            className={cn(
              'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
              getPathError() ? 'border-destructive' : 'border-input'
            )}
            placeholder="/ws"
          />
          {getPathError() && (
            <p id="ws-path-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
              <AlertCircle className="h-3 w-3" />
              {getPathError()}
            </p>
          )}
        </div>
      </div>

      {/* Event Handlers */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Event Handlers</h2>

        {/* Tabs */}
        <div className="mb-4 flex space-x-2 border-b border-border" role="tablist" aria-label="WebSocket event handlers">
          {[
            { id: 'connect', label: 'On Connect' },
            { id: 'message', label: 'On Message' },
            { id: 'disconnect', label: 'On Disconnect' },
          ].map((tab) => (
            <button
              key={tab.id}
              role="tab"
              id={`ws-tab-${tab.id}`}
              aria-selected={activeTab === tab.id}
              aria-controls={`ws-panel-${tab.id}`}
              onClick={() => setActiveTab(tab.id as 'connect' | 'message' | 'disconnect')}
              className={cn(
                'px-4 py-2 text-sm font-medium transition-colors',
                activeTab === tab.id
                  ? 'border-b-2 border-primary text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* On Connect */}
        {activeTab === 'connect' && (
          <div role="tabpanel" id="ws-panel-connect" aria-labelledby="ws-tab-connect" className="space-y-4">
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.on_connect}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateAction('on_connect', {
                        type: 'Send',
                        message: { type: 'Static', content: { message: 'Connected' } },
                      })
                    } else {
                      onChange({ ...config, on_connect: undefined })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Send message on connect</span>
              </label>
            </div>
            {config.on_connect && (
              <div>
                <label className="mb-2 block text-sm font-medium">Message</label>
                <div className={cn(
                  'rounded-lg border',
                  jsonErrors.onConnect ? 'border-destructive' : 'border-border'
                )}>
                  <Editor
                    height="200px"
                    defaultLanguage="json"
                    value={JSON.stringify(config.on_connect?.message?.content || {}, null, 2)}
                    onChange={(value) => {
                      try {
                        const content = JSON.parse(value || '{}')
                        updateAction('on_connect', {
                          type: 'Send',
                          message: { type: 'Static', content },
                        })
                        setJsonErrors((prev) => ({ ...prev, onConnect: null }))
                      } catch (e) {
                        const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                        setJsonErrors((prev) => ({ ...prev, onConnect: errorMessage }))
                      }
                    }}
                    theme="vs-dark"
                    loading={<EditorSkeleton height="200px" />}
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
                {jsonErrors.onConnect && (
                  <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    <span>Invalid JSON: {jsonErrors.onConnect}</span>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* On Message */}
        {activeTab === 'message' && (
          <div role="tabpanel" id="ws-panel-message" aria-labelledby="ws-tab-message" className="space-y-4">
            <div>
              <label htmlFor="ws-message-action" className="mb-2 block text-sm font-medium">Action</label>
              <select
                id="ws-message-action"
                value={config.on_message?.type || 'none'}
                onChange={(e) => {
                  if (e.target.value === 'none') {
                    onChange({ ...config, on_message: undefined })
                  } else if (e.target.value === 'Echo') {
                    updateAction('on_message', { type: 'Echo' })
                  } else {
                    updateAction('on_message', {
                      type: e.target.value,
                      message: { type: 'Static', content: {} },
                    })
                  }
                }}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              >
                <option value="none">None</option>
                <option value="Echo">Echo back</option>
                <option value="Send">Send response</option>
                <option value="Broadcast">Broadcast to all</option>
              </select>
            </div>
            {config.on_message?.type === 'Send' && (
              <div>
                <label className="mb-2 block text-sm font-medium">Response Message</label>
                <div className={cn(
                  'rounded-lg border',
                  jsonErrors.onMessageSend ? 'border-destructive' : 'border-border'
                )}>
                  <Editor
                    height="200px"
                    defaultLanguage="json"
                    value={JSON.stringify(config.on_message?.message?.content || {}, null, 2)}
                    onChange={(value) => {
                      try {
                        const content = JSON.parse(value || '{}')
                        updateAction('on_message', {
                          type: 'Send',
                          message: { type: 'Static', content },
                        })
                        setJsonErrors((prev) => ({ ...prev, onMessageSend: null }))
                      } catch (e) {
                        const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                        setJsonErrors((prev) => ({ ...prev, onMessageSend: errorMessage }))
                      }
                    }}
                    theme="vs-dark"
                    loading={<EditorSkeleton height="200px" />}
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
                {jsonErrors.onMessageSend && (
                  <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    <span>Invalid JSON: {jsonErrors.onMessageSend}</span>
                  </div>
                )}
              </div>
            )}
            {config.on_message?.type === 'Broadcast' && (
              <div>
                <label className="mb-2 block text-sm font-medium">Broadcast Message</label>
                <div className={cn(
                  'rounded-lg border',
                  jsonErrors.onMessageBroadcast ? 'border-destructive' : 'border-border'
                )}>
                  <Editor
                    height="200px"
                    defaultLanguage="json"
                    value={JSON.stringify(config.on_message?.message?.content || {}, null, 2)}
                    onChange={(value) => {
                      try {
                        const content = JSON.parse(value || '{}')
                        updateAction('on_message', {
                          type: 'Broadcast',
                          message: { type: 'Static', content },
                        })
                        setJsonErrors((prev) => ({ ...prev, onMessageBroadcast: null }))
                      } catch (e) {
                        const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                        setJsonErrors((prev) => ({ ...prev, onMessageBroadcast: errorMessage }))
                      }
                    }}
                    theme="vs-dark"
                    loading={<EditorSkeleton height="200px" />}
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
                {jsonErrors.onMessageBroadcast && (
                  <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    <span>Invalid JSON: {jsonErrors.onMessageBroadcast}</span>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* On Disconnect */}
        {activeTab === 'disconnect' && (
          <div role="tabpanel" id="ws-panel-disconnect" aria-labelledby="ws-tab-disconnect" className="space-y-4">
            <p className="text-sm text-muted-foreground">
              No action needed for disconnect events (handled automatically)
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
