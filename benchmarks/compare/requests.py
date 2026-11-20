"""Shared OpenAPI operation request construction for load and fidelity probes."""
import copy
import json
import re
import urllib.parse


def resolve(value, document):
    if isinstance(value, dict):
        if '$ref' in value:
            ref = value['$ref']
            if not ref.startswith('#/'):
                raise ValueError(f'external reference unsupported: {ref}')
            target = document
            for part in ref[2:].split('/'):
                target = target[part.replace('~1', '/').replace('~0', '~')]
            return resolve(copy.deepcopy(target), document)
        return {key: resolve(item, document) for key, item in value.items()}
    if isinstance(value, list):
        return [resolve(item, document) for item in value]
    return value


def sample(schema):
    schema = schema or {}
    if 'example' in schema:
        return schema['example']
    if 'default' in schema:
        return schema['default']
    if schema.get('enum'):
        return schema['enum'][0]
    kind = schema.get('type')
    if kind == 'object' or 'properties' in schema:
        return {key: sample(value) for key, value in schema.get('properties', {}).items()}
    if kind == 'array':
        return [sample(schema.get('items', {}))]
    if kind == 'integer':
        return 1
    if kind == 'number':
        return 1.0
    if kind == 'boolean':
        return True
    if kind == 'string':
        return {'date-time': '2026-01-01T00:00:00Z', 'date': '2026-01-01',
                'email': 'benchmark@example.com'}.get(schema.get('format'), 'benchmark')
    return None


def build_operation(document, path, method, operation, base=''):
    """Return the concrete URL, method, content type, and stable body bytes."""
    actual_path = re.sub(r'\{([^}]+)\}', '1', path)
    query = []
    for parameter in operation.get('parameters', []):
        if parameter.get('in') == 'query':
            query.append((parameter['name'], str(sample(resolve(parameter.get('schema', {}), document)))))
    url = base.rstrip('/') + actual_path
    if query:
        url += '?' + urllib.parse.urlencode(query)
    content_type = None
    body = None
    request_body = operation.get('requestBody', {})
    content = request_body.get('content', {})
    if content:
        content_type = ('application/json' if 'application/json' in content else next(iter(content)))
        body = json.dumps(sample(resolve(content[content_type].get('schema', {}), document)),
                          separators=(',', ':')).encode()
    return {'url': url, 'path': actual_path, 'method': method.upper(),
            'content_type': content_type, 'body': body}
