#!/usr/bin/env python3
"""Render one compact, auditable comparison table per fixture."""
import json
import sys

result = json.load(open(sys.argv[1]))
fixtures = sorted({fixture['fixture'] for tool in result['tools'] for fixture in tool['fixtures']})
for name in fixtures:
    print(f'### {name}')
    print('| Tool | Version | Cold start median | Peak RSS (+mode) | Per-operation summary | Fidelity | Status |')
    print('|---|---|---:|---:|---|---:|---|')
    for tool in result['tools']:
        fixture = next((item for item in tool['fixtures'] if item['fixture'] == name), None)
        if fixture is None:
            continue
        metrics = fixture['metrics']
        load = metrics['load']
        operations = load.get('per_operation', [])
        p50s = [item['load']['p50_ms'] for item in operations if item['load'].get('p50_ms') is not None]
        median_p50 = sorted(p50s)[len(p50s) // 2] if p50s else None
        rps = sum(item['load'].get('throughput_rps') or 0 for item in operations)
        summary = f"p50 median {median_p50:.3f} ms; {rps:.2f} req/s total" if median_p50 is not None else f"n/a; {rps:.2f} req/s total"
        fidelity = metrics['fidelity']
        fidelity_value = ('n/a' if fidelity.get('status') == 'n/a' else
                          fidelity.get('status') if fidelity.get('status') else
                          f"{fidelity.get('passed', 0)}/{fidelity.get('total', 0)} ({fidelity.get('fraction', 0):.1%})")
        rss = metrics.get('peak_rss_bytes')
        rss_value = f"{rss / (1024 * 1024):.1f} MiB ({metrics.get('peak_rss_mode', 'unknown')})" if rss is not None else 'n/a'
        failed = load.get('failed') or any(item['load'].get('exit_code') not in (None, 0) for item in operations)
        if fidelity.get('status') == 'failed':
            failed = True
        cold = metrics.get('cold_start_seconds_median')
        print(f"| {tool['tool']} | {fixture['version']} | {cold:.3f} s | {rss_value} | {summary} | {fidelity_value} | {'FAILED' if failed else 'OK'} |")
    print()
