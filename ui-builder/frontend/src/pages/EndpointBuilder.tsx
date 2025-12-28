import { useState, useEffect, useCallback, useRef } from 'react'
import { useParams, useNavigate, useBlocker } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Save, ArrowLeft, AlertTriangle } from 'lucide-react'
import FocusTrap from 'focus-trap-react'
import { endpointsApi, EndpointConfig, EndpointProtocolConfig } from '@/lib/api'
import { validateEndpoint, nameSchema, ValidationErrors } from '@/lib/validation'
import ProtocolSelector from '@/components/ProtocolSelector'
import HttpEndpointForm from '@/components/HttpEndpointForm'
import GrpcEndpointForm from '@/components/GrpcEndpointForm'
import WebsocketEndpointForm from '@/components/WebsocketEndpointForm'
import GraphqlEndpointForm from '@/components/GraphqlEndpointForm'
import SmtpEndpointForm from '@/components/SmtpEndpointForm'
import MqttEndpointForm from '@/components/MqttEndpointForm'
import AmqpEndpointForm from '@/components/AmqpEndpointForm'
import KafkaEndpointForm from '@/components/KafkaEndpointForm'

export default function EndpointBuilder() {
  const { id } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = Boolean(id)
  const [isFormValid, setIsFormValid] = useState(true)
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false)
  const [validationErrors, setValidationErrors] = useState<ValidationErrors>({})
  const [touchedFields, setTouchedFields] = useState<Set<string>>(new Set())
  const initialLoadComplete = useRef(false)

  const [endpoint, setEndpoint] = useState<Partial<EndpointConfig>>({
    id: '',
    protocol: 'http',
    name: '',
    description: '',
    enabled: true,
    config: {
      type: 'Http',
      method: 'GET',
      path: '/',
      response: {
        status: 200,
        body: { type: 'Static', content: {} },
      },
    },
  })

  // Wrapper to track changes - marks form as dirty on any change after initial load
  const handleChange = useCallback(<T extends Partial<EndpointConfig>>(updater: T | ((prev: Partial<EndpointConfig>) => Partial<EndpointConfig>)) => {
    setEndpoint(updater)
    if (initialLoadComplete.current) {
      setHasUnsavedChanges(true)
    }
  }, [])

  // Mark field as touched (for showing validation on blur)
  const handleFieldBlur = useCallback((fieldPath: string) => {
    setTouchedFields((prev) => new Set(prev).add(fieldPath))
  }, [])

  // Validate name field on blur
  const validateName = useCallback((name: string): string | undefined => {
    const result = nameSchema.safeParse(name)
    if (!result.success) {
      return result.error.issues[0]?.message
    }
    return undefined
  }, [])

  // Block navigation when there are unsaved changes
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      hasUnsavedChanges && currentLocation.pathname !== nextLocation.pathname
  )

  // Warn before closing browser tab
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges) {
        e.preventDefault()
        e.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', handleBeforeUnload)
    return () => window.removeEventListener('beforeunload', handleBeforeUnload)
  }, [hasUnsavedChanges])

  // Fetch endpoint if editing
  const { data: existingEndpoint } = useQuery({
    queryKey: ['endpoint', id],
    queryFn: async () => {
      if (!id) return null
      const response = await endpointsApi.get(id)
      return response.data
    },
    enabled: isEditing,
  })

  useEffect(() => {
    if (existingEndpoint) {
      setEndpoint(existingEndpoint)
      // Mark initial load complete after setting existing data
      setTimeout(() => {
        initialLoadComplete.current = true
      }, 100)
    } else if (!isEditing) {
      // For new endpoints, mark as ready immediately
      initialLoadComplete.current = true
    }
  }, [existingEndpoint, isEditing])

  const saveMutation = useMutation({
    mutationFn: async (data: EndpointConfig) => {
      if (isEditing && id) {
        return endpointsApi.update(id, data)
      } else {
        return endpointsApi.create(data as any)
      }
    },
    onSuccess: () => {
      setHasUnsavedChanges(false)
      queryClient.invalidateQueries({ queryKey: ['endpoints'] })
      toast.success(isEditing ? 'Endpoint updated' : 'Endpoint created')
      navigate('/')
    },
    onError: (error: any) => {
      toast.error(error.response?.data?.message || 'Failed to save endpoint')
    },
  })

  const handleSave = async () => {
    // Run client-side Zod validation first
    const validationResult = validateEndpoint(endpoint)

    if (!validationResult.success) {
      setValidationErrors(validationResult.errors)
      // Mark all fields as touched to show all errors
      setTouchedFields(new Set(Object.keys(validationResult.errors)))

      // Show first error as toast
      const firstError = Object.values(validationResult.errors)[0]
      toast.error(firstError || 'Please fix the validation errors')
      return
    }

    // Clear client-side errors
    setValidationErrors({})

    // Check JSON editor validity
    if (!isFormValid) {
      toast.error('Please fix the JSON errors before saving')
      return
    }

    try {
      // Server-side validation
      const response = await endpointsApi.validate(endpoint as EndpointConfig)
      if (!response.data.valid) {
        toast.error('Validation failed: ' + response.data.errors[0]?.message)
        return
      }

      if (response.data.warnings.length > 0) {
        response.data.warnings.forEach((warning) => toast.warning(warning))
      }

      saveMutation.mutate(endpoint as EndpointConfig)
    } catch (error) {
      toast.error('Failed to validate endpoint')
    }
  }

  return (
    <div className="h-full overflow-auto p-4 sm:p-6 md:p-8">
      {/* Navigation Blocker Dialog */}
      {blocker.state === 'blocked' && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="unsaved-changes-title"
          aria-describedby="unsaved-changes-desc"
        >
          <FocusTrap
            focusTrapOptions={{
              allowOutsideClick: true,
              escapeDeactivates: false,
            }}
          >
            <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-lg">
              <div className="flex items-start space-x-4">
                <div className="rounded-full bg-yellow-500/10 p-2">
                  <AlertTriangle className="h-6 w-6 text-yellow-500" aria-hidden="true" />
                </div>
                <div className="flex-1">
                  <h2 id="unsaved-changes-title" className="text-lg font-semibold">
                    Unsaved Changes
                  </h2>
                  <p id="unsaved-changes-desc" className="mt-2 text-sm text-muted-foreground">
                    You have unsaved changes. Are you sure you want to leave this page? Your changes will be lost.
                  </p>
                </div>
              </div>
              <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:gap-3">
                <button
                  onClick={() => blocker.reset?.()}
                  className="w-full rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring sm:w-auto"
                >
                  Stay on Page
                </button>
                <button
                  onClick={() => blocker.proceed?.()}
                  className="w-full rounded-lg bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 focus:outline-none focus:ring-2 focus:ring-ring sm:w-auto"
                >
                  Leave Page
                </button>
              </div>
            </div>
          </FocusTrap>
        </div>
      )}

      {/* Header */}
      <div className="mb-8">
        <button
          onClick={() => navigate('/')}
          className="mb-4 inline-flex items-center space-x-2 text-sm text-muted-foreground hover:text-foreground focus:outline-none focus:ring-2 focus:ring-ring rounded-lg px-2 py-1 -mx-2"
        >
          <ArrowLeft className="h-4 w-4" aria-hidden="true" />
          <span>Back to Dashboard</span>
        </button>
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">
              {isEditing ? 'Edit Endpoint' : 'Create Endpoint'}
            </h1>
            <p className="mt-1 text-muted-foreground">
              Configure your mock endpoint
            </p>
          </div>
          <button
            onClick={handleSave}
            disabled={saveMutation.isPending}
            className="inline-flex items-center space-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Save className="h-4 w-4" />
            <span>{saveMutation.isPending ? 'Saving...' : 'Save'}</span>
          </button>
        </div>
      </div>

      {/* Basic Info */}
      <div className="mb-8 rounded-lg border border-border bg-card p-6">
        <h2 className="mb-4 text-lg font-semibold">Basic Information</h2>
        <div className="space-y-4">
          <div>
            <label htmlFor="endpoint-name" className="mb-2 block text-sm font-medium">
              Name <span className="text-destructive">*</span>
            </label>
            <input
              id="endpoint-name"
              type="text"
              value={endpoint.name}
              onChange={(e) => {
                handleChange({ ...endpoint, name: e.target.value })
                // Clear error when user starts typing
                if (validationErrors.name) {
                  setValidationErrors((prev) => {
                    const { name, ...rest } = prev
                    return rest
                  })
                }
              }}
              onBlur={() => {
                handleFieldBlur('name')
                const error = validateName(endpoint.name || '')
                if (error) {
                  setValidationErrors((prev) => ({ ...prev, name: error }))
                }
              }}
              className={`w-full rounded-lg border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring ${
                touchedFields.has('name') && validationErrors.name
                  ? 'border-destructive bg-destructive/5'
                  : 'border-input bg-background'
              }`}
              placeholder="My Endpoint"
              aria-invalid={touchedFields.has('name') && !!validationErrors.name}
              aria-describedby={validationErrors.name ? 'name-error' : undefined}
            />
            {touchedFields.has('name') && validationErrors.name && (
              <p id="name-error" className="mt-1 text-sm text-destructive">
                {validationErrors.name}
              </p>
            )}
          </div>
          <div>
            <label htmlFor="endpoint-description" className="mb-2 block text-sm font-medium">
              Description (optional)
            </label>
            <input
              id="endpoint-description"
              type="text"
              value={endpoint.description || ''}
              onChange={(e) => handleChange({ ...endpoint, description: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="A brief description of this endpoint"
              maxLength={500}
            />
            <p className="mt-1 text-xs text-muted-foreground">
              {(endpoint.description || '').length}/500 characters
            </p>
          </div>
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="enabled"
              checked={endpoint.enabled}
              onChange={(e) => handleChange({ ...endpoint, enabled: e.target.checked })}
              className="h-4 w-4 rounded border-input"
            />
            <label htmlFor="enabled" className="text-sm font-medium">
              Enabled
            </label>
          </div>
        </div>
      </div>

      {/* Protocol Selection */}
      {!isEditing && (
        <div className="mb-8">
          <ProtocolSelector
            selected={endpoint.protocol!}
            onSelect={(protocol: any) => {
              handleChange({ ...endpoint, protocol })
              // Reset config based on protocol
              if (protocol === 'http') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Http',
                    method: 'GET',
                    path: '/',
                    response: {
                      status: 200,
                      body: { type: 'Static', content: {} },
                    },
                  },
                }))
              } else if (protocol === 'grpc') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Grpc',
                    service: '',
                    method: '',
                    proto_file: '',
                    request_type: '',
                    response_type: '',
                    response: {
                      body: { type: 'Static', content: {} },
                    },
                  },
                }))
              } else if (protocol === 'websocket') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Websocket',
                    path: '/',
                  },
                }))
              } else if (protocol === 'graphql') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Graphql',
                    path: '/graphql',
                    schema: `type Query {
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
}`,
                    introspection: true,
                    resolvers: [],
                  },
                }))
              } else if (protocol === 'smtp') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Smtp',
                    port: 25,
                    hostname: 'localhost',
                    tlsMode: 'none',
                    authentication: {
                      enabled: false,
                      mechanisms: ['PLAIN'],
                      credentials: [],
                    },
                    messageHandling: {
                      acceptAll: true,
                      senderFilters: [],
                      recipientFilters: [],
                    },
                    behavior: {
                      defaultResponse: 'accept',
                    },
                    messageStorage: {
                      enabled: true,
                      maxMessages: 100,
                      apiPath: '/api/smtp/messages',
                    },
                  },
                }))
              } else if (protocol === 'mqtt') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Mqtt',
                    topicPattern: 'sensors/+/data',
                    qos: 0,
                    retained: false,
                    payload: { type: 'Static', content: {} },
                  },
                }))
              } else if (protocol === 'amqp') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Amqp',
                    exchange: 'my-exchange',
                    exchangeType: 'direct',
                    routingKey: 'my-key',
                    durable: true,
                    autoDelete: false,
                    payload: { type: 'Static', content: {} },
                  },
                }))
              } else if (protocol === 'kafka') {
                handleChange((prev) => ({
                  ...prev,
                  config: {
                    type: 'Kafka',
                    topic: 'my-topic',
                    payload: { type: 'Static', content: {} },
                  },
                }))
              }
            }}
          />
        </div>
      )}

      {/* Protocol-specific forms */}
      {endpoint.protocol === 'http' && endpoint.config?.type === 'Http' && (
        <HttpEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'grpc' && endpoint.config?.type === 'Grpc' && (
        <GrpcEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'websocket' && endpoint.config?.type === 'Websocket' && (
        <WebsocketEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'graphql' && endpoint.config?.type === 'Graphql' && (
        <GraphqlEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'smtp' && endpoint.config?.type === 'Smtp' && (
        <SmtpEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'mqtt' && endpoint.config?.type === 'Mqtt' && (
        <MqttEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'amqp' && endpoint.config?.type === 'Amqp' && (
        <AmqpEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
      {endpoint.protocol === 'kafka' && endpoint.config?.type === 'Kafka' && (
        <KafkaEndpointForm
          config={endpoint.config}
          onChange={(config) => handleChange({ ...endpoint, config: config as EndpointProtocolConfig })}
          onValidationChange={setIsFormValid}
        />
      )}
    </div>
  )
}
