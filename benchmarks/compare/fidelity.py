#!/usr/bin/env python3
"""Probe OpenAPI operations and report auditable, schema-aware results."""
import json
import pathlib
import sys
import urllib.error
import urllib.request

import jsonschema
from requests import build_operation, resolve


def main():
    document = json.loads(pathlib.Path(sys.argv[1]).read_text())
    base = sys.argv[2].rstrip('/')
    results = []
    for path, item in document.get('paths', {}).items():
        for method, op in item.items():
            if method.lower() not in {'get', 'post', 'put', 'patch', 'delete', 'options', 'head'}:
                continue
            name = op.get('operationId', f'{method.upper()} {path}')
            built = build_operation(document, path, method, op, base)
            request = urllib.request.Request(built['url'], data=built['body'], method=built['method'])
            if built['content_type']:
                request.add_header('Content-Type', built['content_type'])
            try:
                with urllib.request.urlopen(request, timeout=3) as response:
                    status, body = response.status, response.read()
            except urllib.error.HTTPError as error:
                status, body = error.code, error.read()
            except Exception as error:
                results.append({'operation': name, 'method': method.upper(), 'path': built['path'],
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
            results.append({'operation': name, 'method': method.upper(), 'path': built['path'],
                            'status': status, 'passed': reason is None,
                            'reason': reason or 'documented status and response schema valid'})
    passed = sum(result['passed'] for result in results)
    print(json.dumps({'passed': passed, 'total': len(results),
                      'fraction': passed / len(results) if results else None,
                      'validator': 'jsonschema', 'operations': results}))


if __name__ == '__main__':
    main()
