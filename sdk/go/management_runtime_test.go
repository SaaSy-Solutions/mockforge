package mockforge

import (
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"testing"
	"time"
)

func TestDynamicMockRuntime(t *testing.T) {
	if _, err := exec.LookPath("mockforge"); err != nil {
		t.Skip("requires MockForge CLI on PATH")
	}
	server := NewMockServer(MockServerConfig{Port: 0})
	if err := server.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := server.Stop(); err != nil {
			t.Error(err)
		}
	})
	if err := server.StubResponseWithOptions("GET", "/sdk1281-go", map[string]interface{}{"sdk": "go"}, 202, map[string]string{"X-SDK-Control": "go"}, nil); err != nil {
		t.Fatal(err)
	}
	client := &http.Client{Timeout: 5 * time.Second}
	response, err := client.Get(server.URL() + "/sdk1281-go")
	if err != nil {
		t.Fatal(err)
	}
	var body map[string]interface{}
	err = json.NewDecoder(response.Body).Decode(&body)
	response.Body.Close()
	if err != nil {
		t.Fatal(err)
	}
	if response.StatusCode != 202 || body["sdk"] != "go" || response.Header.Get("X-SDK-Control") != "go" {
		t.Fatalf("unexpected stub response: status=%d body=%v", response.StatusCode, body)
	}
	if err := server.ClearStubs(); err != nil {
		t.Fatal(err)
	}
	response, err = client.Get(server.URL() + "/sdk1281-go")
	if err != nil {
		t.Fatal(err)
	}
	response.Body.Close()
	if response.StatusCode != 404 {
		t.Fatalf("cleared stub returned HTTP %d", response.StatusCode)
	}
}

func TestManagementFailureIsReported(t *testing.T) {
	expected := "fixture-only-management-credential-1281"
	receiver := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get(ManagementTokenHeader) != expected {
			t.Error("management credential missing from CRUD request")
		}
		if r.URL.Path != "/__mockforge/api/mocks" {
			t.Errorf("unexpected management path: %s", r.URL.Path)
		}
		w.WriteHeader(http.StatusServiceUnavailable)
	}))
	defer receiver.Close()
	server := NewMockServer(MockServerConfig{ManagementToken: expected})
	// Direct this client at a real HTTP server that rejects management writes.
	server.host = receiver.Listener.Addr().(*net.TCPAddr).IP.String()
	server.port = receiver.Listener.Addr().(*net.TCPAddr).Port
	if err := server.StubResponse("GET", "/not-created", nil); err == nil {
		t.Fatal("failed creation must return an error")
	}
	if len(server.stubs) != 0 {
		t.Fatal("failed creation must not be recorded as successful")
	}
	if err := server.ClearStubs(); err == nil {
		t.Fatal("failed list must return an error")
	}
}
