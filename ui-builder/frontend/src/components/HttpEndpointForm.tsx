import { useState, useEffect, useCallback } from 'react'
import { Plus, X, Code, Sparkles, AlertCircle } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import { pathSchema, statusCodeSchema } from '@/lib/validation'
import EditorSkeleton from '@/components/EditorSkeleton'
import type { HttpFormProps } from '@/types/protocol-configs'
import type { HeaderConfig } from '@/lib/api'

interface JsonEditorErrors {
  staticBody: string | null
  fakerSchema: string | null
}

interface FieldErrors {
  path?: string
  status?: string
}

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

export default function HttpEndpointForm({ config, onChange, onValidationChange }: HttpFormProps) {
  const [bodyType, setBodyType] = useState<'static' | 'template' | 'faker' | 'ai'>(
    config.response?.body?.type?.toLowerCase() || 'static'
  )
  const [showBehavior, setShowBehavior] = useState(false)
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    staticBody: null,
    fakerSchema: null,
  })
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})
  const [touchedFields, setTouchedFields] = useState<Set<string>>(new Set())

  const handleFieldBlur = useCallback((field: string) => {
    setTouchedFields((prev) => new Set(prev).add(field))
  }, [])

  const validatePath = useCallback((value: string) => {
    const result = pathSchema.safeParse(value)
    if (!result.success) {
      setFieldErrors((prev) => ({ ...prev, path: result.error.issues[0]?.message }))
      return false
    }
    setFieldErrors((prev) => {
      const { path: _path, ...rest } = prev
      return rest
    })
    return true
  }, [])

  const validateStatus = useCallback((value: number) => {
    const result = statusCodeSchema.safeParse(value)
    if (!result.success) {
      setFieldErrors((prev) => ({ ...prev, status: result.error.issues[0]?.message }))
      return false
    }
    setFieldErrors((prev) => {
      const { status: _status, ...rest } = prev
      return rest
    })
    return true
  }, [])

  // Report validation state to parent when errors change
  useEffect(() => {
    const hasJsonErrors = Object.values(jsonErrors).some((error) => error !== null)
    const hasFieldErrs = Object.keys(fieldErrors).length > 0
    onValidationChange?.(!hasJsonErrors && !hasFieldErrs)
  }, [jsonErrors, fieldErrors, onValidationChange])

  const updateResponse = (updates: any) => {
    onChange({
      ...config,
      response: {
        ...config.response,
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

  const addHeader = () => {
    const headers = config.response?.headers || []
    updateResponse({
      headers: [...headers, { name: '', value: '' }],
    })
  }

  const updateHeader = (index: number, field: 'name' | 'value', value: string) => {
    const headers = [...(config.response?.headers || [])]
    headers[index] = { ...headers[index], [field]: value }
    updateResponse({ headers })
  }

  const removeHeader = (index: number) => {
    const headers = [...(config.response?.headers || [])]
    headers.splice(index, 1)
    updateResponse({ headers })
  }

  return (
    <div className="space-y-6">
      {/* Request Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Request</h2>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <label className="mb-2 block text-sm font-medium">HTTP Method</label>
            <select
              value={config.method}
              onChange={(e) => onChange({ ...config, method: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              {HTTP_METHODS.map((method) => (
                <option key={method} value={method}>
                  {method}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="http-path" className="mb-2 block text-sm font-medium">
              Path <span className="text-destructive">*</span>
            </label>
            <input
              id="http-path"
              type="text"
              value={config.path}
              onChange={(e) => {
                onChange({ ...config, path: e.target.value })
                if (touchedFields.has('path')) {
                  validatePath(e.target.value)
                }
              }}
              onBlur={() => {
                handleFieldBlur('path')
                validatePath(config.path)
              }}
              className={cn(
                'w-full rounded-lg border px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring',
                touchedFields.has('path') && fieldErrors.path
                  ? 'border-destructive bg-destructive/5'
                  : 'border-input bg-background'
              )}
              placeholder="/api/users"
              aria-invalid={touchedFields.has('path') && !!fieldErrors.path}
              aria-describedby={fieldErrors.path ? 'path-error' : undefined}
            />
            {touchedFields.has('path') && fieldErrors.path && (
              <p id="path-error" className="mt-1 text-sm text-destructive">
                {fieldErrors.path}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Response Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Response</h2>

        {/* Status Code */}
        <div className="mb-4">
          <label htmlFor="http-status" className="mb-2 block text-sm font-medium">
            Status Code <span className="text-destructive">*</span>
          </label>
          <input
            id="http-status"
            type="number"
            value={config.response?.status || 200}
            onChange={(e) => {
              const value = parseInt(e.target.value)
              updateResponse({ status: value })
              if (touchedFields.has('status')) {
                validateStatus(value)
              }
            }}
            onBlur={() => {
              handleFieldBlur('status')
              validateStatus(config.response?.status || 200)
            }}
            className={cn(
              'w-full rounded-lg border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring md:w-32',
              touchedFields.has('status') && fieldErrors.status
                ? 'border-destructive bg-destructive/5'
                : 'border-input bg-background'
            )}
            min="100"
            max="599"
            aria-invalid={touchedFields.has('status') && !!fieldErrors.status}
            aria-describedby={fieldErrors.status ? 'status-error' : undefined}
          />
          {touchedFields.has('status') && fieldErrors.status && (
            <p id="status-error" className="mt-1 text-sm text-destructive">
              {fieldErrors.status}
            </p>
          )}
        </div>

        {/* Headers */}
        <div className="mb-4">
          <div className="mb-2 flex items-center justify-between">
            <label className="text-sm font-medium">Headers</label>
            <button
              onClick={addHeader}
              className="inline-flex items-center space-x-1 text-xs text-primary hover:underline"
            >
              <Plus className="h-3 w-3" />
              <span>Add Header</span>
            </button>
          </div>
          {config.response?.headers?.length > 0 && (
            <div className="space-y-2">
              {config.response.headers.map((header: HeaderConfig, index: number) => (
                <div key={index} className="flex items-center space-x-2">
                  <input
                    type="text"
                    value={header.name}
                    onChange={(e) => updateHeader(index, 'name', e.target.value)}
                    placeholder="Header-Name"
                    className="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                  <input
                    type="text"
                    value={header.value}
                    onChange={(e) => updateHeader(index, 'value', e.target.value)}
                    placeholder="value"
                    className="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
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

        {/* Response Body */}
        <div>
          <label className="mb-2 block text-sm font-medium">Response Body</label>

          {/* Body Type Selector */}
          <div className="mb-4 flex space-x-2">
            {[
              { id: 'static', label: 'Static', icon: Code },
              { id: 'template', label: 'Template', icon: Code },
              { id: 'faker', label: 'Faker', icon: Sparkles },
              { id: 'ai', label: 'AI', icon: Sparkles },
            ].map((type) => {
              const Icon = type.icon
              return (
                <button
                  key={type.id}
                  onClick={() => {
                    setBodyType(type.id as any)
                    if (type.id === 'static') {
                      updateResponse({
                        body: { type: 'Static', content: {} },
                      })
                    } else if (type.id === 'template') {
                      updateResponse({
                        body: { type: 'Template', template: '' },
                      })
                    } else if (type.id === 'faker') {
                      updateResponse({
                        body: { type: 'Faker', schema: {} },
                      })
                    } else if (type.id === 'ai') {
                      updateResponse({
                        body: { type: 'AI', prompt: '' },
                      })
                    }
                  }}
                  className={cn(
                    'inline-flex items-center space-x-2 rounded-lg px-3 py-2 text-sm font-medium',
                    bodyType === type.id
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

          {/* Body Editor */}
          {bodyType === 'static' && (
            <div>
              <div className={cn(
                'rounded-lg border',
                jsonErrors.staticBody ? 'border-destructive' : 'border-border'
              )}>
                <Editor
                  height="300px"
                  defaultLanguage="json"
                  value={JSON.stringify(config.response?.body?.content || {}, null, 2)}
                  onChange={(value) => {
                    try {
                      const content = JSON.parse(value || '{}')
                      updateResponse({ body: { type: 'Static', content } })
                      setJsonErrors((prev) => ({ ...prev, staticBody: null }))
                    } catch (e) {
                      const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                      setJsonErrors((prev) => ({ ...prev, staticBody: errorMessage }))
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
              {jsonErrors.staticBody && (
                <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                  <AlertCircle className="h-4 w-4" />
                  <span>Invalid JSON: {jsonErrors.staticBody}</span>
                </div>
              )}
            </div>
          )}

          {bodyType === 'template' && (
            <div>
              <textarea
                value={config.response?.body?.template || ''}
                onChange={(e) =>
                  updateResponse({ body: { type: 'Template', template: e.target.value } })
                }
                className="w-full rounded-lg border border-input bg-background p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                rows={10}
                placeholder="Enter template with {{uuid}}, {{now}}, {{faker.name}}, etc."
              />
              <p className="mt-2 text-xs text-muted-foreground">
                Available tokens: {'{{'}uuid{'}}'}, {'{{'}now{'}}'}, {'{{'}rand.int{'}}'}, {'{{'}faker.name{'}}'}, {'{{'}params.id{'}}'}, etc.
              </p>
            </div>
          )}

          {bodyType === 'faker' && (
            <div>
              <div className={cn(
                'rounded-lg border',
                jsonErrors.fakerSchema ? 'border-destructive' : 'border-border'
              )}>
                <Editor
                  height="300px"
                  defaultLanguage="json"
                  value={JSON.stringify(config.response?.body?.schema || {}, null, 2)}
                  onChange={(value) => {
                    try {
                      const schema = JSON.parse(value || '{}')
                      updateResponse({ body: { type: 'Faker', schema } })
                      setJsonErrors((prev) => ({ ...prev, fakerSchema: null }))
                    } catch (e) {
                      const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                      setJsonErrors((prev) => ({ ...prev, fakerSchema: errorMessage }))
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
              {jsonErrors.fakerSchema && (
                <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                  <AlertCircle className="h-4 w-4" />
                  <span>Invalid JSON: {jsonErrors.fakerSchema}</span>
                </div>
              )}
            </div>
          )}

          {bodyType === 'ai' && (
            <div>
              <textarea
                value={config.response?.body?.prompt || ''}
                onChange={(e) =>
                  updateResponse({ body: { type: 'AI', prompt: e.target.value } })
                }
                className="w-full rounded-lg border border-input bg-background p-3 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                rows={5}
                placeholder="Describe the response you want to generate..."
              />
            </div>
          )}
        </div>
      </div>

      {/* Behavior Configuration (Chaos Engineering) */}
      <div className="rounded-lg border border-border bg-card p-6">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Behavior & Chaos Engineering</h2>
          <button
            onClick={() => setShowBehavior(!showBehavior)}
            className="text-sm text-primary hover:underline"
          >
            {showBehavior ? 'Hide' : 'Show'}
          </button>
        </div>

        {showBehavior && (
          <div className="space-y-4">
            {/* Latency */}
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.behavior?.latency}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateBehavior({
                        latency: { base_ms: 100, jitter_ms: 50, distribution: 'fixed' },
                      })
                    } else {
                      const { latency, ...rest } = config.behavior || {}
                      onChange({ ...config, behavior: rest })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Add Latency</span>
              </label>
              {config.behavior?.latency && (
                <div className="mt-2 grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Base (ms)</label>
                    <input
                      type="number"
                      value={config.behavior.latency.base_ms}
                      onChange={(e) =>
                        updateBehavior({
                          latency: {
                            ...config.behavior.latency,
                            base_ms: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Jitter (ms)</label>
                    <input
                      type="number"
                      value={config.behavior.latency.jitter_ms}
                      onChange={(e) =>
                        updateBehavior({
                          latency: {
                            ...config.behavior.latency,
                            jitter_ms: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Failures */}
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.behavior?.failure}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateBehavior({
                        failure: { error_rate: 0.1, status_codes: [500] },
                      })
                    } else {
                      const { failure, ...rest } = config.behavior || {}
                      onChange({ ...config, behavior: rest })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Add Failures</span>
              </label>
              {config.behavior?.failure && (
                <div className="mt-2 space-y-2">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">
                      Error Rate (0.0 - 1.0)
                    </label>
                    <input
                      type="number"
                      step="0.01"
                      min="0"
                      max="1"
                      value={config.behavior.failure.error_rate}
                      onChange={(e) =>
                        updateBehavior({
                          failure: {
                            ...config.behavior.failure,
                            error_rate: parseFloat(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
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
