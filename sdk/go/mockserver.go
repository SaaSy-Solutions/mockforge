// Package mockforge provides an SDK for embedding MockForge mock servers in Go tests
package mockforge

import (
	"bufio"
	"bytes"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"sync"
	"time"
)

// MockServerConfig holds the configuration for a mock server
type MockServerConfig struct {
	Port        int
	Host        string
	ConfigFile  string
	OpenAPISpec string
	// ManagementToken is the control-plane credential. It is passed to the
	// CLI as MOCKFORGE_MANAGEMENT_TOKEN and sent as
	// X-MockForge-Management-Token on every SDK control-plane request.
	// Default: $MOCKFORGE_MANAGEMENT_TOKEN, else a random 64-char hex token.
	ManagementToken string
}

// ManagementTokenHeader carries the management credential on control-plane requests.
const ManagementTokenHeader = "X-MockForge-Management-Token"

// resolveManagementToken returns the explicit token, else the
// MOCKFORGE_MANAGEMENT_TOKEN env var, else a fresh random hex token. It
// returns "" only if the system CSPRNG fails; Start reports that as an error.
func resolveManagementToken(explicit string) string {
	if explicit != "" {
		return explicit
	}
	if fromEnv := os.Getenv("MOCKFORGE_MANAGEMENT_TOKEN"); fromEnv != "" {
		return fromEnv
	}
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return ""
	}
	return hex.EncodeToString(buf)
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
	// managementToken is sent on every control-plane request.
	managementToken string
	portMutex       sync.RWMutex // Protects port and adminPort during detection
}

// NewMockServer creates a new mock server with the given configuration
func NewMockServer(config MockServerConfig) *MockServer {
	if config.Host == "" {
		config.Host = "127.0.0.1"
	}

	return &MockServer{
		config:          config,
		port:            config.Port,
		host:            config.Host,
		stubs:           make([]ResponseStub, 0),
		managementToken: resolveManagementToken(config.ManagementToken),
	}
}

// ManagementToken returns the control-plane credential this server uses.
func (m *MockServer) ManagementToken() string {
	return m.managementToken
}

// controlRequest performs a control-plane request carrying the management token.
func (m *MockServer) controlRequest(method, url string, body []byte) (*http.Response, error) {
	var reader io.Reader
	if body != nil {
		reader = bytes.NewReader(body)
	}
	req, err := http.NewRequest(method, url, reader)
	if err != nil {
		return nil, err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	req.Header.Set(ManagementTokenHeader, m.managementToken)
	return http.DefaultClient.Do(req)
}

// Start starts the mock server
func (m *MockServer) Start() error {
	if m.managementToken == "" {
		return fmt.Errorf("mockforge: could not generate a management token")
	}

	args := []string{"serve"}

	if m.config.ConfigFile != "" {
		args = append(args, "--config", m.config.ConfigFile)
	}

	if m.config.OpenAPISpec != "" {
		args = append(args, "--spec", m.config.OpenAPISpec)
	}

	if m.port != 0 {
		args = append(args, "--http-port", fmt.Sprintf("%d", m.port))
	} else {
		// Use port 0 to let OS assign a random port
		args = append(args, "--http-port", "0")
	}

	// Enable admin API for dynamic stub management
	args = append(args, "--admin", "--admin-port", "0")

	m.cmd = exec.Command("mockforge", args...)
	m.cmd.Env = append(os.Environ(), "MOCKFORGE_MANAGEMENT_TOKEN="+m.managementToken)

	// Capture stdout and stderr for port detection
	stdoutPipe, err := m.cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderrPipe, err := m.cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := m.cmd.Start(); err != nil {
		return NewCLINotFoundError(err)
	}

	// Start goroutine to parse stdout for port information
	go m.parsePortsFromOutput(stdoutPipe)

	// Start goroutine to read stderr (for error messages)
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			// Log stderr but don't fail - wait for health check
			// Could be enhanced to surface errors to user
		}
	}()

	// Wait for server to be ready
	if err := m.waitForServer(); err != nil {
		m.cmd.Process.Kill()
		m.cmd.Wait() // Clean up zombie process
		m.cmd = nil  // Clear cmd so IsRunning() returns false
		return err
	}

	return nil
}

// parsePortsFromOutput parses port numbers from MockForge CLI output
func (m *MockServer) parsePortsFromOutput(stdout io.Reader) {
	scanner := bufio.NewScanner(stdout)

	// Patterns to match:
	// - "📡 HTTP server listening on http://localhost:PORT"
	// - "📡 HTTP server on port PORT"
	httpPortPattern := regexp.MustCompile(`HTTP server (?:listening on http://[^:]+:|on port )(\d+)`)

	// - "🎛️ Admin UI listening on http://HOST:PORT"
	// - "🎛️ Admin UI on port PORT"
	adminPortPattern := regexp.MustCompile(`Admin UI (?:listening on http://[^:]+:|on port )(\d+)`)

	for scanner.Scan() {
		line := scanner.Text()

		// Parse HTTP server port
		if matches := httpPortPattern.FindStringSubmatch(line); matches != nil {
			m.portMutex.Lock()
			if m.port == 0 {
				if port, err := strconv.Atoi(matches[1]); err == nil && port > 0 {
					m.port = port
				}
			}
			m.portMutex.Unlock()
		}

		// Parse Admin UI port
		if matches := adminPortPattern.FindStringSubmatch(line); matches != nil {
			m.portMutex.Lock()
			if m.adminPort == 0 {
				if port, err := strconv.Atoi(matches[1]); err == nil && port > 0 {
					m.adminPort = port
				}
			}
			m.portMutex.Unlock()
		}
	}
}

// waitForServer waits for the server to be ready
func (m *MockServer) waitForServer() error {
	timeout := time.After(12 * time.Second)
	ticker := time.NewTicker(200 * time.Millisecond)
	defer ticker.Stop()

	portDetectionAttempts := 0
	maxPortDetectionAttempts := 20

	for {
		select {
		case <-timeout:
			m.portMutex.RLock()
			port := m.port
			m.portMutex.RUnlock()
			if port == 0 {
				return NewPortDetectionFailedError(nil)
			}
			return NewHealthCheckTimeoutError(12000, port)
		case <-ticker.C:
			m.portMutex.RLock()
			port := m.port
			m.portMutex.RUnlock()

			// If port is 0, wait for it to be detected from stdout
			if port == 0 && portDetectionAttempts < maxPortDetectionAttempts {
				portDetectionAttempts++
				continue
			}

			// If port is still 0 after detection attempts, return standardized error
			if port == 0 {
				return NewPortDetectionFailedError(nil)
			}

			resp, err := http.Get(fmt.Sprintf("http://%s:%d/health", m.host, port))
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
		// Convert ResponseStub to MockConfig format expected by Admin API
		mockConfig := map[string]interface{}{
			"id":     "",                                           // Empty ID - server will generate one
			"name":   fmt.Sprintf("%s %s", stub.Method, stub.Path), // Generate a name from method and path
			"method": stub.Method,
			"path":   stub.Path,
			"response": map[string]interface{}{
				"body": stub.Body,
			},
			"enabled": true,
		}

		// Add optional fields only if they have values
		if len(stub.Headers) > 0 {
			response := mockConfig["response"].(map[string]interface{})
			response["headers"] = stub.Headers
		}
		if stub.LatencyMs != nil {
			mockConfig["latency_ms"] = *stub.LatencyMs
		}
		if stub.Status != 200 {
			mockConfig["status_code"] = stub.Status
		}

		mockConfigJSON, err := json.Marshal(mockConfig)
		if err != nil {
			return err
		}

		resp, err := m.controlRequest(
			http.MethodPost,
			fmt.Sprintf("http://%s:%d/__mockforge/api/mocks", m.host, m.adminPort),
			mockConfigJSON,
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
		// Get all mocks and delete them one by one
		resp, err := m.controlRequest(
			http.MethodGet,
			fmt.Sprintf("http://%s:%d/__mockforge/api/mocks", m.host, m.adminPort),
			nil,
		)
		if err == nil {
			var result struct {
				Mocks []struct {
					ID string `json:"id"`
				} `json:"mocks"`
			}
			if json.NewDecoder(resp.Body).Decode(&result) == nil {
				resp.Body.Close()
				// Delete each mock
				for _, mock := range result.Mocks {
					deleteResp, err := m.controlRequest(
						http.MethodDelete,
						fmt.Sprintf("http://%s:%d/__mockforge/api/mocks/%s", m.host, m.adminPort, mock.ID),
						nil,
					)
					if err == nil {
						deleteResp.Body.Close()
					}
				}
			} else {
				resp.Body.Close()
			}
		}
	}

	return nil
}

// URL returns the server URL
func (m *MockServer) URL() string {
	m.portMutex.RLock()
	defer m.portMutex.RUnlock()
	return fmt.Sprintf("http://%s:%d", m.host, m.port)
}

// Port returns the server port
func (m *MockServer) Port() int {
	m.portMutex.RLock()
	defer m.portMutex.RUnlock()
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

// FixtureInfo represents fixture metadata
type FixtureInfo struct {
	ID       string                 `json:"id"`
	Protocol string                 `json:"protocol"`
	Method   string                 `json:"method"`
	Path     string                 `json:"path"`
	SavedAt  string                 `json:"saved_at"`
	FileSize int64                  `json:"file_size"`
	FilePath string                 `json:"file_path"`
	Metadata map[string]interface{} `json:"metadata"`
}

// ListFixtures lists all available fixtures
func (m *MockServer) ListFixtures() ([]FixtureInfo, error) {
	m.portMutex.RLock()
	adminPort := m.adminPort
	host := m.host
	m.portMutex.RUnlock()

	if adminPort == 0 {
		return nil, fmt.Errorf("admin port not available")
	}

	resp, err := m.controlRequest(
		http.MethodGet,
		fmt.Sprintf("http://%s:%d/__mockforge/fixtures", host, adminPort),
		nil,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to list fixtures: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to list fixtures: status %d", resp.StatusCode)
	}

	var result struct {
		Data []FixtureInfo `json:"data"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode fixtures response: %w", err)
	}

	return result.Data, nil
}

// DownloadFixture downloads a fixture by ID
func (m *MockServer) DownloadFixture(fixtureID string) ([]byte, error) {
	m.portMutex.RLock()
	adminPort := m.adminPort
	host := m.host
	m.portMutex.RUnlock()

	if adminPort == 0 {
		return nil, fmt.Errorf("admin port not available")
	}

	resp, err := m.controlRequest(
		http.MethodGet,
		fmt.Sprintf("http://%s:%d/__mockforge/fixtures/%s/download", host, adminPort, fixtureID),
		nil,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to download fixture: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to download fixture: status %d", resp.StatusCode)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read fixture data: %w", err)
	}

	return data, nil
}
