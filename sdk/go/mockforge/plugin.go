// Package mockforge provides a Go SDK for building MockForge plugins
//
// This SDK allows developers to write MockForge plugins in Go, compiled to
// WebAssembly using TinyGo. Plugins can implement authentication, template
// functions, response generation, and data source integrations.
//
// Example usage:
//
//	type MyAuthPlugin struct{}
//
//	func (p *MyAuthPlugin) Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error) {
//	    // Your authentication logic here
//	    return &AuthResult{
//	        Authenticated: true,
//	        UserID: "user123",
//	        Claims: map[string]interface{}{"role": "admin"},
//	    }, nil
//	}
//
//	func (p *MyAuthPlugin) GetCapabilities() *PluginCapabilities {
//	    return &PluginCapabilities{
//	        Network: NetworkCapabilities{
//	            AllowHTTPOutbound: false,
//	        },
//	        Filesystem: FilesystemCapabilities{
//	            AllowRead: false,
//	        },
//	        Resources: ResourceLimits{
//	            MaxMemoryBytes: 10 * 1024 * 1024, // 10MB
//	            MaxCPUTimeMs: 1000, // 1 second
//	        },
//	    }
//	}
//
//	func main() {
//	    plugin := &MyAuthPlugin{}
//	    mockforge.ExportAuthPlugin(plugin)
//	}
package mockforge

import (
	"encoding/json"
	"fmt"
)

// PluginContext contains information about the current request
type PluginContext struct {
	Method  string            `json:"method"`
	URI     string            `json:"uri"`
	Headers map[string]string `json:"headers"`
	Body    []byte            `json:"body,omitempty"`
}

// AuthCredentials represents authentication credentials
type AuthCredentials struct {
	Type  string            `json:"type"`
	Token string            `json:"token,omitempty"`
	Data  map[string]string `json:"data,omitempty"`
}

// AuthResult represents the result of authentication
type AuthResult struct {
	Authenticated bool                   `json:"authenticated"`
	UserID        string                 `json:"user_id"`
	Claims        map[string]interface{} `json:"claims"`
}

// PluginCapabilities defines what a plugin can access
type PluginCapabilities struct {
	Network    NetworkCapabilities    `json:"network"`
	Filesystem FilesystemCapabilities `json:"filesystem"`
	Resources  ResourceLimits         `json:"resources"`
}

// NetworkCapabilities defines network access permissions
type NetworkCapabilities struct {
	AllowHTTPOutbound bool     `json:"allow_http_outbound"`
	AllowedHosts      []string `json:"allowed_hosts,omitempty"`
}

// FilesystemCapabilities defines filesystem access permissions
type FilesystemCapabilities struct {
	AllowRead    bool     `json:"allow_read"`
	AllowWrite   bool     `json:"allow_write"`
	AllowedPaths []string `json:"allowed_paths,omitempty"`
}

// ResourceLimits defines resource constraints
type ResourceLimits struct {
	MaxMemoryBytes uint64 `json:"max_memory_bytes"`
	MaxCPUTimeMs   uint64 `json:"max_cpu_time_ms"`
}

// ResponseRequest contains information about the request to generate a response for
type ResponseRequest struct {
	Method  string            `json:"method"`
	Path    string            `json:"path"`
	Headers map[string]string `json:"headers"`
	Body    []byte            `json:"body,omitempty"`
}

// ResponseData contains the generated response
type ResponseData struct {
	StatusCode  int               `json:"status_code"`
	Headers     map[string]string `json:"headers"`
	Body        []byte            `json:"body"`
	ContentType string            `json:"content_type"`
}

// DataQuery represents a query to a data source
type DataQuery struct {
	Query      string                 `json:"query"`
	Parameters map[string]interface{} `json:"parameters"`
}

// DataResult contains the result of a data query
type DataResult struct {
	Columns []ColumnInfo      `json:"columns"`
	Rows    []map[string]interface{} `json:"rows"`
}

// ColumnInfo describes a column in a data result
type ColumnInfo struct {
	Name     string `json:"name"`
	DataType string `json:"data_type"`
}

// ResolutionContext provides context for template resolution
type ResolutionContext struct {
	Environment    map[string]string `json:"environment"`
	RequestContext *PluginContext    `json:"request_context,omitempty"`
}

// PluginError represents an error from a plugin
type PluginError struct {
	Message string `json:"message"`
	Code    int    `json:"code"`
}

func (e *PluginError) Error() string {
	return fmt.Sprintf("plugin error [%d]: %s", e.Code, e.Message)
}

// ============================================================================
// Plugin Interfaces
// ============================================================================

// AuthPlugin is the interface for authentication plugins
type AuthPlugin interface {
	// Authenticate validates credentials and returns authentication result
	Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error)

	// GetCapabilities returns the capabilities this plugin requires
	GetCapabilities() *PluginCapabilities
}

// TemplatePlugin is the interface for template function plugins
type TemplatePlugin interface {
	// ExecuteFunction executes a template function with the given arguments
	ExecuteFunction(functionName string, args []interface{}, ctx *ResolutionContext) (interface{}, error)

	// GetFunctions returns the list of functions this plugin provides
	GetFunctions() []TemplateFunction

	// GetCapabilities returns the capabilities this plugin requires
	GetCapabilities() *PluginCapabilities
}

// TemplateFunction describes a template function
type TemplateFunction struct {
	Name        string              `json:"name"`
	Description string              `json:"description"`
	Parameters  []FunctionParameter `json:"parameters"`
	ReturnType  string              `json:"return_type"`
}

// FunctionParameter describes a function parameter
type FunctionParameter struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Required    bool   `json:"required"`
	Description string `json:"description"`
}

// ResponsePlugin is the interface for response generator plugins
type ResponsePlugin interface {
	// GenerateResponse generates a response based on the request
	GenerateResponse(ctx *PluginContext, req *ResponseRequest) (*ResponseData, error)

	// GetCapabilities returns the capabilities this plugin requires
	GetCapabilities() *PluginCapabilities
}

// DataSourcePlugin is the interface for data source plugins
type DataSourcePlugin interface {
	// Query executes a query against the data source
	Query(query *DataQuery, ctx *PluginContext) (*DataResult, error)

	// GetSchema returns the schema of the data source
	GetSchema() (map[string]interface{}, error)

	// GetCapabilities returns the capabilities this plugin requires
	GetCapabilities() *PluginCapabilities
}

// ============================================================================
// Plugin Export Functions (WASM exports)
// ============================================================================

var (
	currentAuthPlugin       AuthPlugin
	currentTemplatePlugin   TemplatePlugin
	currentResponsePlugin   ResponsePlugin
	currentDataSourcePlugin DataSourcePlugin
)

// ExportAuthPlugin registers an authentication plugin for export to WASM
func ExportAuthPlugin(plugin AuthPlugin) {
	currentAuthPlugin = plugin
}

// ExportTemplatePlugin registers a template plugin for export to WASM
func ExportTemplatePlugin(plugin TemplatePlugin) {
	currentTemplatePlugin = plugin
}

// ExportResponsePlugin registers a response plugin for export to WASM
func ExportResponsePlugin(plugin ResponsePlugin) {
	currentResponsePlugin = plugin
}

// ExportDataSourcePlugin registers a data source plugin for export to WASM
func ExportDataSourcePlugin(plugin DataSourcePlugin) {
	currentDataSourcePlugin = plugin
}

// ============================================================================
// WASM Export Functions
// These are called by the MockForge runtime
// ============================================================================

//export plugin_auth_authenticate
func plugin_auth_authenticate(contextPtr, contextLen, credsPtr, credsLen uint32) uint32 {
	if currentAuthPlugin == nil {
		return encodeError(&PluginError{Message: "no auth plugin registered", Code: 500})
	}

	// Decode inputs from WASM memory
	contextBytes := readMemory(contextPtr, contextLen)
	credsBytes := readMemory(credsPtr, credsLen)

	var ctx PluginContext
	var creds AuthCredentials

	if err := json.Unmarshal(contextBytes, &ctx); err != nil {
		return encodeError(&PluginError{Message: fmt.Sprintf("failed to decode context: %v", err), Code: 400})
	}

	if err := json.Unmarshal(credsBytes, &creds); err != nil {
		return encodeError(&PluginError{Message: fmt.Sprintf("failed to decode credentials: %v", err), Code: 400})
	}

	// Call the plugin
	result, err := currentAuthPlugin.Authenticate(&ctx, &creds)
	if err != nil {
		return encodeError(&PluginError{Message: err.Error(), Code: 500})
	}

	// Encode result
	return encodeResult(result)
}

//export plugin_auth_capabilities
func plugin_auth_capabilities() uint32 {
	if currentAuthPlugin == nil {
		return encodeError(&PluginError{Message: "no auth plugin registered", Code: 500})
	}

	caps := currentAuthPlugin.GetCapabilities()
	return encodeResult(caps)
}

//export plugin_template_execute
func plugin_template_execute(namePtr, nameLen, argsPtr, argsLen, ctxPtr, ctxLen uint32) uint32 {
	if currentTemplatePlugin == nil {
		return encodeError(&PluginError{Message: "no template plugin registered", Code: 500})
	}

	// Decode inputs
	name := string(readMemory(namePtr, nameLen))
	argsBytes := readMemory(argsPtr, argsLen)
	ctxBytes := readMemory(ctxPtr, ctxLen)

	var args []interface{}
	var ctx ResolutionContext

	if err := json.Unmarshal(argsBytes, &args); err != nil {
		return encodeError(&PluginError{Message: fmt.Sprintf("failed to decode args: %v", err), Code: 400})
	}

	if err := json.Unmarshal(ctxBytes, &ctx); err != nil {
		return encodeError(&PluginError{Message: fmt.Sprintf("failed to decode context: %v", err), Code: 400})
	}

	// Call the plugin
	result, err := currentTemplatePlugin.ExecuteFunction(name, args, &ctx)
	if err != nil {
		return encodeError(&PluginError{Message: err.Error(), Code: 500})
	}

	return encodeResult(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

// readMemory reads bytes from WASM linear memory
// This is a placeholder - actual implementation depends on TinyGo's memory model
func readMemory(ptr, length uint32) []byte {
	// In real implementation, this would read from WASM linear memory
	// For now, returning empty slice as placeholder
	return make([]byte, length)
}

// writeMemory writes bytes to WASM linear memory
func writeMemory(data []byte) uint32 {
	// In real implementation, this would:
	// 1. Allocate memory in WASM linear memory
	// 2. Write data to that memory
	// 3. Return pointer to the data
	// For now, returning 0 as placeholder
	return 0
}

// encodeResult encodes a result as JSON and returns pointer to it
func encodeResult(result interface{}) uint32 {
	data, err := json.Marshal(result)
	if err != nil {
		return encodeError(&PluginError{Message: fmt.Sprintf("failed to encode result: %v", err), Code: 500})
	}
	return writeMemory(data)
}

// encodeError encodes an error and returns pointer to it
func encodeError(err *PluginError) uint32 {
	data, _ := json.Marshal(err)
	return writeMemory(data)
}
