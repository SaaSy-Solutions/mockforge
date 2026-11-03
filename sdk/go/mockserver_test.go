package mockforge

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"strconv"
	"strings"
	"sync"
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

func TestManagementTokenResolution(t *testing.T) {
	t.Run("explicit option wins over env", func(t *testing.T) {
		t.Setenv("MOCKFORGE_MANAGEMENT_TOKEN", strings.Repeat("e", 40))
		server := NewMockServer(MockServerConfig{ManagementToken: strings.Repeat("x", 40)})
		if server.ManagementToken() != strings.Repeat("x", 40) {
			t.Errorf("expected explicit token, got %q", server.ManagementToken())
		}
	})

	t.Run("env used when no option", func(t *testing.T) {
		t.Setenv("MOCKFORGE_MANAGEMENT_TOKEN", strings.Repeat("e", 40))
		server := NewMockServer(MockServerConfig{})
		if server.ManagementToken() != strings.Repeat("e", 40) {
			t.Errorf("expected env token, got %q", server.ManagementToken())
		}
	})

	t.Run("random per instance otherwise", func(t *testing.T) {
		t.Setenv("MOCKFORGE_MANAGEMENT_TOKEN", "")
		a := NewMockServer(MockServerConfig{}).ManagementToken()
		b := NewMockServer(MockServerConfig{}).ManagementToken()
		if len(a) < 32 || a == b {
			t.Errorf("expected distinct random tokens >= 32 chars, got %q and %q", a, b)
		}
	})
}

func TestControlPlaneRequestsCarryManagementToken(t *testing.T) {
	type hit struct{ path, token string }
	var (
		mu   sync.Mutex
		hits []hit
	)
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		hits = append(hits, hit{r.URL.Path, r.Header.Get(ManagementTokenHeader)})
		mu.Unlock()
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"matched":true,"count":0,"mocks":[],"data":[]}`))
	}))
	defer ts.Close()

	u, err := url.Parse(ts.URL)
	if err != nil {
		t.Fatal(err)
	}
	port, err := strconv.Atoi(u.Port())
	if err != nil {
		t.Fatal(err)
	}

	token := strings.Repeat("k", 48)
	server := NewMockServer(MockServerConfig{Port: port, ManagementToken: token})
	server.adminPort = port

	if err := server.StubResponse("GET", "/x", map[string]bool{"ok": true}); err != nil {
		t.Fatal(err)
	}
	if _, err := server.Verify(VerificationRequest{Path: "/x"}, Never()); err != nil {
		t.Fatal(err)
	}
	if _, err := server.ListFixtures(); err != nil {
		t.Fatal(err)
	}

	mu.Lock()
	defer mu.Unlock()
	want := []string{"/__mockforge/api/mocks", "/api/verification/verify", "/__mockforge/fixtures"}
	if len(hits) != len(want) {
		t.Fatalf("expected %d requests, got %d: %+v", len(want), len(hits), hits)
	}
	for i, h := range hits {
		if h.path != want[i] {
			t.Errorf("request %d: expected path %s, got %s", i, want[i], h.path)
		}
		if h.token != token {
			t.Errorf("request %d (%s): expected token header, got %q", i, h.path, h.token)
		}
	}
}
