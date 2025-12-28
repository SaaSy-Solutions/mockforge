import { useState, useEffect, useMemo } from 'react'
import { AlertCircle } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { z } from 'zod'
import { cn } from '@/lib/utils'
import EditorSkeleton from '@/components/EditorSkeleton'
import type { GrpcFormProps } from '@/types/protocol-configs'

// Validation schema for gRPC endpoint configuration
const grpcConfigSchema = z.object({
  service: z.string().min(1, 'Service name is required').regex(/^[A-Za-z][A-Za-z0-9]*$/, 'Service name must start with a letter and contain only alphanumeric characters'),
  method: z.string().min(1, 'Method name is required').regex(/^[A-Za-z][A-Za-z0-9]*$/, 'Method name must start with a letter and contain only alphanumeric characters'),
  proto_file: z.string().min(1, 'Proto file is required').regex(/\.proto$/, 'Proto file must end with .proto'),
  request_type: z.string().min(1, 'Request type is required'),
  response_type: z.string().min(1, 'Response type is required'),
})

type ValidationErrors = {
  service?: string
  method?: string
  proto_file?: string
  request_type?: string
  response_type?: string
}

export default function GrpcEndpointForm({ config, onChange, onValidationChange }: GrpcFormProps) {
  const [jsonError, setJsonError] = useState<string | null>(null)
  const [touched, setTouched] = useState<Set<string>>(new Set())

  // Validate all fields
  const validationErrors = useMemo((): ValidationErrors => {
    const result = grpcConfigSchema.safeParse(config)
    if (result.success) return {}

    const errors: ValidationErrors = {}
    for (const issue of result.error.issues) {
      const field = issue.path[0] as keyof ValidationErrors
      if (!errors[field]) {
        errors[field] = issue.message
      }
    }
    return errors
  }, [config])

  // Report validation state to parent when errors change
  useEffect(() => {
    const hasErrors = Object.keys(validationErrors).length > 0 || jsonError !== null
    onValidationChange?.(!hasErrors)
  }, [validationErrors, jsonError, onValidationChange])

  const handleBlur = (field: string) => {
    setTouched(prev => new Set([...prev, field]))
  }

  const getFieldError = (field: keyof ValidationErrors): string | undefined => {
    return touched.has(field) ? validationErrors[field] : undefined
  }

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
            <label htmlFor="grpc-service" className="mb-2 block text-sm font-medium">
              Service Name <span className="text-destructive">*</span>
            </label>
            <input
              id="grpc-service"
              type="text"
              value={config.service}
              onChange={(e) => onChange({ ...config, service: e.target.value })}
              onBlur={() => handleBlur('service')}
              aria-invalid={!!getFieldError('service')}
              aria-describedby={getFieldError('service') ? 'service-error' : undefined}
              className={cn(
                'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                getFieldError('service') ? 'border-destructive' : 'border-input'
              )}
              placeholder="UserService"
            />
            {getFieldError('service') && (
              <p id="service-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
                <AlertCircle className="h-3 w-3" />
                {getFieldError('service')}
              </p>
            )}
          </div>
          <div>
            <label htmlFor="grpc-method" className="mb-2 block text-sm font-medium">
              Method Name <span className="text-destructive">*</span>
            </label>
            <input
              id="grpc-method"
              type="text"
              value={config.method}
              onChange={(e) => onChange({ ...config, method: e.target.value })}
              onBlur={() => handleBlur('method')}
              aria-invalid={!!getFieldError('method')}
              aria-describedby={getFieldError('method') ? 'method-error' : undefined}
              className={cn(
                'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                getFieldError('method') ? 'border-destructive' : 'border-input'
              )}
              placeholder="GetUser"
            />
            {getFieldError('method') && (
              <p id="method-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
                <AlertCircle className="h-3 w-3" />
                {getFieldError('method')}
              </p>
            )}
          </div>
          <div>
            <label htmlFor="grpc-proto" className="mb-2 block text-sm font-medium">
              Proto File <span className="text-destructive">*</span>
            </label>
            <input
              id="grpc-proto"
              type="text"
              value={config.proto_file}
              onChange={(e) => onChange({ ...config, proto_file: e.target.value })}
              onBlur={() => handleBlur('proto_file')}
              aria-invalid={!!getFieldError('proto_file')}
              aria-describedby={getFieldError('proto_file') ? 'proto-error' : undefined}
              className={cn(
                'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                getFieldError('proto_file') ? 'border-destructive' : 'border-input'
              )}
              placeholder="user.proto"
            />
            {getFieldError('proto_file') && (
              <p id="proto-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
                <AlertCircle className="h-3 w-3" />
                {getFieldError('proto_file')}
              </p>
            )}
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <label htmlFor="grpc-request-type" className="mb-2 block text-sm font-medium">
                Request Type <span className="text-destructive">*</span>
              </label>
              <input
                id="grpc-request-type"
                type="text"
                value={config.request_type}
                onChange={(e) => onChange({ ...config, request_type: e.target.value })}
                onBlur={() => handleBlur('request_type')}
                aria-invalid={!!getFieldError('request_type')}
                aria-describedby={getFieldError('request_type') ? 'request-type-error' : undefined}
                className={cn(
                  'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                  getFieldError('request_type') ? 'border-destructive' : 'border-input'
                )}
                placeholder="GetUserRequest"
              />
              {getFieldError('request_type') && (
                <p id="request-type-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
                  <AlertCircle className="h-3 w-3" />
                  {getFieldError('request_type')}
                </p>
              )}
            </div>
            <div>
              <label htmlFor="grpc-response-type" className="mb-2 block text-sm font-medium">
                Response Type <span className="text-destructive">*</span>
              </label>
              <input
                id="grpc-response-type"
                type="text"
                value={config.response_type}
                onChange={(e) => onChange({ ...config, response_type: e.target.value })}
                onBlur={() => handleBlur('response_type')}
                aria-invalid={!!getFieldError('response_type')}
                aria-describedby={getFieldError('response_type') ? 'response-type-error' : undefined}
                className={cn(
                  'w-full rounded-lg border bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                  getFieldError('response_type') ? 'border-destructive' : 'border-input'
                )}
                placeholder="GetUserResponse"
              />
              {getFieldError('response_type') && (
                <p id="response-type-error" className="mt-1 flex items-center gap-1 text-sm text-destructive">
                  <AlertCircle className="h-3 w-3" />
                  {getFieldError('response_type')}
                </p>
              )}
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
              loading={<EditorSkeleton height="300px" />}
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
