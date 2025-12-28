import { useState, useEffect } from 'react'
import { Plus, X, Code, Sparkles, AlertCircle } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { cn } from '@/lib/utils'
import type { GraphqlFormProps } from '@/types/protocol-configs'

interface JsonEditorErrors {
  staticResponse: string | null
  fakerSchema: string | null
  customErrors: string | null
}

interface MockResolver {
  operationType: 'query' | 'mutation' | 'subscription'
  operationName: string
  responseType: 'static' | 'template' | 'faker'
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  response: any
}

export default function GraphqlEndpointForm({ config, onChange, onValidationChange }: GraphqlFormProps) {
  const [jsonErrors, setJsonErrors] = useState<JsonEditorErrors>({
    staticResponse: null,
    fakerSchema: null,
    customErrors: null,
  })
  const [activeResolverIndex, setActiveResolverIndex] = useState<number | null>(null)
  const [showBehavior, setShowBehavior] = useState(false)

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

  const updateBehavior = (updates: any) => {
    onChange({
      ...config,
      behavior: {
        ...config.behavior,
        ...updates,
      },
    })
  }

  const addResolver = () => {
    const resolvers = config.resolvers || []
    const newResolver: MockResolver = {
      operationType: 'query',
      operationName: '',
      responseType: 'static',
      response: { type: 'Static', content: {} },
    }
    updateConfig({
      resolvers: [...resolvers, newResolver],
    })
    setActiveResolverIndex(resolvers.length)
  }

  const updateResolver = (index: number, updates: Partial<MockResolver>) => {
    const resolvers = [...(config.resolvers || [])]
    resolvers[index] = { ...resolvers[index], ...updates }
    updateConfig({ resolvers })
  }

  const removeResolver = (index: number) => {
    const resolvers = [...(config.resolvers || [])]
    resolvers.splice(index, 1)
    updateConfig({ resolvers })
    if (activeResolverIndex === index) {
      setActiveResolverIndex(null)
    } else if (activeResolverIndex !== null && activeResolverIndex > index) {
      setActiveResolverIndex(activeResolverIndex - 1)
    }
  }

  const getResolverResponseContent = (resolver: MockResolver) => {
    if (resolver.responseType === 'static') {
      return resolver.response?.content || {}
    } else if (resolver.responseType === 'faker') {
      return resolver.response?.schema || {}
    }
    return {}
  }

  return (
    <div className="space-y-6">
      {/* GraphQL Configuration */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">GraphQL Endpoint</h2>
        <div className="space-y-4">
          <div>
            <label className="mb-2 block text-sm font-medium">Path</label>
            <input
              type="text"
              value={config.path || '/graphql'}
              onChange={(e) => updateConfig({ path: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="/graphql"
            />
          </div>

          {/* Introspection Toggle */}
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="introspection"
              checked={config.introspection !== false}
              onChange={(e) => updateConfig({ introspection: e.target.checked })}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="introspection" className="text-sm font-medium">
              Enable Introspection
            </label>
            <span className="text-xs text-muted-foreground">
              (Allow schema introspection queries)
            </span>
          </div>
        </div>
      </div>

      {/* Schema Definition */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Schema Definition (SDL)</h2>
        <div>
          <div className="rounded-lg border border-border">
            <Editor
              height="300px"
              defaultLanguage="graphql"
              value={config.schema || `type Query {
  hello: String
  user(id: ID!): User
  users: [User!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User
}

type User {
  id: ID!
  name: String!
  email: String!
}

input CreateUserInput {
  name: String!
  email: String!
}`}
              onChange={(value) => updateConfig({ schema: value })}
              theme="vs-dark"
              options={{
                minimap: { enabled: false },
                fontSize: 13,
                wordWrap: 'on',
              }}
            />
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            Define your GraphQL schema using SDL (Schema Definition Language). Include types, queries, mutations, and subscriptions.
          </p>
        </div>
      </div>

      {/* Mock Resolvers */}
      <div className="rounded-lg border border-border bg-card p-6">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Mock Resolvers</h2>
          <button
            onClick={addResolver}
            className="inline-flex items-center space-x-1 text-sm text-primary hover:underline"
          >
            <Plus className="h-4 w-4" />
            <span>Add Resolver</span>
          </button>
        </div>

        {(!config.resolvers || config.resolvers.length === 0) && (
          <p className="text-sm text-muted-foreground">
            No resolvers configured. Add resolvers to define how queries and mutations should respond.
          </p>
        )}

        {config.resolvers?.length > 0 && (
          <div className="space-y-4">
            {/* Resolver List */}
            <div className="space-y-2">
              {config.resolvers.map((resolver: MockResolver, index: number) => (
                <div
                  key={index}
                  className={cn(
                    'flex items-center justify-between rounded-lg border p-3 cursor-pointer transition-colors',
                    activeResolverIndex === index
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:border-primary/50'
                  )}
                  onClick={() => setActiveResolverIndex(activeResolverIndex === index ? null : index)}
                >
                  <div className="flex items-center space-x-3">
                    <span className={cn(
                      'rounded px-2 py-0.5 text-xs font-medium uppercase',
                      resolver.operationType === 'query' && 'bg-blue-500/10 text-blue-500',
                      resolver.operationType === 'mutation' && 'bg-green-500/10 text-green-500',
                      resolver.operationType === 'subscription' && 'bg-purple-500/10 text-purple-500'
                    )}>
                      {resolver.operationType}
                    </span>
                    <span className="font-mono text-sm">
                      {resolver.operationName || '(unnamed)'}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {resolver.responseType}
                    </span>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      removeResolver(index)
                    }}
                    className="rounded-lg p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>
              ))}
            </div>

            {/* Active Resolver Editor */}
            {activeResolverIndex !== null && config.resolvers[activeResolverIndex] && (
              <div className="mt-4 space-y-4 rounded-lg border border-border bg-background p-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-2 block text-sm font-medium">Operation Type</label>
                    <select
                      value={config.resolvers[activeResolverIndex].operationType}
                      onChange={(e) => updateResolver(activeResolverIndex, { operationType: e.target.value as any })}
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    >
                      <option value="query">Query</option>
                      <option value="mutation">Mutation</option>
                      <option value="subscription">Subscription</option>
                    </select>
                  </div>
                  <div>
                    <label className="mb-2 block text-sm font-medium">Operation Name</label>
                    <input
                      type="text"
                      value={config.resolvers[activeResolverIndex].operationName}
                      onChange={(e) => updateResolver(activeResolverIndex, { operationName: e.target.value })}
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      placeholder="getUser"
                    />
                  </div>
                </div>

                {/* Response Type Selector */}
                <div>
                  <label className="mb-2 block text-sm font-medium">Response Type</label>
                  <div className="flex space-x-2">
                    {[
                      { id: 'static', label: 'Static', icon: Code },
                      { id: 'template', label: 'Template', icon: Code },
                      { id: 'faker', label: 'Faker', icon: Sparkles },
                    ].map((type) => {
                      const Icon = type.icon
                      const isSelected = config.resolvers[activeResolverIndex].responseType === type.id
                      return (
                        <button
                          key={type.id}
                          onClick={() => {
                            const updates: Partial<MockResolver> = { responseType: type.id as any }
                            if (type.id === 'static') {
                              updates.response = { type: 'Static', content: {} }
                            } else if (type.id === 'template') {
                              updates.response = { type: 'Template', template: '' }
                            } else if (type.id === 'faker') {
                              updates.response = { type: 'Faker', schema: {} }
                            }
                            updateResolver(activeResolverIndex, updates)
                          }}
                          className={cn(
                            'inline-flex items-center space-x-2 rounded-lg px-3 py-2 text-sm font-medium',
                            isSelected
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
                </div>

                {/* Response Editor */}
                {config.resolvers[activeResolverIndex].responseType === 'static' && (
                  <div>
                    <label className="mb-2 block text-sm font-medium">Static Response (JSON)</label>
                    <div className={cn(
                      'rounded-lg border',
                      jsonErrors.staticResponse ? 'border-destructive' : 'border-border'
                    )}>
                      <Editor
                        height="200px"
                        defaultLanguage="json"
                        value={JSON.stringify(getResolverResponseContent(config.resolvers[activeResolverIndex]), null, 2)}
                        onChange={(value) => {
                          try {
                            const content = JSON.parse(value || '{}')
                            updateResolver(activeResolverIndex, {
                              response: { type: 'Static', content }
                            })
                            setJsonErrors((prev) => ({ ...prev, staticResponse: null }))
                          } catch (e) {
                            const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                            setJsonErrors((prev) => ({ ...prev, staticResponse: errorMessage }))
                          }
                        }}
                        theme="vs-dark"
                        options={{
                          minimap: { enabled: false },
                          fontSize: 13,
                        }}
                      />
                    </div>
                    {jsonErrors.staticResponse && (
                      <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                        <AlertCircle className="h-4 w-4" />
                        <span>Invalid JSON: {jsonErrors.staticResponse}</span>
                      </div>
                    )}
                  </div>
                )}

                {config.resolvers[activeResolverIndex].responseType === 'template' && (
                  <div>
                    <label className="mb-2 block text-sm font-medium">Template Response</label>
                    <textarea
                      value={config.resolvers[activeResolverIndex].response?.template || ''}
                      onChange={(e) =>
                        updateResolver(activeResolverIndex, {
                          response: { type: 'Template', template: e.target.value }
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                      rows={8}
                      placeholder={`{
  "data": {
    "user": {
      "id": "{{uuid}}",
      "name": "{{faker.name}}",
      "email": "{{faker.email}}"
    }
  }
}`}
                    />
                    <p className="mt-2 text-xs text-muted-foreground">
                      Available tokens: {'{{'}uuid{'}}'}, {'{{'}now{'}}'}, {'{{'}rand.int{'}}'}, {'{{'}faker.name{'}}'}, {'{{'}args.id{'}}'}, etc.
                    </p>
                  </div>
                )}

                {config.resolvers[activeResolverIndex].responseType === 'faker' && (
                  <div>
                    <label className="mb-2 block text-sm font-medium">Faker Schema (JSON)</label>
                    <div className={cn(
                      'rounded-lg border',
                      jsonErrors.fakerSchema ? 'border-destructive' : 'border-border'
                    )}>
                      <Editor
                        height="200px"
                        defaultLanguage="json"
                        value={JSON.stringify(config.resolvers[activeResolverIndex].response?.schema || {}, null, 2)}
                        onChange={(value) => {
                          try {
                            const schema = JSON.parse(value || '{}')
                            updateResolver(activeResolverIndex, {
                              response: { type: 'Faker', schema }
                            })
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
                      Define a schema using Faker.js types like name, email, uuid, etc.
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Error Simulation */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Error Simulation</h2>
        <div className="space-y-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="error-simulation"
              checked={!!config.errorSimulation?.enabled}
              onChange={(e) => {
                if (e.target.checked) {
                  updateConfig({
                    errorSimulation: {
                      enabled: true,
                      errorRate: 10,
                      customErrors: [],
                    },
                  })
                } else {
                  updateConfig({ errorSimulation: { enabled: false } })
                }
              }}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="error-simulation" className="text-sm font-medium">
              Enable Error Simulation
            </label>
          </div>

          {config.errorSimulation?.enabled && (
            <div className="space-y-4 pl-6">
              <div>
                <label className="mb-2 block text-sm font-medium">
                  Error Rate (%)
                </label>
                <div className="flex items-center space-x-4">
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={config.errorSimulation.errorRate || 0}
                    onChange={(e) =>
                      updateConfig({
                        errorSimulation: {
                          ...config.errorSimulation,
                          errorRate: parseInt(e.target.value),
                        },
                      })
                    }
                    className="flex-1"
                  />
                  <span className="w-12 text-right text-sm font-medium">
                    {config.errorSimulation.errorRate || 0}%
                  </span>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  Percentage of requests that will return an error response
                </p>
              </div>

              <div>
                <label className="mb-2 block text-sm font-medium">Custom Error Responses (JSON Array)</label>
                <div className={cn(
                  'rounded-lg border',
                  jsonErrors.customErrors ? 'border-destructive' : 'border-border'
                )}>
                  <Editor
                    height="150px"
                    defaultLanguage="json"
                    value={JSON.stringify(config.errorSimulation.customErrors || [
                      {
                        message: "Internal server error",
                        extensions: { code: "INTERNAL_ERROR" }
                      }
                    ], null, 2)}
                    onChange={(value) => {
                      try {
                        const customErrors = JSON.parse(value || '[]')
                        updateConfig({
                          errorSimulation: {
                            ...config.errorSimulation,
                            customErrors,
                          },
                        })
                        setJsonErrors((prev) => ({ ...prev, customErrors: null }))
                      } catch (e) {
                        const errorMessage = e instanceof Error ? e.message : 'Invalid JSON'
                        setJsonErrors((prev) => ({ ...prev, customErrors: errorMessage }))
                      }
                    }}
                    theme="vs-dark"
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                    }}
                  />
                </div>
                {jsonErrors.customErrors && (
                  <div className="mt-2 flex items-center space-x-2 text-sm text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    <span>Invalid JSON: {jsonErrors.customErrors}</span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Latency Simulation */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Latency Simulation</h2>
        <div className="space-y-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="latency-simulation"
              checked={!!config.latencySimulation?.enabled}
              onChange={(e) => {
                if (e.target.checked) {
                  updateConfig({
                    latencySimulation: {
                      enabled: true,
                      minMs: 100,
                      maxMs: 500,
                    },
                  })
                } else {
                  updateConfig({ latencySimulation: { enabled: false } })
                }
              }}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="latency-simulation" className="text-sm font-medium">
              Enable Latency Simulation
            </label>
          </div>

          {config.latencySimulation?.enabled && (
            <div className="grid gap-4 pl-6 md:grid-cols-2">
              <div>
                <label className="mb-2 block text-sm font-medium">Min Latency (ms)</label>
                <input
                  type="number"
                  min="0"
                  value={config.latencySimulation.minMs || 0}
                  onChange={(e) =>
                    updateConfig({
                      latencySimulation: {
                        ...config.latencySimulation,
                        minMs: parseInt(e.target.value),
                      },
                    })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>
              <div>
                <label className="mb-2 block text-sm font-medium">Max Latency (ms)</label>
                <input
                  type="number"
                  min="0"
                  value={config.latencySimulation.maxMs || 0}
                  onChange={(e) =>
                    updateConfig({
                      latencySimulation: {
                        ...config.latencySimulation,
                        maxMs: parseInt(e.target.value),
                      },
                    })
                  }
                  className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>
              <p className="col-span-2 text-xs text-muted-foreground">
                Response latency will be randomized between min and max values
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Behavior Configuration (Chaos Engineering) */}
      <div className="rounded-lg border border-border bg-card p-6">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Advanced Behavior</h2>
          <button
            onClick={() => setShowBehavior(!showBehavior)}
            className="text-sm text-primary hover:underline"
          >
            {showBehavior ? 'Hide' : 'Show'}
          </button>
        </div>

        {showBehavior && (
          <div className="space-y-4">
            {/* Rate Limiting */}
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.behavior?.rateLimit}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateBehavior({
                        rateLimit: { requestsPerSecond: 100, burstSize: 10 },
                      })
                    } else {
                      const { rateLimit, ...rest } = config.behavior || {}
                      onChange({ ...config, behavior: rest })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Rate Limiting</span>
              </label>
              {config.behavior?.rateLimit && (
                <div className="mt-2 grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Requests per Second</label>
                    <input
                      type="number"
                      value={config.behavior.rateLimit.requestsPerSecond}
                      onChange={(e) =>
                        updateBehavior({
                          rateLimit: {
                            ...config.behavior.rateLimit,
                            requestsPerSecond: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Burst Size</label>
                    <input
                      type="number"
                      value={config.behavior.rateLimit.burstSize}
                      onChange={(e) =>
                        updateBehavior({
                          rateLimit: {
                            ...config.behavior.rateLimit,
                            burstSize: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Query Complexity Limit */}
            <div>
              <label className="mb-2 flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={!!config.behavior?.complexityLimit}
                  onChange={(e) => {
                    if (e.target.checked) {
                      updateBehavior({
                        complexityLimit: { maxDepth: 10, maxComplexity: 100 },
                      })
                    } else {
                      const { complexityLimit, ...rest } = config.behavior || {}
                      onChange({ ...config, behavior: rest })
                    }
                  }}
                  className="h-4 w-4 rounded border-input"
                />
                <span className="text-sm font-medium">Query Complexity Limit</span>
              </label>
              {config.behavior?.complexityLimit && (
                <div className="mt-2 grid gap-4 md:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Max Query Depth</label>
                    <input
                      type="number"
                      value={config.behavior.complexityLimit.maxDepth}
                      onChange={(e) =>
                        updateBehavior({
                          complexityLimit: {
                            ...config.behavior.complexityLimit,
                            maxDepth: parseInt(e.target.value),
                          },
                        })
                      }
                      className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-muted-foreground">Max Complexity Score</label>
                    <input
                      type="number"
                      value={config.behavior.complexityLimit.maxComplexity}
                      onChange={(e) =>
                        updateBehavior({
                          complexityLimit: {
                            ...config.behavior.complexityLimit,
                            maxComplexity: parseInt(e.target.value),
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
