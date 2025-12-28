import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Download, Upload, Save, Loader2, X, RefreshCw } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { configApi } from '@/lib/api'
import EditorSkeleton from '@/components/EditorSkeleton'

export default function ConfigEditor() {
  const queryClient = useQueryClient()
  const [config, setConfig] = useState<string>('')
  const [format, setFormat] = useState<'yaml' | 'json'>('yaml')
  const [isExporting, setIsExporting] = useState(false)
  const [isImporting, setIsImporting] = useState(false)

  const { isLoading, isError, error } = useQuery({
    queryKey: ['config'],
    queryFn: async () => {
      const response = await configApi.get()
      const yaml = await configApi.export()
      setConfig(yaml.data)
      return response.data
    },
  })

  const saveMutation = useMutation({
    mutationFn: () => configApi.import(config, format),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['config'] })
      toast.success('Configuration saved successfully')
    },
    onError: () => {
      toast.error('Failed to save configuration')
    },
  })

  const handleExport = async () => {
    setIsExporting(true)
    try {
      const response = await configApi.export()
      const blob = new Blob([response.data], { type: 'application/x-yaml' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'mockforge-config.yaml'
      a.click()
      URL.revokeObjectURL(url)
      toast.success('Configuration exported')
    } catch (error) {
      toast.error('Failed to export configuration')
    } finally {
      setIsExporting(false)
    }
  }

  const handleImport = () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.yaml,.yml,.json'
    input.onchange = async (e: any) => {
      const file = e.target.files[0]
      if (!file) return

      setIsImporting(true)
      const reader = new FileReader()
      reader.onload = (e) => {
        const content = e.target?.result as string
        setConfig(content)
        const ext = file.name.split('.').pop()
        setFormat(ext === 'json' ? 'json' : 'yaml')
        toast.success('Configuration imported')
        setIsImporting(false)
      }
      reader.onerror = () => {
        toast.error('Failed to read file')
        setIsImporting(false)
      }
      reader.readAsText(file)
    }
    input.click()
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"></div>
          <p className="mt-4 text-sm text-muted-foreground">Loading configuration...</p>
        </div>
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center max-w-md">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
            <X className="h-6 w-6 text-destructive" />
          </div>
          <h2 className="text-lg font-semibold text-foreground">Failed to load configuration</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            {error instanceof Error ? error.message : 'An unexpected error occurred. Please try again.'}
          </p>
          <button
            onClick={() => queryClient.invalidateQueries({ queryKey: ['config'] })}
            className="mt-4 inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <RefreshCw className="h-4 w-4" />
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full p-4 md:p-8">
      {/* Header */}
      <div className="mb-6 md:mb-8 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl md:text-3xl font-bold">Configuration Editor</h1>
          <p className="mt-1 text-sm md:text-base text-muted-foreground">
            Edit your MockForge server configuration
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={handleImport}
            disabled={isImporting || isExporting || saveMutation.isPending}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-background px-3 py-2 text-sm font-medium hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
            title="Import configuration"
          >
            {isImporting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Upload className="h-4 w-4" />
            )}
            <span className="hidden sm:inline">{isImporting ? 'Importing...' : 'Import'}</span>
          </button>
          <button
            onClick={handleExport}
            disabled={isExporting || isImporting || saveMutation.isPending}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-background px-3 py-2 text-sm font-medium hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
            title="Export configuration"
          >
            {isExporting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Download className="h-4 w-4" />
            )}
            <span className="hidden sm:inline">{isExporting ? 'Exporting...' : 'Export'}</span>
          </button>
          <button
            onClick={() => saveMutation.mutate()}
            disabled={saveMutation.isPending || isImporting || isExporting}
            className="inline-flex items-center space-x-2 rounded-lg bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
            title="Save configuration"
          >
            {saveMutation.isPending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Save className="h-4 w-4" />
            )}
            <span className="hidden sm:inline">{saveMutation.isPending ? 'Saving...' : 'Save'}</span>
          </button>
        </div>
      </div>

      {/* Format Selector */}
      <div className="mb-4 flex items-center space-x-2">
        <label className="text-sm font-medium">Format:</label>
        <div className="flex space-x-2">
          <button
            onClick={() => setFormat('yaml')}
            className={`rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
              format === 'yaml'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
            }`}
          >
            YAML
          </button>
          <button
            onClick={() => setFormat('json')}
            className={`rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
              format === 'json'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
            }`}
          >
            JSON
          </button>
        </div>
      </div>

      {/* Editor */}
      <div className="rounded-lg border border-border overflow-hidden" style={{ height: 'calc(100vh - 280px)', minHeight: '300px' }}>
        <Editor
          height="100%"
          defaultLanguage={format}
          language={format}
          value={config}
          onChange={(value) => setConfig(value || '')}
          theme="vs-dark"
          loading={<EditorSkeleton height="100%" />}
          options={{
            minimap: { enabled: window.innerWidth > 768 },
            fontSize: 14,
            wordWrap: 'on',
            formatOnPaste: true,
            formatOnType: true,
          }}
        />
      </div>
    </div>
  )
}
