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

// Fixture-related tests
// These tests verify fixture recording and replay functionality

func TestMockServerListFixtures(t *testing.T) {
	t.Skip("Requires MockForge CLI to be installed and fixtures to exist")

	server := NewMockServer(MockServerConfig{Port: 3458})
	err := server.Start()
	if err != nil {
		t.Fatalf("Failed to start server: %v", err)
	}
	defer server.Stop()

	// List fixtures via admin API
	fixtures, err := server.ListFixtures()
	if err != nil {
		t.Errorf("Failed to list fixtures: %v", err)
	}

	// Verify we got a response (may be empty if no fixtures)
	if fixtures == nil {
		t.Error("Expected fixtures list, got nil")
	}
}

func TestMockServerFixtureOperations(t *testing.T) {
	t.Skip("Requires MockForge CLI to be installed")

	server := NewMockServer(MockServerConfig{Port: 3459})
	err := server.Start()
	if err != nil {
		t.Fatalf("Failed to start server: %v", err)
	}
	defer server.Stop()

	// Test listing fixtures (should work even if empty)
	fixtures, err := server.ListFixtures()
	if err != nil {
		t.Errorf("Failed to list fixtures: %v", err)
	}

	// If we have fixtures, test other operations
	if len(fixtures) > 0 {
		// Test getting fixture info
		fixtureID := fixtures[0].ID
		if fixtureID != "" {
			// Test downloading fixture
			data, err := server.DownloadFixture(fixtureID)
			if err != nil {
				t.Errorf("Failed to download fixture: %v", err)
			}
			if data == nil {
				t.Error("Expected fixture data, got nil")
			}
		}
	}
}
