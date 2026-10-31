package mockforge

import (
	"fmt"
	"net"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
)

func TestManagementRedirectsAreRefused(t *testing.T) {
	for _, status := range []int{301, 302, 303, 307, 308} {
		t.Run(fmt.Sprint(status), func(t *testing.T) {
			var forwarded atomic.Int32
			sink := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				forwarded.Add(1)
				w.WriteHeader(http.StatusOK)
			}))
			defer sink.Close()
			origin := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				if r.Header.Get(ManagementTokenHeader) != "synthetic-redirect-only" {
					t.Error("original request lacks synthetic credential")
				}
				http.Redirect(w, r, sink.URL, status)
			}))
			defer origin.Close()
			server := NewMockServer(MockServerConfig{ManagementToken: "synthetic-redirect-only"})
			address := origin.Listener.Addr().(*net.TCPAddr)
			server.host, server.port = address.IP.String(), address.Port
			if response, err := server.managementRequest(http.MethodPost, "/mocks", nil); err == nil {
				response.Body.Close()
				t.Error("management redirect must fail")
			}
			response, err := server.controlRequest(http.MethodPost, origin.URL+"/api/verification/verify", nil)
			if err != nil {
				t.Fatal(err)
			}
			response.Body.Close()
			if response.StatusCode != status {
				t.Error("control transport followed redirect")
			}
			if forwarded.Load() != 0 {
				t.Error("management request reached a redirect target")
			}
		})
	}
}
