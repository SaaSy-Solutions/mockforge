import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { Plus, Trash2, Edit, Power, PowerOff, Globe, Zap, MessageSquare, Server, FileJson, Upload, Download } from 'lucide-react'
import { toast } from 'sonner'
import { endpointsApi, EndpointConfig, openApiApi } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useState, useRef } from 'react'

export default function Dashboard() {
  const queryClient = useQueryClient()
  const [showImportDialog, setShowImportDialog] = useState(false)
  const [showExportDialog, setShowExportDialog] = useState(false)
  const [validationErrors, setValidationErrors] = useState<string[]>([])
  const [specPreview, setSpecPreview] = useState<{title?: string, version?: string, endpoints?: number} | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

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

  const importOpenApiMutation = useMutation({
    mutationFn: async (file: File) => {
      const content = await file.text()
      const response = await openApiApi.import(content, undefined, true)
      return response.data
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['endpoints'] })
      toast.success(`Successfully imported ${data.endpoints_created} endpoints from ${data.spec_info.title}`)
      if (data.warnings.length > 0) {
        data.warnings.forEach((warning) => toast.warning(warning))
      }
      setShowImportDialog(false)
    },
    onError: (error: any) => {
      toast.error(error?.response?.data?.details || 'Failed to import OpenAPI specification')
    },
  })

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (file) {
      // Reset previous state
      setValidationErrors([])
      setSpecPreview(null)

      // Validate file type
      const validExtensions = ['.json', '.yaml', '.yml']
      const fileExtension = file.name.substring(file.name.lastIndexOf('.')).toLowerCase()

      if (!validExtensions.includes(fileExtension)) {
        setValidationErrors([`Invalid file type: ${file.name}. Please upload a JSON or YAML file.`])
        return
      }

      // Read and preview file content
      try {
        const content = await file.text()
        let spec: any

        // Parse file
        if (fileExtension === '.json') {
          spec = JSON.parse(content)
        } else {
          // For YAML, we'll just show basic validation
          spec = { info: { title: 'YAML Spec', version: '1.0.0' } }
        }

        // Basic validation
        const errors: string[] = []

        if (!spec.openapi && !spec.asyncapi && !spec.swagger) {
          errors.push('Missing API specification version (openapi, asyncapi, or swagger field)')
        }

        if (!spec.info?.title) {
          errors.push('Missing spec title (info.title)')
        }

        if (!spec.info?.version) {
          errors.push('Missing spec version (info.version)')
        }

        if (!spec.paths && !spec.channels) {
          errors.push('No endpoints found (missing paths or channels)')
        }

        // Count endpoints
        const endpointCount = Object.keys(spec.paths || spec.channels || {}).length

        if (errors.length > 0) {
          setValidationErrors(errors)
        } else {
          setSpecPreview({
            title: spec.info?.title,
            version: spec.info?.version,
            endpoints: endpointCount,
          })

          // If valid, proceed with import
          importOpenApiMutation.mutate(file)
        }
      } catch (error) {
        setValidationErrors([`Failed to parse file: ${error instanceof Error ? error.message : 'Unknown error'}`])
      }
    }
  }

  const handleExportOpenApi = async () => {
    try {
      const response = await openApiApi.export()
      const blob = new Blob([JSON.stringify(response.data, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'mockforge-openapi.json'
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.success('OpenAPI specification exported successfully')
      setShowExportDialog(false)
    } catch (error) {
      toast.error('Failed to export OpenAPI specification')
    }
  }

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
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowImportDialog(true)}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-card px-4 py-2 text-sm font-medium hover:bg-accent"
          >
            <Upload className="h-4 w-4" />
            <span>Import OpenAPI</span>
          </button>
          <button
            onClick={() => setShowExportDialog(true)}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-card px-4 py-2 text-sm font-medium hover:bg-accent"
            disabled={!data || data.endpoints.length === 0}
          >
            <Download className="h-4 w-4" />
            <span>Export OpenAPI</span>
          </button>
          <Link
            to="/endpoints/new"
            className="inline-flex items-center space-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            <Plus className="h-4 w-4" />
            <span>New Endpoint</span>
          </Link>
        </div>
      </div>

      {/* Import Dialog */}
      {showImportDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-lg">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-xl font-semibold">Import OpenAPI Specification</h2>
              <button
                onClick={() => {
                  setShowImportDialog(false)
                  setValidationErrors([])
                  setSpecPreview(null)
                }}
                className="text-muted-foreground hover:text-foreground"
              >
                ✕
              </button>
            </div>
            <p className="mb-6 text-sm text-muted-foreground">
              Upload an OpenAPI (Swagger) or AsyncAPI specification file to automatically generate mock endpoints.
              Supports JSON and YAML formats.
            </p>

            {/* Validation Errors */}
            {validationErrors.length > 0 && (
              <div className="mb-4 rounded-lg border border-red-500/20 bg-red-500/10 p-4">
                <p className="mb-2 font-medium text-red-500">Validation Errors:</p>
                <ul className="list-disc space-y-1 pl-4 text-sm text-red-400">
                  {validationErrors.map((error, index) => (
                    <li key={index}>{error}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Spec Preview */}
            {specPreview && (
              <div className="mb-4 rounded-lg border border-green-500/20 bg-green-500/10 p-4">
                <p className="mb-2 font-medium text-green-500">Validation Successful</p>
                <div className="space-y-1 text-sm text-green-400">
                  <p><strong>Title:</strong> {specPreview.title}</p>
                  <p><strong>Version:</strong> {specPreview.version}</p>
                  <p><strong>Endpoints:</strong> {specPreview.endpoints}</p>
                </div>
              </div>
            )}

            <input
              ref={fileInputRef}
              type="file"
              accept=".json,.yaml,.yml"
              onChange={handleFileSelect}
              className="hidden"
            />
            <button
              onClick={() => fileInputRef.current?.click()}
              disabled={importOpenApiMutation.isPending}
              className="w-full rounded-lg border-2 border-dashed border-border bg-accent/50 px-4 py-8 text-center hover:bg-accent disabled:opacity-50"
            >
              <FileJson className="mx-auto mb-2 h-8 w-8 text-muted-foreground" />
              <p className="text-sm font-medium">
                {importOpenApiMutation.isPending ? 'Importing...' : 'Click to select specification file'}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">OpenAPI, AsyncAPI, JSON, YAML, or YML</p>
            </button>
          </div>
        </div>
      )}

      {/* Export Dialog */}
      {showExportDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-lg">
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-xl font-semibold">Export OpenAPI Specification</h2>
              <button
                onClick={() => setShowExportDialog(false)}
                className="text-muted-foreground hover:text-foreground"
              >
                ✕
              </button>
            </div>
            <p className="mb-6 text-sm text-muted-foreground">
              Export your current endpoints as an OpenAPI 3.0 specification file.
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => setShowExportDialog(false)}
                className="flex-1 rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent"
              >
                Cancel
              </button>
              <button
                onClick={handleExportOpenApi}
                className="flex-1 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
              >
                Export
              </button>
            </div>
          </div>
        </div>
      )}

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
