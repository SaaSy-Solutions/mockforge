import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { Plus, Trash2, Edit, Power, PowerOff, Globe, Zap, MessageSquare, Server, FileJson, Upload, Download, Loader2, Radio, Mail, Code, Search, X, Filter } from 'lucide-react'
import { toast } from 'sonner'
import FocusTrap from 'focus-trap-react'
import { endpointsApi, EndpointConfig, openApiApi } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useState, useRef, useMemo } from 'react'
import ConfirmDialog from '@/components/ConfirmDialog'
import yaml from 'js-yaml'

const PROTOCOL_OPTIONS = [
  { value: 'all', label: 'All Protocols' },
  { value: 'http', label: 'HTTP' },
  { value: 'grpc', label: 'gRPC' },
  { value: 'websocket', label: 'WebSocket' },
  { value: 'graphql', label: 'GraphQL' },
  { value: 'mqtt', label: 'MQTT' },
  { value: 'smtp', label: 'SMTP' },
  { value: 'amqp', label: 'AMQP' },
  { value: 'kafka', label: 'Kafka' },
]

export default function Dashboard() {
  const queryClient = useQueryClient()
  const [showImportDialog, setShowImportDialog] = useState(false)
  const [showExportDialog, setShowExportDialog] = useState(false)
  const [validationErrors, setValidationErrors] = useState<string[]>([])
  const [specPreview, setSpecPreview] = useState<{title?: string, version?: string, endpoints?: number} | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [deleteConfirmation, setDeleteConfirmation] = useState<{isOpen: boolean, endpoint: EndpointConfig | null}>({
    isOpen: false,
    endpoint: null,
  })
  const [isExporting, setIsExporting] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [protocolFilter, setProtocolFilter] = useState('all')

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
      setDeleteConfirmation({ isOpen: false, endpoint: null })
    },
    onError: () => {
      toast.error('Failed to delete endpoint')
      setDeleteConfirmation({ isOpen: false, endpoint: null })
    },
  })

  const handleDeleteClick = (endpoint: EndpointConfig) => {
    setDeleteConfirmation({ isOpen: true, endpoint })
  }

  const handleDeleteConfirm = () => {
    if (deleteConfirmation.endpoint) {
      deleteMutation.mutate(deleteConfirmation.endpoint.id)
    }
  }

  const handleDeleteCancel = () => {
    setDeleteConfirmation({ isOpen: false, endpoint: null })
  }

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
          // Parse YAML/YML files
          spec = yaml.load(content)
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
    setIsExporting(true)
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
    } finally {
      setIsExporting(false)
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
      case 'graphql':
        return <Code className="h-5 w-5" />
      case 'mqtt':
        return <Radio className="h-5 w-5" />
      case 'smtp':
        return <Mail className="h-5 w-5" />
      case 'amqp':
        return <Radio className="h-5 w-5" />
      case 'kafka':
        return <Server className="h-5 w-5" />
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
      case 'graphql':
        return 'bg-pink-500/10 text-pink-500'
      case 'mqtt':
        return 'bg-orange-500/10 text-orange-500'
      case 'smtp':
        return 'bg-teal-500/10 text-teal-500'
      case 'amqp':
        return 'bg-cyan-500/10 text-cyan-500'
      case 'kafka':
        return 'bg-amber-500/10 text-amber-500'
      default:
        return 'bg-gray-500/10 text-gray-500'
    }
  }

  // Filter endpoints based on search query and protocol filter
  const filteredEndpoints = useMemo(() => {
    if (!data?.endpoints) return []

    return data.endpoints.filter((endpoint: EndpointConfig) => {
      // Protocol filter
      if (protocolFilter !== 'all' && endpoint.protocol !== protocolFilter) {
        return false
      }

      // Search filter (search in name, description, path/topic)
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase()
        const nameMatch = endpoint.name.toLowerCase().includes(query)
        const descMatch = endpoint.description?.toLowerCase().includes(query) || false

        // Search in protocol-specific fields
        let pathMatch = false
        if (endpoint.config.type === 'Http') {
          pathMatch = endpoint.config.path?.toLowerCase().includes(query) || false
        } else if (endpoint.config.type === 'Grpc') {
          pathMatch = `${endpoint.config.service}.${endpoint.config.method}`.toLowerCase().includes(query)
        } else if (endpoint.config.type === 'Websocket') {
          pathMatch = endpoint.config.path?.toLowerCase().includes(query) || false
        } else if (endpoint.config.type === 'Graphql') {
          pathMatch = endpoint.config.path?.toLowerCase().includes(query) || false
        } else if (endpoint.config.type === 'Mqtt') {
          pathMatch = endpoint.config.topicPattern?.toLowerCase().includes(query) || false
        } else if (endpoint.config.type === 'Kafka') {
          pathMatch = endpoint.config.topic?.toLowerCase().includes(query) || false
        } else if (endpoint.config.type === 'Amqp') {
          pathMatch = `${endpoint.config.exchange}/${endpoint.config.routingKey}`.toLowerCase().includes(query)
        }

        return nameMatch || descMatch || pathMatch
      }

      return true
    })
  }, [data?.endpoints, searchQuery, protocolFilter])

  const clearFilters = () => {
    setSearchQuery('')
    setProtocolFilter('all')
  }

  const hasActiveFilters = searchQuery.trim() !== '' || protocolFilter !== 'all'

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
    <div className="h-full p-4 sm:p-6 md:p-8">
      {/* Header */}
      <div className="mb-6 sm:mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl sm:text-3xl font-bold">Endpoints</h1>
          <p className="mt-1 text-sm sm:text-base text-muted-foreground">
            Manage your mock endpoints and configurations
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2 sm:gap-3">
          <button
            onClick={() => setShowImportDialog(true)}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-card px-3 sm:px-4 py-2 text-sm font-medium hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <Upload className="h-4 w-4" aria-hidden="true" />
            <span className="hidden sm:inline">Import OpenAPI</span>
            <span className="sm:hidden">Import</span>
          </button>
          <button
            onClick={() => setShowExportDialog(true)}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-card px-3 sm:px-4 py-2 text-sm font-medium hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
            disabled={!data || data.endpoints.length === 0}
          >
            <Download className="h-4 w-4" aria-hidden="true" />
            <span className="hidden sm:inline">Export OpenAPI</span>
            <span className="sm:hidden">Export</span>
          </button>
          <Link
            to="/endpoints/new"
            className="inline-flex items-center space-x-2 rounded-lg bg-primary px-3 sm:px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
          >
            <Plus className="h-4 w-4" aria-hidden="true" />
            <span className="hidden sm:inline">New Endpoint</span>
            <span className="sm:hidden">New</span>
          </Link>
        </div>
      </div>

      {/* Search and Filter Bar */}
      <div className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search endpoints by name, path, or topic..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full rounded-lg border border-input bg-background py-2 pl-10 pr-10 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              aria-label="Clear search"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Filter className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground pointer-events-none" />
            <select
              value={protocolFilter}
              onChange={(e) => setProtocolFilter(e.target.value)}
              className="appearance-none rounded-lg border border-input bg-background py-2 pl-10 pr-8 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              {PROTOCOL_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          {hasActiveFilters && (
            <button
              onClick={clearFilters}
              className="inline-flex items-center gap-1 rounded-lg border border-border bg-secondary px-3 py-2 text-sm font-medium hover:bg-secondary/80"
            >
              <X className="h-3 w-3" />
              <span className="hidden sm:inline">Clear filters</span>
            </button>
          )}
        </div>
      </div>

      {/* Filter Results Info */}
      {hasActiveFilters && (
        <div className="mb-4 text-sm text-muted-foreground">
          Showing {filteredEndpoints.length} of {data?.endpoints?.length || 0} endpoints
          {searchQuery && <span> matching "{searchQuery}"</span>}
          {protocolFilter !== 'all' && <span> in {PROTOCOL_OPTIONS.find(p => p.value === protocolFilter)?.label}</span>}
        </div>
      )}

      {/* Import Dialog */}
      {showImportDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="import-dialog-title"
        >
          <FocusTrap focusTrapOptions={{ allowOutsideClick: true }}>
            <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-lg">
              <div className="mb-4 flex items-center justify-between">
                <h2 id="import-dialog-title" className="text-xl font-semibold">Import OpenAPI Specification</h2>
                <button
                  onClick={() => {
                    setShowImportDialog(false)
                    setValidationErrors([])
                    setSpecPreview(null)
                  }}
                  className="text-muted-foreground hover:text-foreground p-1 rounded-md hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring"
                  aria-label="Close import dialog"
                >
                  <span aria-hidden="true">✕</span>
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
              className="w-full rounded-lg border-2 border-dashed border-border bg-accent/50 px-4 py-8 text-center hover:bg-accent disabled:opacity-50 focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <FileJson className="mx-auto mb-2 h-8 w-8 text-muted-foreground" aria-hidden="true" />
              <p className="text-sm font-medium">
                {importOpenApiMutation.isPending ? 'Importing...' : 'Click to select specification file'}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">OpenAPI, AsyncAPI, JSON, YAML, or YML</p>
            </button>
            </div>
          </FocusTrap>
        </div>
      )}

      {/* Export Dialog */}
      {showExportDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="export-dialog-title"
        >
          <FocusTrap focusTrapOptions={{ allowOutsideClick: true }}>
            <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-lg">
              <div className="mb-4 flex items-center justify-between">
                <h2 id="export-dialog-title" className="text-xl font-semibold">Export OpenAPI Specification</h2>
                <button
                  onClick={() => setShowExportDialog(false)}
                  className="text-muted-foreground hover:text-foreground p-1 rounded-md hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring"
                  aria-label="Close export dialog"
                >
                  <span aria-hidden="true">✕</span>
                </button>
              </div>
              <p className="mb-6 text-sm text-muted-foreground">
                Export your current endpoints as an OpenAPI 3.0 specification file.
              </p>
              <div className="flex flex-col-reverse gap-2 sm:flex-row sm:gap-3">
                <button
                  onClick={() => setShowExportDialog(false)}
                  disabled={isExporting}
                  className="flex-1 rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  Cancel
                </button>
                <button
                  onClick={handleExportOpenApi}
                  disabled={isExporting}
                  className="flex-1 inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-ring"
                >
                  {isExporting ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
                      <span>Exporting...</span>
                    </>
                  ) : (
                    <span>Export</span>
                  )}
                </button>
              </div>
            </div>
          </FocusTrap>
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={deleteConfirmation.isOpen}
        onConfirm={handleDeleteConfirm}
        onCancel={handleDeleteCancel}
        title="Delete Endpoint"
        message={`Are you sure you want to delete "${deleteConfirmation.endpoint?.name || ''}"? This action cannot be undone.`}
        confirmLabel="Delete"
        cancelLabel="Cancel"
      />

      {/* Stats */}
      <div className="mb-8 grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8">
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold">{data?.total || 0}</div>
          <div className="text-xs text-muted-foreground">Total</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-green-500">{data?.enabled || 0}</div>
          <div className="text-xs text-muted-foreground">Enabled</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-blue-500">{data?.by_protocol?.http || 0}</div>
          <div className="text-xs text-muted-foreground">HTTP</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-purple-500">{data?.by_protocol?.grpc || 0}</div>
          <div className="text-xs text-muted-foreground">gRPC</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-green-500">{data?.by_protocol?.websocket || 0}</div>
          <div className="text-xs text-muted-foreground">WebSocket</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-pink-500">{data?.by_protocol?.graphql || 0}</div>
          <div className="text-xs text-muted-foreground">GraphQL</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-orange-500">{data?.by_protocol?.mqtt || 0}</div>
          <div className="text-xs text-muted-foreground">MQTT</div>
        </div>
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-2xl font-bold text-teal-500">{data?.by_protocol?.smtp || 0}</div>
          <div className="text-xs text-muted-foreground">SMTP</div>
        </div>
      </div>

      {/* Endpoints list */}
      {data && data.endpoints.length > 0 ? (
        <div className="space-y-4">
          {filteredEndpoints.length === 0 && hasActiveFilters ? (
            <div className="rounded-lg border border-dashed border-border bg-card p-8 text-center">
              <Search className="mx-auto h-8 w-8 text-muted-foreground" />
              <h3 className="mt-3 text-lg font-semibold">No matching endpoints</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Try adjusting your search or filter criteria
              </p>
              <button
                onClick={clearFilters}
                className="mt-4 inline-flex items-center gap-2 rounded-lg bg-secondary px-4 py-2 text-sm font-medium hover:bg-secondary/80"
              >
                <X className="h-4 w-4" />
                Clear filters
              </button>
            </div>
          ) : (
          filteredEndpoints.map((endpoint: EndpointConfig) => (
            <div
              key={endpoint.id}
              className="rounded-lg border border-border bg-card p-4 sm:p-6 transition-shadow hover:shadow-md"
            >
              <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
                <div className="flex items-start space-x-3 sm:space-x-4 min-w-0">
                  <div className={cn('rounded-lg p-2 sm:p-3 shrink-0', getProtocolColor(endpoint.protocol))}>
                    {getProtocolIcon(endpoint.protocol)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex flex-wrap items-center gap-2 sm:gap-3">
                      <h3 className="text-base sm:text-lg font-semibold truncate">{endpoint.name}</h3>
                      <span className="rounded-full bg-secondary px-2 py-0.5 text-xs font-medium uppercase shrink-0">
                        {endpoint.protocol}
                      </span>
                      {endpoint.enabled ? (
                        <Power className="h-4 w-4 text-green-500 shrink-0" aria-label="Enabled" />
                      ) : (
                        <PowerOff className="h-4 w-4 text-muted-foreground shrink-0" aria-label="Disabled" />
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
                      {endpoint.config.type === 'Graphql' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          {endpoint.config.path || '/graphql'}
                        </div>
                      )}
                      {endpoint.config.type === 'Mqtt' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          mqtt://{endpoint.config.topicPattern || '#'}
                        </div>
                      )}
                      {endpoint.config.type === 'Smtp' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          smtp://{endpoint.config.hostname || 'localhost'}:{endpoint.config.port}
                        </div>
                      )}
                      {endpoint.config.type === 'Amqp' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          amqp://{endpoint.config.exchange}/{endpoint.config.routingKey}
                        </div>
                      )}
                      {endpoint.config.type === 'Kafka' && (
                        <div className="text-sm font-mono text-muted-foreground">
                          kafka://{endpoint.config.topic}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
                <div className="flex items-center space-x-1 sm:space-x-2 shrink-0">
                  <Link
                    to={`/endpoints/${endpoint.id}`}
                    className="rounded-lg p-2 text-muted-foreground hover:bg-accent hover:text-accent-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                    aria-label={`Edit endpoint ${endpoint.name}`}
                  >
                    <Edit className="h-4 w-4" aria-hidden="true" />
                  </Link>
                  <button
                    onClick={() => handleDeleteClick(endpoint)}
                    className="rounded-lg p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive focus:outline-none focus:ring-2 focus:ring-ring"
                    aria-label={`Delete endpoint ${endpoint.name}`}
                  >
                    <Trash2 className="h-4 w-4" aria-hidden="true" />
                  </button>
                </div>
              </div>
            </div>
          ))
          )}
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
