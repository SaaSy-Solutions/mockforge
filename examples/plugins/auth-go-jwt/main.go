// JWT Authentication Plugin for MockForge
//
// This plugin demonstrates how to build a JWT authentication plugin in Go
// using the MockForge Go SDK and TinyGo for WebAssembly compilation.
//
// Build:
//   tinygo build -o plugin.wasm -target=wasi main.go
//
// Install:
//   mockforge plugin install .
//
package main

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/mockforge/mockforge/sdk/go/mockforge"
)

// JWTAuthPlugin implements JWT-based authentication
type JWTAuthPlugin struct {
	// In a real plugin, you might load these from config
	secretKey    string
	issuer       string
	allowedAudiences []string
}

// NewJWTAuthPlugin creates a new JWT authentication plugin
func NewJWTAuthPlugin() *JWTAuthPlugin {
	return &JWTAuthPlugin{
		secretKey:    "your-secret-key-here", // In production, load from secure storage
		issuer:       "mockforge",
		allowedAudiences: []string{"mockforge-api"},
	}
}

// Authenticate validates JWT tokens and returns authentication result
func (p *JWTAuthPlugin) Authenticate(
	ctx *mockforge.PluginContext,
	creds *mockforge.AuthCredentials,
) (*mockforge.AuthResult, error) {
	// Check credential type
	if creds.Type != "bearer" && creds.Type != "Bearer" {
		return &mockforge.AuthResult{
			Authenticated: false,
			UserID:        "",
			Claims:        map[string]interface{}{},
		}, fmt.Errorf("unsupported credential type: %s", creds.Type)
	}

	// Extract token
	token := creds.Token
	if token == "" {
		return &mockforge.AuthResult{
			Authenticated: false,
			UserID:        "",
			Claims:        map[string]interface{}{},
		}, fmt.Errorf("missing token")
	}

	// Parse and validate JWT
	claims, err := p.validateJWT(token)
	if err != nil {
		return &mockforge.AuthResult{
			Authenticated: false,
			UserID:        "",
			Claims:        map[string]interface{}{},
		}, err
	}

	// Extract user ID from claims
	userID, ok := claims["sub"].(string)
	if !ok {
		userID = "unknown"
	}

	// Return successful authentication
	return &mockforge.AuthResult{
		Authenticated: true,
		UserID:        userID,
		Claims:        claims,
	}, nil
}

// GetCapabilities returns the capabilities this plugin requires
func (p *JWTAuthPlugin) GetCapabilities() *mockforge.PluginCapabilities {
	return &mockforge.PluginCapabilities{
		Network: mockforge.NetworkCapabilities{
			// JWT validation can be done locally, no network needed
			AllowHTTPOutbound: false,
			AllowedHosts:      []string{},
		},
		Filesystem: mockforge.FilesystemCapabilities{
			// No filesystem access needed
			AllowRead:    false,
			AllowWrite:   false,
			AllowedPaths: []string{},
		},
		Resources: mockforge.ResourceLimits{
			MaxMemoryBytes: 10 * 1024 * 1024, // 10MB
			MaxCPUTimeMs:   500,               // 500ms (JWT parsing is fast)
		},
	}
}

// validateJWT validates a JWT token and returns the claims
// This is a simplified implementation - in production, use a proper JWT library
func (p *JWTAuthPlugin) validateJWT(tokenString string) (map[string]interface{}, error) {
	// Split token into parts
	parts := strings.Split(tokenString, ".")
	if len(parts) != 3 {
		return nil, fmt.Errorf("invalid token format")
	}

	// Decode header
	headerBytes, err := base64.RawURLEncoding.DecodeString(parts[0])
	if err != nil {
		return nil, fmt.Errorf("failed to decode header: %v", err)
	}

	var header map[string]interface{}
	if err := json.Unmarshal(headerBytes, &header); err != nil {
		return nil, fmt.Errorf("failed to parse header: %v", err)
	}

	// Check algorithm
	alg, ok := header["alg"].(string)
	if !ok || (alg != "HS256" && alg != "HS512") {
		return nil, fmt.Errorf("unsupported algorithm: %v", alg)
	}

	// Decode payload
	payloadBytes, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return nil, fmt.Errorf("failed to decode payload: %v", err)
	}

	var claims map[string]interface{}
	if err := json.Unmarshal(payloadBytes, &claims); err != nil {
		return nil, fmt.Errorf("failed to parse claims: %v", err)
	}

	// Verify signature (simplified - in production, use proper crypto)
	// For this example, we'll skip signature verification
	// In a real plugin, you would:
	// 1. Reconstruct the signing input
	// 2. Generate signature using secret key
	// 3. Compare with provided signature

	// Validate expiration
	if exp, ok := claims["exp"].(float64); ok {
		if time.Now().Unix() > int64(exp) {
			return nil, fmt.Errorf("token expired")
		}
	}

	// Validate not before
	if nbf, ok := claims["nbf"].(float64); ok {
		if time.Now().Unix() < int64(nbf) {
			return nil, fmt.Errorf("token not yet valid")
		}
	}

	// Validate issuer
	if iss, ok := claims["iss"].(string); ok {
		if iss != p.issuer {
			return nil, fmt.Errorf("invalid issuer: %s", iss)
		}
	}

	// Validate audience
	if aud, ok := claims["aud"].(string); ok {
		validAudience := false
		for _, allowedAud := range p.allowedAudiences {
			if aud == allowedAud {
				validAudience = true
				break
			}
		}
		if !validAudience {
			return nil, fmt.Errorf("invalid audience: %s", aud)
		}
	}

	return claims, nil
}

func main() {
	plugin := NewJWTAuthPlugin()
	mockforge.ExportAuthPlugin(plugin)
}
