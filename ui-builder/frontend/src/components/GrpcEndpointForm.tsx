import Editor from '@monaco-editor/react'

interface GrpcEndpointFormProps {
  config: any
  onChange: (config: any) => void
}

export default function GrpcEndpointForm({ config, onChange }: GrpcEndpointFormProps) {
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
          <div className="rounded-lg border border-border">
            <Editor
              height="300px"
              defaultLanguage="json"
              value={JSON.stringify(config.response?.body?.content || {}, null, 2)}
              onChange={(value) => {
                try {
                  const content = JSON.parse(value || '{}')
                  updateResponse({ body: { type: 'Static', content } })
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
          <p className="mt-2 text-xs text-muted-foreground">
            Define the gRPC response message as JSON. It will be converted to protobuf format.
          </p>
        </div>
      </div>
    </div>
  )
}
