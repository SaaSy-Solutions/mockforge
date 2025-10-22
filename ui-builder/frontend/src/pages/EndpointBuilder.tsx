import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Save, ArrowLeft } from 'lucide-react'
import { endpointsApi, EndpointConfig } from '@/lib/api'
import ProtocolSelector from '@/components/ProtocolSelector'
import HttpEndpointForm from '@/components/HttpEndpointForm'
import GrpcEndpointForm from '@/components/GrpcEndpointForm'
import WebsocketEndpointForm from '@/components/WebsocketEndpointForm'

export default function EndpointBuilder() {
  const { id } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = Boolean(id)

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
    }
  }, [existingEndpoint])

  const saveMutation = useMutation({
    mutationFn: async (data: EndpointConfig) => {
      if (isEditing && id) {
        return endpointsApi.update(id, data)
      } else {
        return endpointsApi.create(data as any)
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['endpoints'] })
      toast.success(isEditing ? 'Endpoint updated' : 'Endpoint created')
      navigate('/')
    },
    onError: (error: any) => {
      toast.error(error.response?.data?.message || 'Failed to save endpoint')
    },
  })

  const handleSave = async () => {
    // Validate
    if (!endpoint.name) {
      toast.error('Please enter an endpoint name')
      return
    }

    try {
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
    <div className="h-full overflow-auto p-8">
      {/* Header */}
      <div className="mb-8">
        <button
          onClick={() => navigate('/')}
          className="mb-4 inline-flex items-center space-x-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-4 w-4" />
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
            <label className="mb-2 block text-sm font-medium">Name</label>
            <input
              type="text"
              value={endpoint.name}
              onChange={(e) => setEndpoint({ ...endpoint, name: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="My Endpoint"
            />
          </div>
          <div>
            <label className="mb-2 block text-sm font-medium">Description (optional)</label>
            <input
              type="text"
              value={endpoint.description || ''}
              onChange={(e) => setEndpoint({ ...endpoint, description: e.target.value })}
              className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="A brief description of this endpoint"
            />
          </div>
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="enabled"
              checked={endpoint.enabled}
              onChange={(e) => setEndpoint({ ...endpoint, enabled: e.target.checked })}
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
              setEndpoint({ ...endpoint, protocol })
              // Reset config based on protocol
              if (protocol === 'http') {
                setEndpoint((prev) => ({
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
                setEndpoint((prev) => ({
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
                setEndpoint((prev) => ({
                  ...prev,
                  config: {
                    type: 'Websocket',
                    path: '/',
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
          onChange={(config) => setEndpoint({ ...endpoint, config })}
        />
      )}
      {endpoint.protocol === 'grpc' && endpoint.config?.type === 'Grpc' && (
        <GrpcEndpointForm
          config={endpoint.config}
          onChange={(config) => setEndpoint({ ...endpoint, config })}
        />
      )}
      {endpoint.protocol === 'websocket' && endpoint.config?.type === 'Websocket' && (
        <WebsocketEndpointForm
          config={endpoint.config}
          onChange={(config) => setEndpoint({ ...endpoint, config })}
        />
      )}
    </div>
  )
}
