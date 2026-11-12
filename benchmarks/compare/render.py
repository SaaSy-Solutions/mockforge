#!/usr/bin/env python3
"""Render measured tool and fixture values from a comparison result."""
import json
import sys

result = json.load(open(sys.argv[1]))
print('| Tool | Version | Fixture | Metric | Value |')
print('|---|---|---|---|---|')
for tool in result['tools']:
    for fixture in tool['fixtures']:
        for metric, value in fixture['metrics'].items():
            rendered = json.dumps(value, sort_keys=True) if isinstance(value, (dict, list)) else value
            print(f"| {tool['tool']} | {fixture['version']} | {fixture['fixture']} | {metric} | {rendered} |")
