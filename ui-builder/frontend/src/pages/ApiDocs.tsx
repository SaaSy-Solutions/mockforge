import { useState, useEffect, Suspense, lazy } from 'react'
import { FileText, Loader2, AlertCircle, RefreshCw } from 'lucide-react'
import 'swagger-ui-react/swagger-ui.css'

// Lazy load SwaggerUI for better initial load performance
const SwaggerUI = lazy(() => import('swagger-ui-react'))

// Loading fallback component
function SwaggerLoading() {
  return (
    <div className="flex h-64 items-center justify-center">
      <div className="text-center">
        <Loader2 className="mx-auto h-8 w-8 animate-spin text-primary" />
        <p className="mt-4 text-sm text-muted-foreground">Loading API documentation...</p>
      </div>
    </div>
  )
}

// Error fallback component
function SwaggerError({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex h-64 items-center justify-center">
      <div className="mx-auto max-w-md text-center">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
          <AlertCircle className="h-6 w-6 text-destructive" />
        </div>
        <h2 className="text-lg font-semibold text-foreground">Failed to load API documentation</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          Unable to fetch the OpenAPI specification. The mock server may not be running or the endpoint is unavailable.
        </p>
        <button
          onClick={onRetry}
          className="mt-4 inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <RefreshCw className="h-4 w-4" />
          Retry
        </button>
      </div>
    </div>
  )
}

export default function ApiDocs() {
  const [hasError, setHasError] = useState(false)
  const [key, setKey] = useState(0)

  // Check if the OpenAPI spec is accessible
  useEffect(() => {
    const checkSpec = async () => {
      try {
        const response = await fetch('/api/openapi/export')
        if (!response.ok) {
          setHasError(true)
        }
      } catch {
        setHasError(true)
      }
    }
    checkSpec()
  }, [key])

  const handleRetry = () => {
    setHasError(false)
    setKey(prev => prev + 1)
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-card p-6">
        <div className="flex items-center space-x-3">
          <FileText className="h-8 w-8 text-primary" />
          <div>
            <h1 className="text-2xl font-bold">API Documentation</h1>
            <p className="text-sm text-muted-foreground">
              Interactive documentation for your mock endpoints
            </p>
          </div>
        </div>
      </div>

      {/* Swagger UI */}
      <div className="flex-1 overflow-auto bg-background p-6">
        {hasError ? (
          <SwaggerError onRetry={handleRetry} />
        ) : (
          <Suspense fallback={<SwaggerLoading />}>
            <SwaggerUI
              key={key}
              url="/api/openapi/export"
              docExpansion="list"
              defaultModelsExpandDepth={1}
              defaultModelExpandDepth={1}
              onComplete={() => setHasError(false)}
            />
          </Suspense>
        )}
      </div>
    </div>
  )
}
