#!/usr/bin/env python3
"""Hookify UserPromptSubmit hook — evaluates rules on user prompts."""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from core.rule_engine import evaluate_rules, format_output


def main():
    try:
        stdin_data = sys.stdin.read()
        if not stdin_data.strip():
            sys.exit(0)
        context = json.loads(stdin_data)
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    eval_context = {
        "prompt": context.get("prompt", ""),
        "message": context.get("prompt", ""),
    }

    triggered = evaluate_rules("prompt", eval_context)

    if not triggered:
        sys.exit(0)

    output, has_block = format_output(triggered)
    if output:
        print(output)

    sys.exit(2 if has_block else 0)


if __name__ == "__main__":
    main()
