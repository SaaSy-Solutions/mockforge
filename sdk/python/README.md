# MockForge Python SDK

Embed MockForge mock servers directly in your Python tests.

## Prerequisites

The Python SDK requires the MockForge CLI to be installed and available in your PATH:

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

## Installation

```bash
pip install mockforge-sdk
```

## Usage

### Basic Example

```python
from mockforge_sdk import MockServer
import requests

def test_user_api():
    # Context manager automatically starts/stops server
    with MockServer(port=3000) as server:
        server.stub_response('GET', '/api/users/123', {
            'id': 123,
            'name': 'John Doe',
            'email': 'john@example.com'
        })

        response = requests.get('http://localhost:3000/api/users/123')
        assert response.status_code == 200

        data = response.json()
        assert data['id'] == 123
        assert data['name'] == 'John Doe'
```

### Manual Start/Stop

```python
from mockforge_sdk import MockServer

server = MockServer(port=3000)
server.start()

try:
    server.stub_response('GET', '/api/test', {'status': 'ok'})
    # ... make requests ...
finally:
    server.stop()
```

### With OpenAPI Specification

```python
with MockServer(port=3000, openapi_spec='./openapi.yaml') as server:
    # Routes are auto-generated from OpenAPI spec
    response = requests.get('http://localhost:3000/api/users')
```

### With Custom Configuration

```python
server = MockServer(
    port=3000,
    host='127.0.0.1',
    config_file='./mockforge.yaml'
)
```

## API Reference

### `MockServer(port=0, host='127.0.0.1', config_file=None, openapi_spec=None)`

Creates a mock server instance.

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `port` | `int` | `0` (random) | Port to listen on |
| `host` | `str` | `127.0.0.1` | Host to bind to |
| `config_file` | `str` | `None` | Path to MockForge config file |
| `openapi_spec` | `str` | `None` | Path to OpenAPI specification |

### Instance Methods

| Method | Description |
|--------|-------------|
| `start()` | Start the server |
| `stub_response(method, path, body, status=200, headers=None, latency_ms=None)` | Add a stub |
| `clear_stubs()` | Remove all stubs |
| `stop()` | Stop the server |
| `url()` | Get the server URL |
| `get_port()` | Get the server port |
| `is_running()` | Check if server is running |

### Stub Options

```python
server.stub_response(
    'GET',
    '/api/users',
    {'users': []},
    status=200,
    headers={'X-Custom-Header': 'value'},
    latency_ms=100
)
```

## pytest Integration

### Fixture Example

```python
# conftest.py
import pytest
from mockforge_sdk import MockServer

@pytest.fixture(scope="session")
def mock_server():
    server = MockServer(port=3000)
    server.start()
    yield server
    server.stop()

@pytest.fixture(autouse=True)
def clear_stubs(mock_server):
    yield
    mock_server.clear_stubs()
```

### pytest Plugin

MockForge also provides a pytest plugin for easier integration:

```bash
pip install mockforge-pytest
```

```python
# pytest will auto-discover the plugin
def test_api(mockforge):
    mockforge.stub_response('GET', '/api/test', {'status': 'ok'})
    # ...
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MOCKFORGE_CLI_PATH` | Custom path to MockForge CLI binary |
| `MOCKFORGE_LOG_LEVEL` | Log level (debug, info, warn, error) |

## License

Apache-2.0 OR MIT
