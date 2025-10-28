import { useState } from 'react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'

interface WebsocketEndpointFormProps {
  config: any
  onChange: (config: any) => void
}

export default function WebsocketEndpointForm({ config, onChange }: WebsocketEndpointFormProps) {
  const [activeTab, setActiveTab] = useState<'connect' | 'message' | 'disconnect'>('connect')

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
          <label className="mb-2 block text-sm font-medium">Path</label>
          <input
            type="text"
            value={config.path}
            onChange={(e) => onChange({ ...config, path: e.target.value })}
            className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            placeholder="/ws"
          />
        </div>
      </div>

      {/* Event Handlers */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Event Handlers</h2>

        {/* Tabs */}
        <div className="mb-4 flex space-x-2 border-b border-border">
          {[
            { id: 'connect', label: 'On Connect' },
            { id: 'message', label: 'On Message' },
            { id: 'disconnect', label: 'On Disconnect' },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={cn(
                'px-4 py-2 text-sm font-medium',
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
          <div className="space-y-4">
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
                <div className="rounded-lg border border-border">
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
                      } catch (e) {
                        // Invalid JSON
                      }
                    }}
                    theme="vs-dark"
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
              </div>
            )}
          </div>
        )}

        {/* On Message */}
        {activeTab === 'message' && (
          <div className="space-y-4">
            <div>
              <label className="mb-2 block text-sm font-medium">Action</label>
              <select
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
                <div className="rounded-lg border border-border">
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
                      } catch (e) {
                        // Invalid JSON
                      }
                    }}
                    theme="vs-dark"
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
              </div>
            )}
            {config.on_message?.type === 'Broadcast' && (
              <div>
                <label className="mb-2 block text-sm font-medium">Broadcast Message</label>
                <div className="rounded-lg border border-border">
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
                      } catch (e) {
                        // Invalid JSON
                      }
                    }}
                    theme="vs-dark"
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
              </div>
            )}
          </div>
        )}

        {/* On Disconnect */}
        {activeTab === 'disconnect' && (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              No action needed for disconnect events (handled automatically)
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
