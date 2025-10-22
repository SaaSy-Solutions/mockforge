package mockforge

import (
	"testing"
)

func TestNewMockServer(t *testing.T) {
	t.Run("creates server with default config", func(t *testing.T) {
		server := NewMockServer(MockServerConfig{})
		if server == nil {
			t.Fatal("Expected server to be created")
		}
		if server.host != "127.0.0.1" {
			t.Errorf("Expected host to be 127.0.0.1, got %s", server.host)
		}
	})

	t.Run("creates server with custom port", func(t *testing.T) {
		server := NewMockServer(MockServerConfig{Port: 3000})
		if server.port != 3000 {
			t.Errorf("Expected port to be 3000, got %d", server.port)
		}
	})

	t.Run("creates server with custom host", func(t *testing.T) {
		server := NewMockServer(MockServerConfig{Host: "0.0.0.0"})
		if server.host != "0.0.0.0" {
			t.Errorf("Expected host to be 0.0.0.0, got %s", server.host)
		}
	})
}

func TestMockServerURL(t *testing.T) {
	server := NewMockServer(MockServerConfig{Port: 3000, Host: "127.0.0.1"})
	expected := "http://127.0.0.1:3000"
	if server.URL() != expected {
		t.Errorf("Expected URL to be %s, got %s", expected, server.URL())
	}
}

func TestMockServerIsRunning(t *testing.T) {
	server := NewMockServer(MockServerConfig{})
	if server.IsRunning() {
		t.Error("Expected server to not be running before start")
	}
}

// Integration tests that require MockForge CLI
// Run with: go test -tags=integration
//go:build integration
// +build integration

func TestMockServerStart(t *testing.T) {
	t.Skip("Requires MockForge CLI to be installed")

	server := NewMockServer(MockServerConfig{Port: 3456})
	err := server.Start()
	if err != nil {
		t.Fatalf("Failed to start server: %v", err)
	}
	defer server.Stop()

	if !server.IsRunning() {
		t.Error("Expected server to be running after start")
	}
}

func TestMockServerStubResponse(t *testing.T) {
	t.Skip("Requires MockForge CLI to be installed")

	server := NewMockServer(MockServerConfig{Port: 3457})
	err := server.Start()
	if err != nil {
		t.Fatalf("Failed to start server: %v", err)
	}
	defer server.Stop()

	err = server.StubResponse("GET", "/test", map[string]interface{}{
		"message": "hello",
	})
	if err != nil {
		t.Errorf("Failed to stub response: %v", err)
	}
}
