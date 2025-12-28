import { useState, useCallback, useEffect } from 'react'
import { z } from 'zod'
import { ValidationErrors, validateField } from '@/lib/validation'

interface UseFormValidationOptions {
  schema: z.ZodSchema
  validateOnChange?: boolean
  validateOnBlur?: boolean
}

interface UseFormValidationReturn {
  errors: ValidationErrors
  touched: Record<string, boolean>
  validate: () => boolean
  validateFieldByPath: (path: string, value: unknown) => boolean
  setFieldTouched: (path: string) => void
  clearErrors: () => void
  clearFieldError: (path: string) => void
  getFieldError: (path: string) => string | undefined
  hasError: (path: string) => boolean
  isValid: boolean
}

/**
 * Hook for form validation with Zod schemas
 */
export function useFormValidation<T>(
  data: T,
  options: UseFormValidationOptions
): UseFormValidationReturn {
  const { schema, validateOnChange = false } = options

  const [errors, setErrors] = useState<ValidationErrors>({})
  const [touched, setTouched] = useState<Record<string, boolean>>({})
  const [isValid, setIsValid] = useState(true)

  // Validate entire form
  const validate = useCallback((): boolean => {
    const result = schema.safeParse(data)

    if (result.success) {
      setErrors({})
      setIsValid(true)
      return true
    }

    const newErrors: ValidationErrors = {}
    for (const issue of result.error.issues) {
      const path = issue.path.join('.')
      newErrors[path] = issue.message
    }
    setErrors(newErrors)
    setIsValid(false)
    return false
  }, [data, schema])

  // Validate on change if enabled
  useEffect(() => {
    if (validateOnChange) {
      validate()
    }
  }, [data, validateOnChange, validate])

  // Validate a specific field by path
  const validateFieldByPath = useCallback(
    (path: string, value: unknown): boolean => {
      // Extract the appropriate schema for the field
      // This is a simplified approach - for complex nested schemas,
      // you may need to traverse the schema structure
      try {
        const pathParts = path.split('.')
        let currentSchema: z.ZodSchema = schema

        for (const part of pathParts) {
          if (currentSchema instanceof z.ZodObject) {
            currentSchema = currentSchema.shape[part]
          } else {
            // Can't traverse further
            break
          }
        }

        const result = currentSchema.safeParse(value)

        if (result.success) {
          setErrors((prev) => {
            const newErrors = { ...prev }
            delete newErrors[path]
            return newErrors
          })
          return true
        }

        setErrors((prev) => ({
          ...prev,
          [path]: result.error.issues[0]?.message || 'Invalid value',
        }))
        return false
      } catch {
        return true // If we can't find the schema, assume valid
      }
    },
    [schema]
  )

  // Mark a field as touched
  const setFieldTouched = useCallback((path: string) => {
    setTouched((prev) => ({ ...prev, [path]: true }))
  }, [])

  // Clear all errors
  const clearErrors = useCallback(() => {
    setErrors({})
    setIsValid(true)
  }, [])

  // Clear error for a specific field
  const clearFieldError = useCallback((path: string) => {
    setErrors((prev) => {
      const newErrors = { ...prev }
      delete newErrors[path]
      return newErrors
    })
  }, [])

  // Get error for a specific field
  const getFieldError = useCallback(
    (path: string): string | undefined => {
      return errors[path]
    },
    [errors]
  )

  // Check if a field has an error
  const hasError = useCallback(
    (path: string): boolean => {
      return path in errors
    },
    [errors]
  )

  return {
    errors,
    touched,
    validate,
    validateFieldByPath,
    setFieldTouched,
    clearErrors,
    clearFieldError,
    getFieldError,
    hasError,
    isValid,
  }
}

/**
 * Simple validation for individual field values
 */
export function useFieldValidation(schema: z.ZodSchema) {
  const [error, setError] = useState<string | undefined>()
  const [touched, setTouched] = useState(false)

  const validate = useCallback(
    (value: unknown): boolean => {
      const result = validateField(schema, value)
      if (result.valid) {
        setError(undefined)
        return true
      }
      setError(result.error)
      return false
    },
    [schema]
  )

  const touch = useCallback(() => {
    setTouched(true)
  }, [])

  const clear = useCallback(() => {
    setError(undefined)
  }, [])

  return {
    error,
    touched,
    showError: touched && error !== undefined,
    validate,
    touch,
    clear,
  }
}
