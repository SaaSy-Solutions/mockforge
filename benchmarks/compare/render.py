#!/usr/bin/env python3
"""Render measured tool and fixture values from a comparison result."""
import json
import sys

result = json.load(open(sys.argv[1]))
print('| Tool | Version | Fixture | Metric | Value |')
print('|---|---|---|---|---|')
for tool in result['tools']:
    for fixture in tool['fixtures']:
        metrics = fixture['metrics']
        fidelity = metrics['fidelity']
        value = ('n/a (no native OpenAPI import)' if fidelity.get('status') == 'n/a'
                 else fidelity.get('fraction', fidelity.get('status', 'n/a')))
        print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | fidelity | {value} |")
        operations = metrics['load']['operations_returning_2xx']
        print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | operations returning 2xx | {json.dumps(operations)} |")
        for metric, value in metrics.items():
            if metric in {'fidelity', 'load'}:
                continue
            rendered = json.dumps(value, sort_keys=True) if isinstance(value, (dict, list)) else value
            print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | {metric} | {rendered} |")
        for item in metrics['load']['per_operation']:
            load = item['load']
            print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | {item['operation']} p50/p95/p99 ms | {load['p50_ms']}/{load['p95_ms']}/{load['p99_ms']} |")
            print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | {item['operation']} requests/s | {load.get('throughput_rps')} |")
            print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | {item['operation']} status/errors | {json.dumps({'status_counts': load.get('status_counts'), 'errors': load.get('errors'), 'non_2xx': load.get('non_2xx')}, sort_keys=True)} |")
