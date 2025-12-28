import { useEffect, useRef } from 'react'
import { AlertTriangle } from 'lucide-react'
import FocusTrap from 'focus-trap-react'

interface ConfirmDialogProps {
  isOpen: boolean
  onConfirm: () => void
  onCancel: () => void
  title: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
}

export default function ConfirmDialog({
  isOpen,
  onConfirm,
  onCancel,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
}: ConfirmDialogProps) {
  const cancelButtonRef = useRef<HTMLButtonElement>(null)

  // Handle escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onCancel()
      }
    }
    document.addEventListener('keydown', handleEscape)
    return () => document.removeEventListener('keydown', handleEscape)
  }, [isOpen, onCancel])

  // Prevent body scroll when dialog is open
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => {
      document.body.style.overflow = ''
    }
  }, [isOpen])

  if (!isOpen) {
    return null
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      aria-describedby="confirm-dialog-message"
    >
      <FocusTrap
        focusTrapOptions={{
          initialFocus: () => cancelButtonRef.current,
          allowOutsideClick: true,
          escapeDeactivates: false, // We handle escape ourselves
        }}
      >
        <div className="w-full max-w-md rounded-lg border border-border bg-card p-4 shadow-lg sm:p-6">
          <div className="mb-4 flex items-start space-x-3 sm:space-x-4">
            <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-destructive/10 sm:h-10 sm:w-10">
              <AlertTriangle className="h-4 w-4 text-destructive sm:h-5 sm:w-5" aria-hidden="true" />
            </div>
            <div>
              <h2 id="confirm-dialog-title" className="text-base font-semibold sm:text-lg">{title}</h2>
              <p id="confirm-dialog-message" className="mt-1 text-sm text-muted-foreground">{message}</p>
            </div>
          </div>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:gap-3">
            <button
              ref={cancelButtonRef}
              onClick={onCancel}
              className="w-full rounded-lg border border-border px-4 py-2 text-sm font-medium hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring sm:w-auto"
            >
              {cancelLabel}
            </button>
            <button
              onClick={onConfirm}
              className="w-full rounded-lg bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 focus:outline-none focus:ring-2 focus:ring-ring sm:w-auto"
            >
              {confirmLabel}
            </button>
          </div>
        </div>
      </FocusTrap>
    </div>
  )
}
