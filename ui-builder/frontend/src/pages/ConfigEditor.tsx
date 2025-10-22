import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Download, Upload, Save } from 'lucide-react'
import Editor from '@monaco-editor/react'
import { configApi } from '@/lib/api'

export default function ConfigEditor() {
  const queryClient = useQueryClient()
  const [config, setConfig] = useState<string>('')
  const [format, setFormat] = useState<'yaml' | 'json'>('yaml')

  const { isLoading } = useQuery({
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
    }
  }

  const handleImport = () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.yaml,.yml,.json'
    input.onchange = async (e: any) => {
      const file = e.target.files[0]
      if (!file) return

      const reader = new FileReader()
      reader.onload = (e) => {
        const content = e.target?.result as string
        setConfig(content)
        const ext = file.name.split('.').pop()
        setFormat(ext === 'json' ? 'json' : 'yaml')
        toast.success('Configuration imported')
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

  return (
    <div className="h-full p-8">
      {/* Header */}
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Configuration Editor</h1>
          <p className="mt-1 text-muted-foreground">
            Edit your MockForge server configuration
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <button
            onClick={handleImport}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-accent"
          >
            <Upload className="h-4 w-4" />
            <span>Import</span>
          </button>
          <button
            onClick={handleExport}
            className="inline-flex items-center space-x-2 rounded-lg border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-accent"
          >
            <Download className="h-4 w-4" />
            <span>Export</span>
          </button>
          <button
            onClick={() => saveMutation.mutate()}
            disabled={saveMutation.isPending}
            className="inline-flex items-center space-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Save className="h-4 w-4" />
            <span>{saveMutation.isPending ? 'Saving...' : 'Save'}</span>
          </button>
        </div>
      </div>

      {/* Format Selector */}
      <div className="mb-4 flex items-center space-x-2">
        <label className="text-sm font-medium">Format:</label>
        <div className="flex space-x-2">
          <button
            onClick={() => setFormat('yaml')}
            className={`rounded-lg px-3 py-1 text-sm ${
              format === 'yaml'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary text-secondary-foreground'
            }`}
          >
            YAML
          </button>
          <button
            onClick={() => setFormat('json')}
            className={`rounded-lg px-3 py-1 text-sm ${
              format === 'json'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary text-secondary-foreground'
            }`}
          >
            JSON
          </button>
        </div>
      </div>

      {/* Editor */}
      <div className="rounded-lg border border-border overflow-hidden" style={{ height: 'calc(100vh - 300px)' }}>
        <Editor
          height="100%"
          defaultLanguage={format}
          language={format}
          value={config}
          onChange={(value) => setConfig(value || '')}
          theme="vs-dark"
          options={{
            minimap: { enabled: true },
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
