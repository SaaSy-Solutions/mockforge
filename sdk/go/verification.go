// Package mockforge provides verification types and helpers
package mockforge

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
)

// VerificationRequest represents a pattern for matching requests during verification
type VerificationRequest struct {
	// HTTP method to match (e.g., "GET", "POST"). Case-insensitive. If empty, matches any method.
	Method string `json:"method,omitempty"`
	// URL path to match. Supports exact match, wildcards (*, **), and regex. If empty, matches any path.
	Path string `json:"path,omitempty"`
	// Query parameters to match (all must be present and match). If empty, query parameters are not checked.
	QueryParams map[string]string `json:"query_params,omitempty"`
	// Headers to match (all must be present and match). Case-insensitive header names. If empty, headers are not checked.
	Headers map[string]string `json:"headers,omitempty"`
	// Request body pattern to match. Supports exact match or regex. If empty, body is not checked.
	BodyPattern string `json:"body_pattern,omitempty"`
}

// VerificationCount represents a count assertion for verification
type VerificationCount struct {
	Type  string `json:"type"`
	Value *int   `json:"value,omitempty"`
}

// VerificationCount helpers
func Exactly(n int) VerificationCount {
	return VerificationCount{Type: "exactly", Value: &n}
}

func AtLeast(n int) VerificationCount {
	return VerificationCount{Type: "at_least", Value: &n}
}

func AtMost(n int) VerificationCount {
	return VerificationCount{Type: "at_most", Value: &n}
}

func Never() VerificationCount {
	return VerificationCount{Type: "never"}
}

func AtLeastOnce() VerificationCount {
	return VerificationCount{Type: "at_least_once"}
}

// VerificationResult represents the result of a verification operation
type VerificationResult struct {
	// Whether the verification passed
	Matched bool `json:"matched"`
	// Actual count of matching requests
	Count int `json:"count"`
	// Expected count assertion
	Expected VerificationCount `json:"expected"`
	// All matching request log entries (for inspection)
	Matches []map[string]interface{} `json:"matches"`
	// Error message if verification failed
	ErrorMessage *string `json:"error_message,omitempty"`
}

// Verify verifies requests against a pattern and count assertion
func (m *MockServer) Verify(pattern VerificationRequest, expected VerificationCount) (*VerificationResult, error) {
	requestBody := map[string]interface{}{
		"pattern":  pattern,
		"expected": expected,
	}

	jsonData, err := json.Marshal(requestBody)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	resp, err := http.Post(
		fmt.Sprintf("%s/api/verification/verify", m.URL()),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return nil, fmt.Errorf("verification request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusExpectationFailed {
		return nil, fmt.Errorf("verification request failed with status: %d", resp.StatusCode)
	}

	var result VerificationResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// VerifyNever verifies that a request was never made
func (m *MockServer) VerifyNever(pattern VerificationRequest) (*VerificationResult, error) {
	jsonData, err := json.Marshal(pattern)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	resp, err := http.Post(
		fmt.Sprintf("%s/api/verification/never", m.URL()),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return nil, fmt.Errorf("verification request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusExpectationFailed {
		return nil, fmt.Errorf("verification request failed with status: %d", resp.StatusCode)
	}

	var result VerificationResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// VerifyAtLeast verifies that a request was made at least N times
func (m *MockServer) VerifyAtLeast(pattern VerificationRequest, min int) (*VerificationResult, error) {
	requestBody := map[string]interface{}{
		"pattern": pattern,
		"min":     min,
	}

	jsonData, err := json.Marshal(requestBody)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	resp, err := http.Post(
		fmt.Sprintf("%s/api/verification/at-least", m.URL()),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return nil, fmt.Errorf("verification request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusExpectationFailed {
		return nil, fmt.Errorf("verification request failed with status: %d", resp.StatusCode)
	}

	var result VerificationResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// VerifySequence verifies that requests occurred in a specific sequence
func (m *MockServer) VerifySequence(patterns []VerificationRequest) (*VerificationResult, error) {
	requestBody := map[string]interface{}{
		"patterns": patterns,
	}

	jsonData, err := json.Marshal(requestBody)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	resp, err := http.Post(
		fmt.Sprintf("%s/api/verification/sequence", m.URL()),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return nil, fmt.Errorf("verification request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusExpectationFailed {
		return nil, fmt.Errorf("verification request failed with status: %d", resp.StatusCode)
	}

	var result VerificationResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// CountRequests gets the count of matching requests
func (m *MockServer) CountRequests(pattern VerificationRequest) (int, error) {
	requestBody := map[string]interface{}{
		"pattern": pattern,
	}

	jsonData, err := json.Marshal(requestBody)
	if err != nil {
		return 0, fmt.Errorf("failed to marshal request: %w", err)
	}

	resp, err := http.Post(
		fmt.Sprintf("%s/api/verification/count", m.URL()),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return 0, fmt.Errorf("verification request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return 0, fmt.Errorf("verification request failed with status: %d", resp.StatusCode)
	}

	var result struct {
		Count int `json:"count"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, fmt.Errorf("failed to decode response: %w", err)
	}

	return result.Count, nil
}
