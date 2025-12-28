import { useState, useEffect } from 'react'
import { AlertCircle } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import type { GrpcFormProps } from '@/types/protocol-configs'

export default function GrpcEndpointForm({ config, onChange, onValidationChange }: GrpcFormProps) {
  const [jsonError, setJsonError] = useState<string | null>(null)

  // Report validation state to parent when error changes
  useEffect(() => {
    onValidationChange?.(jsonError === null)
  }, [jsonError, onValidationChange])

  const updateResponse = (updates: any) => {
    onChange({
      ...config,
      response: {
        ...config.response,
        ...updates,
      },
    })
  }

  return (
    <div className="space-y-6">
      {/* Service Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">gRPC Service</h2>
        <div className="space-y-4">
          <div>
            <label className="mb-2 block text-sm font-medium">Service Name</label>
            <input
              type="text"
              value={config.service}
              onChange={(e) => onChange({ ...config, service: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="UserService"
            />
          </div>
          <div>
            <label className="mb-2 block text-sm font-medium">Method Name</label>
            <input
              type="text"
              value={config.method}
              onChange={(e) => onChange({ ...config, method: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="GetUser"
            />
          </div>
          <div>
            <label className="mb-2 block text-sm font-medium">Proto File</label>
            <input
              type="text"
              value={config.proto_file}
              onChange={(e) => onChange({ ...config, proto_file: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="user.proto"
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium">Request Type</label>
              <input
                type="text"
                value={config.request_type}
                onChange={(e) => onChange({ ...config, request_type: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="GetUserRequest"
              />
            </div>
            <div>
              <label className="mb-2 block text-sm font-medium">Response Type</label>
              <input
                type="text"
                value={config.response_type}
                onChange={(e) => onChange({ ...config, response_type: e.target.value })}
                className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="GetUserResponse"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Response Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Response</h2>
        <div>
          <label className="mb-2 block text-sm font-medium">Response Body (JSON)</label>
          <div className={cn(
            'rounded-lg border',
            jsonError ? 'border-destructive' : 'border-border'
          )}>
            <Editor
              height="300px"
              defaultLanguage="json"
              value={JSON.stringify(config.response?.body?.content || {}, null, 2)}
              onChange={(value) => {
                try {
                  const content = JSON.parse(value || '{}')
                  updateResponse({ body: { type: 'Static', content } })
                  setJsonError(null)
                } catch (e) {
                  const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                  setJsonError(errorMessage)
                }
              }}
              theme="vs-dark"
              options={{
                minimap: { enabled: false },
                fontSize: 13,
              }}
            />
          </div>
          {jsonError && (
            <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
              <AlertCircle className="h-4 w-4" />
              <span>Invalid JSON: {jsonError}</span>
            </div>
          )}
          <p className="mt-2 text-xs text-muted-foreground">
            Define the gRPC response message as JSON. It will be converted to protobuf format.
          </p>
        </div>
      </div>
    </div>
  )
}
