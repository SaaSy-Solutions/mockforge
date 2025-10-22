package mockforge

// StubBuilder provides a fluent interface for creating response stubs
type StubBuilder struct {
	method    string
	path      string
	status    int
	headers   map[string]string
	body      interface{}
	latencyMs *int
}

// NewStubBuilder creates a new StubBuilder
func NewStubBuilder(method, path string) *StubBuilder {
	return &StubBuilder{
		method:  method,
		path:    path,
		status:  200,
		headers: make(map[string]string),
	}
}

// Status sets the HTTP status code
func (b *StubBuilder) Status(code int) *StubBuilder {
	b.status = code
	return b
}

// Header sets a response header
func (b *StubBuilder) Header(key, value string) *StubBuilder {
	b.headers[key] = value
	return b
}

// Headers sets multiple response headers
func (b *StubBuilder) Headers(headers map[string]string) *StubBuilder {
	for k, v := range headers {
		b.headers[k] = v
	}
	return b
}

// Body sets the response body
func (b *StubBuilder) Body(body interface{}) *StubBuilder {
	b.body = body
	return b
}

// Latency sets the response latency in milliseconds
func (b *StubBuilder) Latency(ms int) *StubBuilder {
	b.latencyMs = &ms
	return b
}

// Build builds the ResponseStub
func (b *StubBuilder) Build() ResponseStub {
	return ResponseStub{
		Method:    b.method,
		Path:      b.path,
		Status:    b.status,
		Headers:   b.headers,
		Body:      b.body,
		LatencyMs: b.latencyMs,
	}
}
