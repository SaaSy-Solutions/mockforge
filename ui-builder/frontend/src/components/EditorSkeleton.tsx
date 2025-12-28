import { cn } from '@/lib/utils'

interface EditorSkeletonProps {
  height?: string
  className?: string
}

/**
 * A skeleton loading state for Monaco Editor
 */
export default function EditorSkeleton({ height = '300px', className }: EditorSkeletonProps) {
  return (
    <div
      className={cn('rounded-lg border border-border bg-[#1e1e1e] overflow-hidden', className)}
      style={{ height }}
      role="status"
      aria-label="Loading editor"
    >
      <div className="animate-pulse p-4 space-y-3">
        {/* Line numbers column */}
        <div className="flex gap-4">
          <div className="w-8 space-y-2">
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} className="h-4 bg-gray-700/50 rounded w-full" />
            ))}
          </div>
          {/* Code lines */}
          <div className="flex-1 space-y-2">
            <div className="h-4 bg-gray-700/30 rounded w-3/4" />
            <div className="h-4 bg-gray-700/30 rounded w-1/2" />
            <div className="h-4 bg-gray-700/30 rounded w-5/6" />
            <div className="h-4 bg-gray-700/30 rounded w-2/3" />
            <div className="h-4 bg-gray-700/30 rounded w-3/5" />
            <div className="h-4 bg-gray-700/30 rounded w-4/5" />
            <div className="h-4 bg-gray-700/30 rounded w-1/3" />
            <div className="h-4 bg-gray-700/30 rounded w-2/4" />
          </div>
        </div>
      </div>
      <span className="sr-only">Loading code editor...</span>
    </div>
  )
}
