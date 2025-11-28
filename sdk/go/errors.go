// Package mockforge provides standardized error types for MockForge SDK
package mockforge

import "fmt"

// MockServerErrorCode represents error codes for MockServer operations
type MockServerErrorCode string

const (
	ErrorCodeCLINotFound         MockServerErrorCode = "CLI_NOT_FOUND"
	ErrorCodeServerStartFailed   MockServerErrorCode = "SERVER_START_FAILED"
	ErrorCodePortDetectionFailed MockServerErrorCode = "PORT_DETECTION_FAILED"
	ErrorCodeAdminAPIError       MockServerErrorCode = "ADMIN_API_ERROR"
	ErrorCodeHealthCheckTimeout  MockServerErrorCode = "HEALTH_CHECK_TIMEOUT"
	ErrorCodeInvalidConfig       MockServerErrorCode = "INVALID_CONFIG"
	ErrorCodeStubNotFound        MockServerErrorCode = "STUB_NOT_FOUND"
	ErrorCodeNetworkError        MockServerErrorCode = "NETWORK_ERROR"
	ErrorCodeUnknownError        MockServerErrorCode = "UNKNOWN_ERROR"
)

// MockServerError represents a standardized error for MockServer operations
type MockServerError struct {
	Code    MockServerErrorCode
	Message string
	Cause   error
	Details map[string]interface{}
}

// Error implements the error interface
func (e *MockServerError) Error() string {
	return e.Message
}

// Unwrap returns the underlying error (for error wrapping)
func (e *MockServerError) Unwrap() error {
	return e.Cause
}

// NewCLINotFoundError creates an error for CLI not found
func NewCLINotFoundError(cause error) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeCLINotFound,
		Message: "MockForge CLI not found. Install with: cargo install mockforge-cli",
		Cause:   cause,
		Details: map[string]interface{}{
			"hint": "Ensure mockforge is in your PATH",
		},
	}
}

// NewServerStartFailedError creates an error for server start failure
func NewServerStartFailedError(message string, cause error) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeServerStartFailed,
		Message: fmt.Sprintf("Failed to start MockForge server: %s", message),
		Cause:   cause,
	}
}

// NewPortDetectionFailedError creates an error for port detection failure
func NewPortDetectionFailedError(cause error) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodePortDetectionFailed,
		Message: "Failed to detect server port from MockForge output. The server may have failed to start.",
		Cause:   cause,
		Details: map[string]interface{}{
			"hint": "Check that mockforge CLI is installed and the server started successfully",
		},
	}
}

// NewAdminAPIError creates an error for Admin API operations
func NewAdminAPIError(operation, message string, cause error) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeAdminAPIError,
		Message: fmt.Sprintf("Admin API %s failed: %s", operation, message),
		Cause:   cause,
		Details: map[string]interface{}{
			"operation": operation,
		},
	}
}

// NewHealthCheckTimeoutError creates an error for health check timeout
func NewHealthCheckTimeoutError(timeout int, port int) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeHealthCheckTimeout,
		Message: fmt.Sprintf("Health check timed out after %dms. Could not connect to http://127.0.0.1:%d/health", timeout, port),
		Details: map[string]interface{}{
			"timeout": timeout,
			"port":    port,
			"hint":    "Check that the server started successfully",
		},
	}
}

// NewInvalidConfigError creates an error for invalid configuration
func NewInvalidConfigError(message string, details map[string]interface{}) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeInvalidConfig,
		Message: fmt.Sprintf("Invalid configuration: %s", message),
		Details: details,
	}
}

// NewStubNotFoundError creates an error for stub not found
func NewStubNotFoundError(method, path string) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeStubNotFound,
		Message: fmt.Sprintf("Stub not found: %s %s", method, path),
		Details: map[string]interface{}{
			"method": method,
			"path":   path,
		},
	}
}

// NewNetworkError creates an error for network operations
func NewNetworkError(message string, cause error) *MockServerError {
	return &MockServerError{
		Code:    ErrorCodeNetworkError,
		Message: fmt.Sprintf("Network error: %s", message),
		Cause:   cause,
	}
}
