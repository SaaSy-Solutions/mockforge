#!/usr/bin/env python3
import json,sys
x=json.load(open(sys.argv[1])); print('| Tool | Version | Metric | Value | Runs |\n|---|---|---|---:|---:|')
for t in x['tools']:
    for k,v in t['metrics'].items(): print(f"| {t['tool']} | {t['version']} | {k} | {v} | {t['metrics'].get('cold_start_run_count','')} |")
