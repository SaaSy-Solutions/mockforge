//go:build integration
// +build integration

// Integration tests that require the MockForge CLI to be installed and on PATH.
// Run with: go test -tags=integration
//
// These were previously appended to mockserver_test.go with the build
// constraint placed mid-file, which is invalid (constraints must precede the
// package clause) and broke `go test ./...` for everyone. They now live in
// their own properly-tagged file; default `go test` skips them at compile time.
package mockforge

import (
	"testing"
)

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

// Fixture-related tests verify fixture recording and replay functionality.

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
