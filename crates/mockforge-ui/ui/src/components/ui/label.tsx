import * as React from "react"
import { cn } from "../../utils/cn"

export interface LabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  /** Show a required indicator (*) */
  required?: boolean;
}

const Label = React.forwardRef<HTMLLabelElement, LabelProps>(
  ({ className, children, required, ...props }, ref) => (
    <label
      ref={ref}
      className={cn(
        "text-sm font-medium leading-none text-gray-900 dark:text-gray-100 peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className
      )}
      {...props}
    >
      {children}
      {required && (
        <span className="text-red-500 ml-0.5" aria-hidden="true">*</span>
      )}
    </label>
  )
)
Label.displayName = "Label"

export { Label }
