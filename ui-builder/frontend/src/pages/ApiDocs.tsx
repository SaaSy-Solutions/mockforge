import { FileText } from 'lucide-react'
// @ts-ignore - swagger-ui-react types not available
import SwaggerUI from 'swagger-ui-react'
import 'swagger-ui-react/swagger-ui.css'

export default function ApiDocs() {
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
        <SwaggerUI
          url="/api/openapi/export"
          docExpansion="list"
          defaultModelsExpandDepth={1}
          defaultModelExpandDepth={1}
        />
      </div>
    </div>
  )
}
