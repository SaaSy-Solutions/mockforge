// Package mockforge provides an SDK for embedding MockForge mock servers in Go tests
package mockforge

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os/exec"
	"time"
)

// MockServerConfig holds the configuration for a mock server
type MockServerConfig struct {
	Port         int
	Host         string
	ConfigFile   string
	OpenAPISpec  string
}

// ResponseStub represents a stubbed HTTP response
type ResponseStub struct {
	Method    string            `json:"method"`
	Path      string            `json:"path"`
	Status    int               `json:"status"`
	Headers   map[string]string `json:"headers"`
	Body      interface{}       `json:"body"`
	LatencyMs *int              `json:"latency_ms,omitempty"`
}

// MockServer represents an embedded mock server
type MockServer struct {
	config    MockServerConfig
	cmd       *exec.Cmd
	port      int
	host      string
	adminPort int
	stubs     []ResponseStub
}

// NewMockServer creates a new mock server with the given configuration
func NewMockServer(config MockServerConfig) *MockServer {
	if config.Host == "" {
		config.Host = "127.0.0.1"
	}

	return &MockServer{
		config: config,
		port:   config.Port,
		host:   config.Host,
		stubs:  make([]ResponseStub, 0),
	}
}

// Start starts the mock server
func (m *MockServer) Start() error {
	args := []string{"serve"}

	if m.config.ConfigFile != "" {
		args = append(args, "--config", m.config.ConfigFile)
	}

	if m.config.OpenAPISpec != "" {
		args = append(args, "--spec", m.config.OpenAPISpec)
	}

	if m.port != 0 {
		args = append(args, "--http-port", fmt.Sprintf("%d", m.port))
	}

	// Enable admin API for dynamic stub management
	args = append(args, "--admin", "--admin-port", "0")

	m.cmd = exec.Command("mockforge", args...)

	if err := m.cmd.Start(); err != nil {
		return fmt.Errorf("failed to start mockforge: %w", err)
	}

	// Wait for server to be ready
	if err := m.waitForServer(); err != nil {
		m.cmd.Process.Kill()
		m.cmd.Wait() // Clean up zombie process
		m.cmd = nil  // Clear cmd so IsRunning() returns false
		return err
	}

	return nil
}

// waitForServer waits for the server to be ready
func (m *MockServer) waitForServer() error {
	timeout := time.After(10 * time.Second)
	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-timeout:
			return fmt.Errorf("timeout waiting for server to start")
		case <-ticker.C:
			resp, err := http.Get(fmt.Sprintf("http://%s:%d/health", m.host, m.port))
			if err == nil && resp.StatusCode == 200 {
				resp.Body.Close()
				return nil
			}
		}
	}
}

// StubResponse adds a stubbed response
func (m *MockServer) StubResponse(method, path string, body interface{}) error {
	return m.StubResponseWithOptions(method, path, body, 200, nil, nil)
}

// StubResponseWithOptions adds a stubbed response with options
func (m *MockServer) StubResponseWithOptions(
	method, path string,
	body interface{},
	status int,
	headers map[string]string,
	latencyMs *int,
) error {
	if headers == nil {
		headers = make(map[string]string)
	}

	stub := ResponseStub{
		Method:    method,
		Path:      path,
		Status:    status,
		Headers:   headers,
		Body:      body,
		LatencyMs: latencyMs,
	}

	m.stubs = append(m.stubs, stub)

	// If admin API is available, use it to add the stub dynamically
	if m.adminPort != 0 {
		stubJSON, err := json.Marshal(stub)
		if err != nil {
			return err
		}

		resp, err := http.Post(
			fmt.Sprintf("http://%s:%d/api/stubs", m.host, m.adminPort),
			"application/json",
			bytes.NewBuffer(stubJSON),
		)
		if err == nil {
			resp.Body.Close()
		}
	}

	return nil
}

// ClearStubs removes all stubs
func (m *MockServer) ClearStubs() error {
	m.stubs = make([]ResponseStub, 0)

	if m.adminPort != 0 {
		req, err := http.NewRequest(
			"DELETE",
			fmt.Sprintf("http://%s:%d/api/stubs", m.host, m.adminPort),
			nil,
		)
		if err != nil {
			return err
		}

		resp, err := http.DefaultClient.Do(req)
		if err == nil {
			resp.Body.Close()
		}
	}

	return nil
}

// URL returns the server URL
func (m *MockServer) URL() string {
	return fmt.Sprintf("http://%s:%d", m.host, m.port)
}

// Port returns the server port
func (m *MockServer) Port() int {
	return m.port
}

// IsRunning checks if the server is running
func (m *MockServer) IsRunning() bool {
	return m.cmd != nil && m.cmd.Process != nil
}

// Stop stops the mock server
func (m *MockServer) Stop() error {
	if m.cmd != nil && m.cmd.Process != nil {
		if err := m.cmd.Process.Kill(); err != nil {
			return err
		}
		m.cmd.Wait()
		m.cmd = nil
	}
	return nil
}
