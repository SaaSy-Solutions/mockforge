#!/usr/bin/env python3
"""Probe OpenAPI operations and report auditable, schema-aware results."""
import copy
import json
import pathlib
import re
import sys
import urllib.error
import urllib.request

import jsonschema


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
    if 'example' in schema:
        return schema['example']
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
        if schema.get('format') == 'date-time':
            return '2026-01-01T00:00:00Z'
        if schema.get('format') == 'date':
            return '2026-01-01'
        if schema.get('format') == 'email':
            return 'benchmark@example.com'
        return 'benchmark'
    return None


def main():
    document = json.loads(pathlib.Path(sys.argv[1]).read_text())
    base = sys.argv[2].rstrip('/')
    results = []
    for path, item in document.get('paths', {}).items():
        for method, op in item.items():
            if method.lower() not in {'get', 'post', 'put', 'patch', 'delete', 'options', 'head'}:
                continue
            name = op.get('operationId', f'{method.upper()} {path}')
            actual_path = re.sub(r'\{([^}]+)\}', '1', path)
            req_body = None
            content_type = None
            request_body = op.get('requestBody', {})
            if request_body:
                content = request_body.get('content', {})
                content_type = next((key for key in content if key == 'application/json'), None) or next(iter(content), None)
                if content_type:
                    req_body = json.dumps(sample(resolve(content[content_type].get('schema', {}), document))).encode()
            query = []
            for parameter in op.get('parameters', []):
                if parameter.get('in') == 'query':
                    query.append(f"{parameter['name']}={sample(resolve(parameter.get('schema', {}), document))}")
            url = base + actual_path + (('?' + '&'.join(query)) if query else '')
            request = urllib.request.Request(url, data=req_body, method=method.upper())
            if content_type:
                request.add_header('Content-Type', content_type)
            try:
                with urllib.request.urlopen(request, timeout=3) as response:
                    status, body = response.status, response.read()
            except urllib.error.HTTPError as error:
                status, body = error.code, error.read()
            except Exception as error:
                results.append({'operation': name, 'method': method.upper(), 'path': actual_path,
                                'passed': False, 'reason': f'request failed: {error}'})
                continue
            responses = op.get('responses', {})
            declared = responses.get(str(status)) or responses.get(f'{status // 100}XX') or responses.get('default')
            reason = None
            if declared is None:
                reason = f'undocumented status {status}'
            elif declared.get('content'):
                response_content = declared['content']
                response_type = next((key for key in response_content if key == 'application/json'), None)
                if response_type:
                    try:
                        schema = resolve(response_content[response_type].get('schema', {}), document)
                        jsonschema.validate(json.loads(body), schema)
                    except Exception as error:
                        reason = f'response schema invalid for status {status}: {error}'
            results.append({'operation': name, 'method': method.upper(), 'path': actual_path,
                            'status': status, 'passed': reason is None,
                            'reason': reason or 'documented status and response schema valid'})
    passed = sum(result['passed'] for result in results)
    print(json.dumps({'passed': passed, 'total': len(results),
                      'fraction': passed / len(results) if results else None,
                      'validator': 'jsonschema', 'operations': results}))


if __name__ == '__main__':
    main()
