import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { Plus, Trash2, Edit, Power, PowerOff, Globe, Zap, MessageSquare, Server } from 'lucide-react'
import { toast } from 'sonner'
import { endpointsApi, EndpointConfig } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function Dashboard() {
  const queryClient = useQueryClient()

  const { data, isLoading } = useQuery({
    queryKey: ['endpoints'],
    queryFn: async () => {
      const response = await endpointsApi.list()
      return response.data
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) => endpointsApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['endpoints'] })
      toast.success('Endpoint deleted successfully')
    },
    onError: () => {
      toast.error('Failed to delete endpoint')
    },
  })

  const getProtocolIcon = (protocol: string) => {
    switch (protocol) {
      case 'http':
        return <Globe className="h-5 w-5" />
      case 'grpc':
        return <Zap className="h-5 w-5" />
      case 'websocket':
        return <MessageSquare className="h-5 w-5" />
      default:
        return <Globe className="h-5 w-5" />
    }
  }

  const getProtocolColor = (protocol: string) => {
    switch (protocol) {
      case 'http':
        return 'bg-blue-500/10 text-blue-500'
      case 'grpc':
        return 'bg-purple-500/10 text-purple-500'
      case 'websocket':
        return 'bg-green-500/10 text-green-500'
      default:
        return 'bg-gray-500/10 text-gray-500'
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"></div>
          <p className="mt-4 text-sm text-muted-foreground">Loading endpoints...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full p-8">
      {/* Header */}
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Endpoints</h1>
          <p className="mt-1 text-muted-foreground">
            Manage your mock endpoints and configurations
          </p>
        </div>
        <Link
          to="/endpoints/new"
          className="inline-flex items-center space-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          <Plus className="h-4 w-4" />
          <span>New Endpoint</span>
        </Link>
      </div>

      {/* Stats */}
      <div className="mb-8 grid grid-cols-1 gap-6 md:grid-cols-4">
        <div className="rounded-lg border border-border bg-card p-6">
          <div className="text-2xl font-bold">{data?.total || 0}</div>
          <div className="text-sm text-muted-foreground">Total Endpoints</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-6">
          <div className="text-2xl font-bold text-green-500">{data?.enabled || 0}</div>
          <div className="text-sm text-muted-foreground">Enabled</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-6">
          <div className="text-2xl font-bold text-blue-500">{data?.by_protocol?.http || 0}</div>
          <div className="text-sm text-muted-foreground">HTTP</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-6">
          <div className="text-2xl font-bold text-purple-500">{data?.by_protocol?.grpc || 0}</div>
          <div className="text-sm text-muted-foreground">gRPC</div>
        </div>
      </div>

      {/* Endpoints list */}
      {data && data.endpoints.length > 0 ? (
        <div className="space-y-4">
          {data.endpoints.map((endpoint: EndpointConfig) => (
            <div
              key={endpoint.id}
              className="rounded-lg border border-border bg-card p-6 transition-shadow hover:shadow-md"
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start space-x-4">
                  <div className={cn('rounded-lg p-3', getProtocolColor(endpoint.protocol))}>
                    {getProtocolIcon(endpoint.protocol)}
                  </div>
                  <div className="flex-1">
                    <div className="flex items-center space-x-3">
                      <h3 className="text-lg font-semibold">{endpoint.name}</h3>
                      <span className="rounded-full bg-secondary px-2.5 py-0.5 text-xs font-medium uppercase">
                        {endpoint.protocol}
                      </span>
                      {endpoint.enabled ? (
                        <Power className="h-4 w-4 text-green-500" />
                      ) : (
                        <PowerOff className="h-4 w-4 text-muted-foreground" />
                      )}
                    </div>
                    {endpoint.description && (
                      <p className="mt-1 text-sm text-muted-foreground">{endpoint.description}</p>
                    )}
                    <div className="mt-2">
                      {endpoint.config.type === 'Http' && (
                        <div className="flex items-center space-x-2 text-sm">
                          <span className="rounded bg-secondary px-2 py-1 font-mono font-semibold">
                            {endpoint.config.method}
                          </span>
                          <span className="font-mono text-muted-foreground">{endpoint.config.path}</span>
                        </div>
                      )}
                      {endpoint.config.type === 'Grpc' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          {endpoint.config.service}.{endpoint.config.method}
                        </div>
                      )}
                      {endpoint.config.type === 'Websocket' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          ws://{endpoint.config.path}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
                <div className="flex items-center space-x-2">
                  <Link
                    to={`/endpoints/${endpoint.id}`}
                    className="rounded-lg p-2 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  >
                    <Edit className="h-4 w-4" />
                  </Link>
                  <button
                    onClick={() => deleteMutation.mutate(endpoint.id)}
                    className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="rounded-lg border border-dashed border-border bg-card p-12 text-center">
          <div className="mx-auto max-w-md">
            <Server className="mx-auto h-12 w-12 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-semibold">No endpoints yet</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Get started by creating your first mock endpoint
            </p>
            <Link
              to="/endpoints/new"
              className="mt-6 inline-flex items-center space-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
            >
              <Plus className="h-4 w-4" />
              <span>Create Endpoint</span>
            </Link>
          </div>
        </div>
      )}
    </div>
  )
}
