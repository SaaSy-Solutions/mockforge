import * as React from "react"
import { cn } from "../../utils/cn"

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  /** Error message to display - sets aria-invalid and aria-describedby */
  error?: string;
  /** ID for the error message element */
  errorId?: string;
}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, error, errorId, "aria-invalid": ariaInvalid, "aria-describedby": ariaDescribedby, ...props }, ref) => {
    const hasError = !!error || ariaInvalid === true || ariaInvalid === "true";
    const describedBy = [ariaDescribedby, errorId].filter(Boolean).join(" ") || undefined;

    return (
      <textarea
        className={cn(
          "flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          hasError && "border-red-500 focus-visible:ring-red-500",
          className
        )}
        ref={ref}
        aria-invalid={hasError || undefined}
        aria-describedby={describedBy}
        {...props}
      />
    )
  }
)
Textarea.displayName = "Textarea"

export { Textarea }
