import * as React from "react"
import { cn } from "../../utils/cn"

export interface FormMessageProps extends React.HTMLAttributes<HTMLParagraphElement> {
  /** The error or helper message to display */
  children?: React.ReactNode;
  /** Whether this is an error message (affects styling and aria-live) */
  error?: boolean;
}

/**
 * Accessible form message component for displaying errors and helper text.
 *
 * Features:
 * - Uses aria-live="polite" for error messages to announce to screen readers
 * - Uses role="alert" for immediate error announcements
 * - Supports both error and helper text styling
 *
 * @example
 * ```tsx
 * <div>
 *   <Label htmlFor="email">Email</Label>
 *   <Input
 *     id="email"
 *     error={errors.email}
 *     errorId="email-error"
 *     aria-describedby="email-error"
 *   />
 *   <FormMessage id="email-error" error>
 *     {errors.email}
 *   </FormMessage>
 * </div>
 * ```
 */
const FormMessage = React.forwardRef<HTMLParagraphElement, FormMessageProps>(
  ({ className, children, error = false, ...props }, ref) => {
    if (!children) {
      return null;
    }

    return (
      <p
        ref={ref}
        role={error ? "alert" : undefined}
        aria-live={error ? "polite" : undefined}
        className={cn(
          "text-sm mt-1.5",
          error
            ? "text-red-600 dark:text-red-400"
            : "text-muted-foreground",
          className
        )}
        {...props}
      >
        {children}
      </p>
    );
  }
);
FormMessage.displayName = "FormMessage";

export { FormMessage };
